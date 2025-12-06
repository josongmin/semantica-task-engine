// SQLite JobRepository Implementation

use crate::SqliteJobTransaction;
use async_trait::async_trait;
use semantica_core::domain::{Job, JobId, JobState};
use semantica_core::error::{AppError, Result};
use semantica_core::port::{
    JobRepository, JobRepositoryTransaction, TimeProvider, TransactionalJobRepository,
};
use sqlx::SqlitePool;
use std::sync::Arc;

// Helper to convert sqlx::Error to AppError with structured information
fn map_sqlx_error(err: sqlx::Error) -> AppError {
    match &err {
        sqlx::Error::Database(db_err) => {
            // Extract database-specific error code and message
            if let Some(code) = db_err.code() {
                let code_str = code.as_ref();

                // SQLite error codes: https://www.sqlite.org/rescode.html
                match code_str {
                    "2067" | "1555" => {
                        // UNIQUE constraint failed
                        AppError::Database(format!(
                            "Unique constraint violation: {} ({})",
                            db_err.message(),
                            code_str
                        ))
                    }
                    "787" | "3850" => {
                        // FOREIGN KEY constraint failed
                        AppError::Database(format!(
                            "Foreign key constraint violation: {} ({})",
                            db_err.message(),
                            code_str
                        ))
                    }
                    "5" => {
                        // SQLITE_BUSY - database is locked
                        AppError::Database(format!(
                            "Database locked (SQLITE_BUSY): {}",
                            db_err.message()
                        ))
                    }
                    "13" => {
                        // SQLITE_FULL - database or disk is full
                        AppError::Database(format!("Database full: {}", db_err.message()))
                    }
                    _ => {
                        // Other database errors
                        AppError::Database(format!(
                            "Database error [{}]: {}",
                            code_str,
                            db_err.message()
                        ))
                    }
                }
            } else {
                AppError::Database(format!("Database error: {}", db_err.message()))
            }
        }
        sqlx::Error::RowNotFound => AppError::Database("Row not found".to_string()),
        sqlx::Error::ColumnNotFound(col) => {
            AppError::Database(format!("Column not found: {}", col))
        }
        _ => {
            // Connection, pool, protocol errors
            AppError::Database(format!("{}: {}", err, err))
        }
    }
}

pub struct SqliteJobRepository {
    pool: SqlitePool,
    time_provider: Arc<dyn TimeProvider>,
}

impl SqliteJobRepository {
    pub fn new(pool: SqlitePool, time_provider: Arc<dyn TimeProvider>) -> Self {
        Self {
            pool,
            time_provider,
        }
    }
}

#[async_trait]
impl JobRepository for SqliteJobRepository {
    async fn insert(&self, job: &Job) -> Result<()> {
        let execution_mode_str = job.execution_mode.as_ref().map(|m| m.to_string());
        let env_vars_str = job.env_vars.as_ref().map(|v| v.to_string());

        sqlx::query(
            r#"
            INSERT INTO jobs (
                id, queue, job_type, subject_key, generation,
                priority, state, created_at, started_at, finished_at,
                payload, log_path,
                execution_mode, pid, env_vars,
                attempts, max_attempts, backoff_factor,
                deadline, ttl_ms, trace_id,
                schedule_at, wait_for_idle, require_charging, wait_for_event,
                user_tag, parent_job_id, chain_group_id, result_summary, artifacts
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&job.id)
        .bind(&job.queue)
        .bind(job.job_type.as_str())
        .bind(&job.subject_key)
        .bind(job.generation)
        .bind(job.priority)
        .bind(job.state.to_string())
        .bind(job.created_at)
        .bind(job.started_at)
        .bind(job.finished_at)
        .bind(job.payload.as_value().to_string())
        .bind(&job.log_path)
        // Phase 2 fields
        .bind(&execution_mode_str)
        .bind(job.pid)
        .bind(&env_vars_str)
        .bind(job.attempts)
        .bind(job.max_attempts)
        .bind(job.backoff_factor)
        .bind(job.deadline)
        .bind(job.ttl_ms)
        .bind(&job.trace_id)
        // Phase 3 fields
        .bind(job.schedule_at)
        .bind(if job.wait_for_idle { 1 } else { 0 })
        .bind(if job.require_charging { 1 } else { 0 })
        .bind(&job.wait_for_event)
        // Phase 4 fields
        .bind(&job.user_tag)
        .bind(&job.parent_job_id)
        .bind(&job.chain_group_id)
        .bind(&job.result_summary)
        .bind(&job.artifacts)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn find_by_id(&self, id: &JobId) -> Result<Option<Job>> {
        let row = sqlx::query_as::<_, JobRow>("SELECT * FROM jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(row.map(|r| r.into_job()))
    }

    async fn update(&self, job: &Job) -> Result<()> {
        let execution_mode_str = job.execution_mode.as_ref().map(|m| m.to_string());
        let env_vars_str = job.env_vars.as_ref().map(|v| v.to_string());

        sqlx::query(
            r#"
            UPDATE jobs
            SET state = ?, started_at = ?, finished_at = ?, log_path = ?,
                execution_mode = ?, pid = ?, env_vars = ?,
                attempts = ?, deadline = ?, trace_id = ?,
                schedule_at = ?, wait_for_idle = ?, require_charging = ?, wait_for_event = ?,
                user_tag = ?, parent_job_id = ?, chain_group_id = ?, result_summary = ?, artifacts = ?
            WHERE id = ?
            "#,
        )
        .bind(job.state.to_string())
        .bind(job.started_at)
        .bind(job.finished_at)
        .bind(&job.log_path)
        // Phase 2 fields
        .bind(&execution_mode_str)
        .bind(job.pid)
        .bind(&env_vars_str)
        .bind(job.attempts)
        .bind(job.deadline)
        .bind(&job.trace_id)
        // Phase 3 fields
        .bind(job.schedule_at)
        .bind(job.wait_for_idle)
        .bind(job.require_charging)
        .bind(&job.wait_for_event)
        // Phase 4 fields
        .bind(&job.user_tag)
        .bind(&job.parent_job_id)
        .bind(&job.chain_group_id)
        .bind(&job.result_summary)
        .bind(&job.artifacts)
        .bind(&job.id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update_state(
        &self,
        id: &JobId,
        state: JobState,
        finished_at: Option<i64>,
    ) -> Result<()> {
        // Optimization: Update only state and finished_at (reduces WAL writes)
        // Security: Conditional update to prevent race conditions (e.g., cancel after completion)
        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET state = ?, finished_at = ?
            WHERE id = ?
              AND state NOT IN ('COMPLETED', 'FAILED', 'CANCELLED', 'SUPERSEDED')
            "#,
        )
        .bind(state.to_string())
        .bind(finished_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        // Check if row was actually updated
        if result.rows_affected() == 0 {
            // Job might not exist or already in terminal state
            // Verify existence
            let exists: Option<String> = sqlx::query_scalar("SELECT state FROM jobs WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_sqlx_error)?;

            match exists {
                None => Err(AppError::NotFound(format!("Job {} not found", id))),
                Some(current_state) => Err(AppError::InvalidState(format!(
                    "Cannot update job {} from {} to {}",
                    id, current_state, state
                ))),
            }
        } else {
            Ok(())
        }
    }

    async fn increment_attempts(&self, id: &JobId) -> Result<()> {
        // Optimization: Atomic increment without reading
        sqlx::query(
            r#"
            UPDATE jobs
            SET attempts = attempts + 1
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn pop_next(&self, queue: &str) -> Result<Option<Job>> {
        // Phase 3: Pop-time supersede
        // Strategy: Only pop jobs with latest generation for their subject_key
        // This prevents popping obsolete jobs that were enqueued before a newer version

        let now = self.time_provider.now_millis();
        let state_running = JobState::Running.to_string();
        let state_queued = JobState::Queued.to_string();

        let row = sqlx::query_as::<_, JobRow>(
            r#"
            UPDATE jobs
            SET state = ?, started_at = ?
            WHERE id = (
                SELECT j.id FROM jobs j
                WHERE j.queue = ? AND j.state = ?
                  -- Pop-time supersede: Only pop if this job has the latest generation
                  AND j.generation = (
                      SELECT MAX(generation) 
                      FROM jobs 
                      WHERE subject_key = j.subject_key
                  )
                ORDER BY j.priority DESC, j.created_at ASC, j.id ASC
                LIMIT 1
            )
            RETURNING *
            "#,
        )
        .bind(&state_running)
        .bind(now)
        .bind(queue)
        .bind(&state_queued)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| r.into_job()))
    }

    async fn get_latest_generation(&self, subject_key: &str) -> Result<i64> {
        let gen: Option<i64> =
            sqlx::query_scalar("SELECT latest_generation FROM subjects WHERE subject_key = ?")
                .bind(subject_key)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_sqlx_error)?;

        match gen {
            Some(g) => Ok(g),
            None => {
                // Insert new subject with generation 0
                sqlx::query("INSERT INTO subjects (subject_key, latest_generation) VALUES (?, 0)")
                    .bind(subject_key)
                    .execute(&self.pool)
                    .await
                    .map_err(map_sqlx_error)?;
                Ok(0)
            }
        }
    }

    async fn mark_superseded(&self, subject_key: &str, below_generation: i64) -> Result<u64> {
        let now = self.time_provider.now_millis();
        let state_superseded = JobState::Superseded.to_string();
        let state_queued = JobState::Queued.to_string();

        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET state = ?, finished_at = ?
            WHERE subject_key = ? AND generation < ? AND state = ?
            "#,
        )
        .bind(&state_superseded)
        .bind(now)
        .bind(subject_key)
        .bind(below_generation)
        .bind(&state_queued)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        // Update subjects table
        sqlx::query("UPDATE subjects SET latest_generation = ? WHERE subject_key = ?")
            .bind(below_generation)
            .bind(subject_key)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(result.rows_affected())
    }

    async fn count_by_state(&self, queue: &str, state: JobState) -> Result<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE queue = ? AND state = ?")
                .bind(queue)
                .bind(state.to_string())
                .fetch_one(&self.pool)
                .await
                .map_err(map_sqlx_error)?;

        Ok(count)
    }

    async fn find_by_state(&self, state: JobState) -> Result<Vec<Job>> {
        let rows: Vec<JobRow> = sqlx::query_as(
            r#"
            SELECT * FROM jobs
            WHERE state = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(state.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(|row| row.into_job()).collect())
    }
}

#[async_trait]
impl TransactionalJobRepository for SqliteJobRepository {
    async fn begin_transaction(&self) -> Result<Box<dyn JobRepositoryTransaction>> {
        let tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        Ok(Box::new(SqliteJobTransaction::new(
            tx,
            Arc::clone(&self.time_provider),
        )))
    }
}

/// SQLite row representation (Phase 1 + Phase 2 + Phase 3)
#[derive(Debug, sqlx::FromRow)]
struct JobRow {
    // Phase 1
    id: String,
    queue: String,
    job_type: String,
    subject_key: String,
    generation: i64, // Matches Generation type
    state: String,
    priority: i32,
    payload: String,
    log_path: Option<String>,
    created_at: i64,
    started_at: Option<i64>,
    finished_at: Option<i64>,

    // Phase 2
    execution_mode: Option<String>,
    pid: Option<i32>,
    env_vars: Option<String>,
    attempts: i32,
    max_attempts: i32,
    backoff_factor: f64,
    deadline: Option<i64>,
    ttl_ms: Option<i64>,
    trace_id: Option<String>,

    // Phase 3
    schedule_at: Option<i64>,
    wait_for_idle: i32,    // SQLite boolean as integer
    require_charging: i32, // SQLite boolean as integer
    wait_for_event: Option<String>,

    // Phase 4
    user_tag: Option<String>,
    parent_job_id: Option<String>,
    chain_group_id: Option<String>,
    result_summary: Option<String>,
    artifacts: Option<String>,
}

impl JobRow {
    fn into_job(self) -> Job {
        use semantica_core::domain::{ExecutionMode, JobPayload, JobType};

        let state = match self.state.as_str() {
            "QUEUED" => JobState::Queued,
            "RUNNING" => JobState::Running,
            "DONE" => JobState::Done,
            "FAILED" => JobState::Failed,
            "SUPERSEDED" => JobState::Superseded,
            "CANCELLED" => JobState::Cancelled,
            "REQUEUED" => JobState::Requeued,
            _ => JobState::Failed, // Default fallback
        };

        let execution_mode = self.execution_mode.as_deref().and_then(|mode| match mode {
            "IN_PROCESS" => Some(ExecutionMode::InProcess),
            "SUBPROCESS" => Some(ExecutionMode::Subprocess),
            _ => None,
        });

        let payload: serde_json::Value =
            serde_json::from_str(&self.payload).unwrap_or(serde_json::json!({}));

        let env_vars = self.env_vars.and_then(|s| serde_json::from_str(&s).ok());

        Job {
            // Phase 1 fields
            id: self.id,
            queue: self.queue,
            job_type: JobType::new(self.job_type),
            subject_key: self.subject_key,
            generation: self.generation,
            priority: self.priority,
            state,
            created_at: self.created_at,
            started_at: self.started_at,
            finished_at: self.finished_at,
            payload: JobPayload::new(payload),
            log_path: self.log_path,

            // Phase 2 fields
            execution_mode,
            pid: self.pid,
            env_vars,
            attempts: self.attempts,
            max_attempts: self.max_attempts,
            backoff_factor: self.backoff_factor,
            deadline: self.deadline,
            ttl_ms: self.ttl_ms,
            trace_id: self.trace_id,

            // Phase 3 fields
            schedule_at: self.schedule_at,
            wait_for_idle: self.wait_for_idle != 0,
            require_charging: self.require_charging != 0,
            wait_for_event: self.wait_for_event,

            // Phase 4 fields
            user_tag: self.user_tag,
            parent_job_id: self.parent_job_id,
            chain_group_id: self.chain_group_id,
            result_summary: self.result_summary,
            artifacts: self.artifacts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_pool, run_migrations};
    use semantica_core::domain::{JobPayload, JobType};
    use semantica_core::port::time_provider::SystemTimeProvider;

    async fn setup_test_db() -> (SqlitePool, Arc<dyn TimeProvider>) {
        let pool = create_pool("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        let time_provider = Arc::new(SystemTimeProvider);
        (pool, time_provider)
    }

    #[tokio::test]
    async fn test_insert_and_find() {
        let (pool, time_provider) = setup_test_db().await;
        let repo = SqliteJobRepository::new(pool, time_provider);

        let job = Job::new_test(
            "test_queue",
            JobType::new("TEST"),
            "test::subject",
            1,
            JobPayload::new(serde_json::json!({"key": "value"})),
        );

        repo.insert(&job).await.unwrap();

        let found = repo.find_by_id(&job.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, job.id);
    }

    #[tokio::test]
    async fn test_pop_next() {
        let (pool, time_provider) = setup_test_db().await;
        let repo = SqliteJobRepository::new(pool, time_provider);

        // Insert jobs with different priorities
        let mut job1 = Job::new_test(
            "test_queue",
            JobType::new("TEST"),
            "subject1",
            1,
            JobPayload::new(serde_json::json!({})),
        );
        job1.priority = 0;

        let mut job2 = Job::new_test(
            "test_queue",
            JobType::new("TEST"),
            "subject2",
            1,
            JobPayload::new(serde_json::json!({})),
        );
        job2.priority = 10;

        repo.insert(&job1).await.unwrap();
        repo.insert(&job2).await.unwrap();

        // Should pop job2 first (higher priority)
        let popped = repo.pop_next("test_queue").await.unwrap();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().id, job2.id);
    }

    #[tokio::test]
    async fn test_supersede() {
        let (pool, time_provider) = setup_test_db().await;
        let repo = SqliteJobRepository::new(pool, time_provider);

        // Insert 3 jobs with same subject_key, different generations
        for gen in 1..=3 {
            let job = Job::new_test(
                "test_queue",
                JobType::new("TEST"),
                "same::subject",
                gen,
                JobPayload::new(serde_json::json!({})),
            );
            repo.insert(&job).await.unwrap();
        }

        // Supersede generations < 3
        let count = repo.mark_superseded("same::subject", 3).await.unwrap();
        assert_eq!(count, 2); // 2 jobs superseded

        // Check that only generation 3 is QUEUED
        let queued = repo
            .count_by_state("test_queue", JobState::Queued)
            .await
            .unwrap();
        assert_eq!(queued, 1);

        let superseded = repo
            .count_by_state("test_queue", JobState::Superseded)
            .await
            .unwrap();
        assert_eq!(superseded, 2);
    }
}
