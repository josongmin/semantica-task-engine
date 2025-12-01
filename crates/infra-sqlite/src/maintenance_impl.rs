// SQLite Maintenance Implementation (Phase 4)
use async_trait::async_trait;
use semantica_core::domain::JobState;
use semantica_core::error::{AppError, Result};
use semantica_core::port::{Maintenance, MaintenanceStats, TimeProvider};
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{info, warn};

/// SQLite maintenance implementation
pub struct SqliteMaintenance {
    pool: SqlitePool,
    time_provider: Arc<dyn TimeProvider>,
}

impl SqliteMaintenance {
    pub fn new(pool: SqlitePool, time_provider: Arc<dyn TimeProvider>) -> Self {
        Self {
            pool,
            time_provider,
        }
    }

    /// Get DB file size in MB
    async fn get_db_size(&self) -> Result<f64> {
        // Query database page count and page size
        let page_count: i64 = sqlx::query_scalar("PRAGMA page_count")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get page count: {}", e)))?;

        let page_size: i64 = sqlx::query_scalar("PRAGMA page_size")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get page size: {}", e)))?;

        let size_bytes = page_count * page_size;
        let size_mb = size_bytes as f64 / (1024.0 * 1024.0);

        Ok(size_mb)
    }
}

#[async_trait]
impl Maintenance for SqliteMaintenance {
    async fn vacuum(&self) -> Result<f64> {
        info!("Running VACUUM to optimize database...");

        // Get size before VACUUM
        let size_before = self.get_db_size().await?;

        // Run VACUUM (reclaims space and defragments)
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("VACUUM failed: {}", e)))?;

        // Get size after VACUUM
        let size_after = self.get_db_size().await?;
        let reclaimed = (size_before - size_after).max(0.0);

        info!(
            size_before_mb = size_before,
            size_after_mb = size_after,
            reclaimed_mb = reclaimed,
            "VACUUM completed"
        );

        Ok(reclaimed)
    }

    async fn gc_finished_jobs(&self, retention_days: i64) -> Result<i64> {
        let now = self.time_provider.now_millis();
        let retention_ms = retention_days * 24 * 60 * 60 * 1000;
        let cutoff_time = now - retention_ms;

        info!(
            retention_days = retention_days,
            cutoff_time = cutoff_time,
            "Running finished job GC"
        );

        // Delete jobs that are DONE/FAILED/SUPERSEDED and finished before cutoff
        let result = sqlx::query(
            r#"
            DELETE FROM jobs
            WHERE state IN (?, ?, ?)
            AND finished_at IS NOT NULL
            AND finished_at < ?
            "#,
        )
        .bind(JobState::Done.to_string())
        .bind(JobState::Failed.to_string())
        .bind(JobState::Superseded.to_string())
        .bind(cutoff_time)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Job GC failed: {}", e)))?;

        let deleted = result.rows_affected() as i64;

        info!(deleted_jobs = deleted, "Finished job GC completed");

        Ok(deleted)
    }

    async fn gc_artifacts(&self, retention_days: i64) -> Result<usize> {
        let now = self.time_provider.now_millis();
        let retention_ms = retention_days * 24 * 60 * 60 * 1000;
        let cutoff_time = now - retention_ms;

        info!(
            retention_days = retention_days,
            cutoff_time = cutoff_time,
            "Running artifact GC"
        );

        // Find log files for old finished jobs
        let log_paths: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT log_path FROM jobs
            WHERE state IN (?, ?, ?)
            AND finished_at IS NOT NULL
            AND finished_at < ?
            AND log_path IS NOT NULL
            "#,
        )
        .bind(JobState::Done.to_string())
        .bind(JobState::Failed.to_string())
        .bind(JobState::Superseded.to_string())
        .bind(cutoff_time)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to query log paths: {}", e)))?;

        let mut deleted_count = 0;

        // Delete log files
        for log_path in log_paths {
            match tokio::fs::remove_file(&log_path).await {
                Ok(_) => {
                    deleted_count += 1;
                    info!(path = %log_path, "Deleted log file");
                }
                Err(e) => {
                    // Not critical - log file might already be deleted
                    warn!(path = %log_path, error = %e, "Failed to delete log file");
                }
            }
        }

        info!(deleted_artifacts = deleted_count, "Artifact GC completed");

        Ok(deleted_count)
    }

    async fn get_stats(&self) -> Result<MaintenanceStats> {
        // Get DB size
        let db_size_mb = self.get_db_size().await?;

        // Get job counts
        let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to count jobs: {}", e)))?;

        let finished_job_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM jobs
            WHERE state IN (?, ?, ?)
            "#,
        )
        .bind(JobState::Done.to_string())
        .bind(JobState::Failed.to_string())
        .bind(JobState::Superseded.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count finished jobs: {}", e)))?;

        // Get log file count and size
        let log_paths: Vec<String> =
            sqlx::query_scalar("SELECT log_path FROM jobs WHERE log_path IS NOT NULL")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to query log paths: {}", e)))?;

        let mut log_files_size_mb = 0.0;
        let artifact_count = log_paths.len();

        for log_path in log_paths {
            if let Ok(metadata) = tokio::fs::metadata(&log_path).await {
                log_files_size_mb += metadata.len() as f64 / (1024.0 * 1024.0);
            }
        }

        // Calculate DB size in bytes
        let db_size_bytes = (db_size_mb * 1024.0 * 1024.0) as i64;

        // Calculate fragmentation (simplified)
        let fragmentation_percent = if job_count > 0 {
            (finished_job_count as f64 / job_count as f64) * 100.0
        } else {
            0.0
        };

        Ok(MaintenanceStats {
            db_size_mb,
            db_size_bytes,
            job_count,
            finished_job_count,
            artifact_count,
            log_files_size_mb,
            fragmentation_percent,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_pool, run_migrations, SqliteJobRepository};
    use semantica_core::domain::{Job, JobPayload, JobType};
    use semantica_core::port::time_provider::SystemTimeProvider;
    use semantica_core::port::JobRepository; // Need trait in scope

    #[tokio::test]
    async fn test_maintenance_stats() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        let time_provider = Arc::new(SystemTimeProvider);
        let maintenance = SqliteMaintenance::new(pool, time_provider);

        let stats = maintenance.get_stats().await.unwrap();

        assert!(stats.db_size_mb > 0.0);
        assert_eq!(stats.job_count, 0);
        assert_eq!(stats.finished_job_count, 0);
    }

    #[tokio::test]
    async fn test_vacuum() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        let time_provider = Arc::new(SystemTimeProvider);
        let maintenance = SqliteMaintenance::new(pool, time_provider);

        // VACUUM should not error (even if no space is reclaimed in memory DB)
        let reclaimed = maintenance.vacuum().await.unwrap();
        assert!(reclaimed >= 0.0);
    }

    #[tokio::test]
    async fn test_gc_finished_jobs() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        let time_provider = Arc::new(SystemTimeProvider);
        let job_repo = SqliteJobRepository::new(pool.clone(), time_provider.clone());
        let maintenance = SqliteMaintenance::new(pool, time_provider.clone());

        // Create a finished job (10 days ago)
        let now_ms = time_provider.now_millis();
        let ten_days_ago = now_ms - (10 * 24 * 60 * 60 * 1000);

        let mut job = Job::new_test(
            "test",
            JobType::new("TEST"),
            "subject",
            1,
            JobPayload::new(serde_json::json!({})),
        );
        job.state = JobState::Done;
        job.finished_at = Some(ten_days_ago);

        job_repo.insert(&job).await.unwrap();

        // GC with 7 day retention should delete it
        let deleted = maintenance.gc_finished_jobs(7).await.unwrap();
        assert_eq!(deleted, 1);

        // Verify job is deleted
        let found = job_repo.find_by_id(&job.id).await.unwrap();
        assert!(found.is_none());
    }
}
