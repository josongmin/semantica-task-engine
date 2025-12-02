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
    // Input validation (Security: prevent DoS and resource exhaustion)
    validate_request(&req)?;

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

/// Validate enqueue request (Security: ADR-040)
///
/// Prevents:
/// - DoS via large payloads
/// - Invalid queue names
/// - Priority abuse
/// - Subject key overflow
fn validate_request(req: &EnqueueRequest) -> Result<()> {
    use crate::error::AppError;

    // Queue name validation
    if req.queue.is_empty() {
        return Err(AppError::Validation("Queue name cannot be empty".to_string()));
    }
    if req.queue.len() > 64 {
        return Err(AppError::Validation(format!(
            "Queue name too long (max 64 chars, got {})",
            req.queue.len()
        )));
    }
    if !req.queue.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::Validation(
            "Queue name must be alphanumeric with _ or -".to_string(),
        ));
    }

    // Job type validation
    if req.job_type.is_empty() {
        return Err(AppError::Validation("Job type cannot be empty".to_string()));
    }
    if req.job_type.len() > 128 {
        return Err(AppError::Validation(format!(
            "Job type too long (max 128 chars, got {})",
            req.job_type.len()
        )));
    }

    // Subject key validation
    if req.subject_key.is_empty() {
        return Err(AppError::Validation(
            "Subject key cannot be empty".to_string(),
        ));
    }
    if req.subject_key.len() > 512 {
        return Err(AppError::Validation(format!(
            "Subject key too long (max 512 chars, got {})",
            req.subject_key.len()
        )));
    }

    // Payload size validation (lightweight check)
    // Note: Heavy validation done at RPC layer (max_request_body_size)
    // This is a secondary check for depth/complexity
    validate_payload_complexity(&req.payload)?;

    // Priority validation
    const MIN_PRIORITY: i32 = -100;
    const MAX_PRIORITY: i32 = 100;
    if req.priority < MIN_PRIORITY || req.priority > MAX_PRIORITY {
        return Err(AppError::Validation(format!(
            "Priority out of range (must be between {} and {}, got {})",
            MIN_PRIORITY, MAX_PRIORITY, req.priority
        )));
    }

    Ok(())
}

/// Validate payload complexity (depth and structure)
///
/// Prevents deeply nested JSON that could cause stack overflow
/// during processing or serialization.
fn validate_payload_complexity(value: &serde_json::Value) -> Result<()> {
    use crate::error::AppError;
    
    const MAX_DEPTH: usize = 32;
    
    fn check_depth(value: &serde_json::Value, current_depth: usize) -> Result<()> {
        if current_depth > MAX_DEPTH {
            return Err(AppError::Validation(format!(
                "Payload too deeply nested (max depth: {}, exceeded)",
                MAX_DEPTH
            )));
        }
        
        match value {
            serde_json::Value::Array(arr) => {
                for item in arr {
                    check_depth(item, current_depth + 1)?;
                }
            }
            serde_json::Value::Object(obj) => {
                for (_, val) in obj {
                    check_depth(val, current_depth + 1)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    check_depth(value, 0)
}
