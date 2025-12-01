// Job Repository Port (Interface)

use crate::domain::{Job, JobId, JobState};
use crate::error::Result;
use async_trait::async_trait;

/// Repository interface for Job persistence
#[async_trait]
pub trait JobRepository: Send + Sync {
    /// Insert a new job
    async fn insert(&self, job: &Job) -> Result<()>;

    /// Find job by ID
    async fn find_by_id(&self, id: &JobId) -> Result<Option<Job>>;

    /// Update job
    async fn update(&self, job: &Job) -> Result<()>;

    /// Pop next job from queue (FIFO with priority)
    async fn pop_next(&self, queue: &str) -> Result<Option<Job>>;

    /// Get latest generation for subject_key
    async fn get_latest_generation(&self, subject_key: &str) -> Result<i64>;

    /// Mark jobs as superseded
    async fn mark_superseded(&self, subject_key: &str, below_generation: i64) -> Result<u64>;

    /// Count jobs by state
    async fn count_by_state(&self, queue: &str, state: JobState) -> Result<i64>;

    /// Find all jobs by state (Phase 2 - for recovery)
    async fn find_by_state(&self, state: JobState) -> Result<Vec<Job>>;
}
