// SQLite Transaction Implementation

use async_trait::async_trait;
use semantica_core::domain::{Job, JobState};
use semantica_core::error::Result;
use semantica_core::port::{JobRepositoryTransaction, TimeProvider, Transaction};
use sqlx::{Sqlite, Transaction as SqlxTransaction};
use std::sync::Arc;

pub struct SqliteJobTransaction<'a> {
    tx: SqlxTransaction<'a, Sqlite>,
    time_provider: Arc<dyn TimeProvider>,
}

impl<'a> SqliteJobTransaction<'a> {
    pub fn new(tx: SqlxTransaction<'a, Sqlite>, time_provider: Arc<dyn TimeProvider>) -> Self {
        Self { tx, time_provider }
    }
}

#[async_trait]
impl Transaction for SqliteJobTransaction<'_> {
    async fn commit(mut self: Box<Self>) -> Result<()> {
        self.tx
            .commit()
            .await
            .map_err(|e| semantica_core::error::AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> Result<()> {
        self.tx
            .rollback()
            .await
            .map_err(|e| semantica_core::error::AppError::Database(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl JobRepositoryTransaction for SqliteJobTransaction<'_> {
    async fn get_latest_generation(&mut self, subject_key: &str) -> Result<i64> {
        let gen: Option<i64> =
            sqlx::query_scalar("SELECT latest_generation FROM subjects WHERE subject_key = ?")
                .bind(subject_key)
                .fetch_optional(&mut *self.tx)
                .await
                .map_err(|e| semantica_core::error::AppError::Database(e.to_string()))?;

        match gen {
            Some(g) => Ok(g),
            None => {
                // Insert new subject with generation 0
                sqlx::query("INSERT INTO subjects (subject_key, latest_generation) VALUES (?, 0)")
                    .bind(subject_key)
                    .execute(&mut *self.tx)
                    .await
                    .map_err(|e| semantica_core::error::AppError::Database(e.to_string()))?;
                Ok(0)
            }
        }
    }

    async fn insert(&mut self, job: &Job) -> Result<()> {
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
                deadline, ttl_ms, trace_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .execute(&mut *self.tx)
        .await
        .map_err(|e| semantica_core::error::AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn mark_superseded(&mut self, subject_key: &str, below_generation: i64) -> Result<u64> {
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
        .execute(&mut *self.tx)
        .await
        .map_err(|e| semantica_core::error::AppError::Database(e.to_string()))?;

        // Update subjects table
        sqlx::query("UPDATE subjects SET latest_generation = ? WHERE subject_key = ?")
            .bind(below_generation)
            .bind(subject_key)
            .execute(&mut *self.tx)
            .await
            .map_err(|e| semantica_core::error::AppError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }
}
