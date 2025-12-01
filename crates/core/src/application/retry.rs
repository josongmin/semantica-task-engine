// Retry logic (Phase 2, ADR-002)
use crate::domain::{Job, JobState};
use crate::port::TimeProvider;
use std::sync::Arc;
use tracing::{info, warn};

/// Retry decision result
#[derive(Debug, PartialEq, Eq)]
pub enum RetryDecision {
    /// Retry the job (with backoff delay in ms)
    Retry(i64),
    /// Do not retry, job has failed permanently
    Failed,
}

/// Retry policy based on ADR-002
///
/// Determines if a job should be retried based on:
/// - Current attempt count
/// - Maximum attempts allowed
/// - Backoff factor for exponential delay
pub struct RetryPolicy {
    time_provider: Arc<dyn TimeProvider>,
    base_delay_ms: i64,
}

impl RetryPolicy {
    /// Create a new retry policy
    ///
    /// # Arguments
    /// * `time_provider` - Time provider for current time
    /// * `base_delay_ms` - Base delay in milliseconds (default: 1000)
    ///
    /// # Example
    /// ```text
    /// let policy = RetryPolicy::new(Arc::new(SystemTimeProvider), 1000);
    /// ```
    pub fn new(time_provider: Arc<dyn TimeProvider>, base_delay_ms: i64) -> Self {
        Self {
            time_provider,
            base_delay_ms,
        }
    }

    /// Determine if a job should be retried
    ///
    /// Returns:
    /// - `RetryDecision::Retry(delay_ms)` if job should be retried with calculated backoff
    /// - `RetryDecision::Failed` if max attempts reached
    ///
    /// Backoff formula (ADR-002):
    /// delay = base_delay * (backoff_factor ^ attempt)
    ///
    /// # Example
    /// ```text
    /// let decision = policy.should_retry(job);
    /// match decision {
    ///     RetryDecision::Retry(delay_ms) => {
    ///         println!("Retry after {} ms", delay_ms);
    ///     }
    ///     RetryDecision::Failed => {
    ///         println!("Max retries exceeded");
    ///     }
    /// }
    /// ```
    pub fn should_retry(&self, job: &Job) -> RetryDecision {
        // Check if max attempts reached
        if job.attempts >= job.max_attempts {
            warn!(
                job_id = %job.id,
                attempts = %job.attempts,
                max_attempts = %job.max_attempts,
                "Max retry attempts reached"
            );
            return RetryDecision::Failed;
        }

        // Calculate exponential backoff with jitter (ADR-002)
        // delay = base_delay * (backoff_factor ^ attempt) * (1.0 ± 0.1)
        let base_delay_ms = self.base_delay_ms as f64 * job.backoff_factor.powi(job.attempts);

        // Apply ±10% jitter to prevent "Thundering Herd" problem
        // Use job.id as seed for deterministic jitter per job
        let jitter_seed = job.id.chars().map(|c| c as u32).sum::<u32>();
        let jitter_factor = 0.9 + ((jitter_seed % 21) as f64 / 100.0); // 0.9 to 1.1

        let delay_ms = (base_delay_ms * jitter_factor) as i64;

        info!(
            job_id = %job.id,
            attempt = %job.attempts,
            max_attempts = %job.max_attempts,
            delay_ms = %delay_ms,
            "Scheduling retry"
        );

        RetryDecision::Retry(delay_ms)
    }

    /// Prepare a job for retry
    ///
    /// Updates job state and increments attempt counter
    ///
    /// # Arguments
    /// * `job` - Job to prepare for retry
    pub fn prepare_for_retry(&self, job: &mut Job) {
        job.attempts += 1;
        job.state = JobState::Queued;
        job.started_at = None;
        job.pid = None;

        info!(
            job_id = %job.id,
            attempt = %job.attempts,
            "Job prepared for retry"
        );
    }

    /// Check if a job has exceeded its deadline
    ///
    /// Returns true if deadline is set and has passed
    pub fn is_deadline_exceeded(&self, job: &Job) -> bool {
        if let Some(deadline) = job.deadline {
            let now = self.time_provider.now_millis();
            if now > deadline {
                warn!(
                    job_id = %job.id,
                    deadline = %deadline,
                    now = %now,
                    "Job deadline exceeded"
                );
                return true;
            }
        }
        false
    }

    /// Check if a job has exceeded its TTL in queue
    ///
    /// Returns true if ttl_ms is set and job has been queued too long
    pub fn is_ttl_exceeded(&self, job: &Job) -> bool {
        if let Some(ttl_ms) = job.ttl_ms {
            let now = self.time_provider.now_millis();
            let age_ms = now - job.created_at;

            if age_ms > ttl_ms {
                warn!(
                    job_id = %job.id,
                    ttl_ms = %ttl_ms,
                    age_ms = %age_ms,
                    "Job TTL exceeded"
                );
                return true;
            }
        }
        false
    }
}
