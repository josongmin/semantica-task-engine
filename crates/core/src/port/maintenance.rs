// DB Maintenance port (Phase 4 - ADR-050)
use crate::error::Result;
use async_trait::async_trait;

/// Database maintenance statistics
#[derive(Debug, Clone)]
pub struct MaintenanceStats {
    pub db_size_mb: f64,
    pub db_size_bytes: i64,
    pub job_count: i64,
    pub finished_job_count: i64,
    pub artifact_count: usize,
    pub log_files_size_mb: f64,
    pub fragmentation_percent: f64,
}

/// Maintenance configuration
#[derive(Debug, Clone)]
pub struct MaintenanceConfig {
    /// Retention period for finished jobs (days)
    pub finished_job_retention_days: i64,

    /// Maximum DB size before forcing VACUUM (MB)
    pub max_db_size_mb: f64,

    /// Artifact retention period (days)
    pub artifact_retention_days: i64,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            finished_job_retention_days: 7, // Keep finished jobs for 7 days
            max_db_size_mb: 1000.0,         // 1GB max
            artifact_retention_days: 3,     // Keep artifacts for 3 days
        }
    }
}

/// Database maintenance operations
#[async_trait]
pub trait Maintenance: Send + Sync {
    /// Run VACUUM to reclaim space and optimize DB
    ///
    /// # Returns
    /// Space reclaimed in MB
    async fn vacuum(&self) -> Result<f64>;

    /// Delete finished jobs older than retention period
    ///
    /// # Arguments
    /// * `retention_days` - Keep jobs finished within this many days
    ///
    /// # Returns
    /// Number of jobs deleted
    async fn gc_finished_jobs(&self, retention_days: i64) -> Result<i64>;

    /// Delete artifact files for deleted/old jobs
    ///
    /// # Arguments
    /// * `retention_days` - Keep artifacts for jobs finished within this many days
    ///
    /// # Returns
    /// Number of artifacts deleted
    async fn gc_artifacts(&self, retention_days: i64) -> Result<usize>;

    /// Get maintenance statistics
    async fn get_stats(&self) -> Result<MaintenanceStats>;

    /// Run full maintenance (VACUUM + GC)
    ///
    /// Runs all maintenance operations based on config
    async fn run_full_maintenance(&self, config: &MaintenanceConfig) -> Result<MaintenanceStats> {
        // 1. Get pre-maintenance stats
        let stats_before = self.get_stats().await?;

        // 2. GC finished jobs
        let deleted_jobs = self
            .gc_finished_jobs(config.finished_job_retention_days)
            .await?;

        // 3. GC artifacts
        let deleted_artifacts = self.gc_artifacts(config.artifact_retention_days).await?;

        // 4. VACUUM if DB is large
        let reclaimed_mb = if stats_before.db_size_mb > config.max_db_size_mb {
            self.vacuum().await?
        } else {
            0.0
        };

        // 5. Get post-maintenance stats
        let stats_after = self.get_stats().await?;

        // Add info about what was done
        tracing::info!(
            deleted_jobs = deleted_jobs,
            deleted_artifacts = deleted_artifacts,
            reclaimed_mb = reclaimed_mb,
            db_size_mb = stats_after.db_size_mb,
            "Maintenance completed"
        );

        Ok(stats_after)
    }
}
