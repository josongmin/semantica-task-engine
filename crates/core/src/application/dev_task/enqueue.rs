// Enqueue Use Case

use crate::domain::{Job, JobPayload, JobType};
use crate::error::Result;
use crate::port::{IdProvider, TimeProvider, TransactionalJobRepository};
use serde::{Deserialize, Serialize};

/// Enqueue request (Phase 1: minimal fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnqueueRequest {
    pub job_type: String,
    pub queue: String,
    pub subject_key: String,
    pub payload: serde_json::Value,

    #[serde(default)]
    pub priority: i32,
}

/// Execute enqueue use case (with transaction for atomicity)
///
/// # Arguments
///
/// * `job_repo` - Transactional job repository
/// * `id_provider` - ID generator (injected for determinism)
/// * `time_provider` - Time provider (injected for determinism)
/// * `req` - Enqueue request
pub async fn execute(
    job_repo: &dyn TransactionalJobRepository,
    id_provider: &dyn IdProvider,
    time_provider: &dyn TimeProvider,
    req: EnqueueRequest,
) -> Result<String> {
    // Start transaction to prevent generation conflicts
    let mut tx = job_repo.begin_transaction().await?;

    // Get latest generation for this subject (within transaction)
    let latest_gen = tx.get_latest_generation(&req.subject_key).await?;
    let new_gen = latest_gen + 1;

    // Create new job (with injected ID and timestamp for determinism)
    let job_id = id_provider.generate_id();
    let created_at = time_provider.now_millis();

    let mut job = Job::new(
        job_id.clone(),
        created_at,
        req.queue,
        JobType::new(req.job_type),
        req.subject_key.clone(),
        new_gen,
        JobPayload::new(req.payload),
    );

    // Set priority from request
    job.priority = req.priority;

    // Insert job (within transaction)
    tx.insert(&job).await?;

    // Mark older generations as superseded (within transaction)
    tx.mark_superseded(&req.subject_key, new_gen).await?;

    // Commit transaction
    tx.commit().await?;

    Ok(job_id)
}
