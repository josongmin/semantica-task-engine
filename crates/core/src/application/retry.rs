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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{JobPayload, JobType};

    struct MockTimeProvider {
        now_ms: i64,
    }

    impl TimeProvider for MockTimeProvider {
        fn now_millis(&self) -> i64 {
            self.now_ms
        }
    }

    fn create_test_job(id: &str, attempts: i32, max_attempts: i32, backoff_factor: f64) -> Job {
        let mut job = Job::new(
            id.to_string(),
            1000, // created_at
            "test".to_string(),
            JobType::new("TEST".to_string()),
            "test.rs".to_string(),
            1, // generation
            JobPayload::new(serde_json::json!({})),
        );
        job.attempts = attempts;
        job.max_attempts = max_attempts;
        job.backoff_factor = backoff_factor;
        job
    }

    #[test]
    fn test_max_attempts_returns_failed() {
        let policy = RetryPolicy::new(Arc::new(MockTimeProvider { now_ms: 1000 }), 1000);

        // attempts == max_attempts → Failed
        let job = create_test_job("job-001", 3, 3, 2.0);
        assert_eq!(policy.should_retry(&job), RetryDecision::Failed);

        // attempts > max_attempts → Failed
        let job2 = create_test_job("job-002", 5, 3, 2.0);
        assert_eq!(policy.should_retry(&job2), RetryDecision::Failed);
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let policy = RetryPolicy::new(Arc::new(MockTimeProvider { now_ms: 1000 }), 1000);

        // Same job ID for consistent jitter
        let job0 = create_test_job("job-test", 0, 5, 2.0);
        let job1 = create_test_job("job-test", 1, 5, 2.0);
        let job2 = create_test_job("job-test", 2, 5, 2.0);

        let delay0 = match policy.should_retry(&job0) {
            RetryDecision::Retry(d) => d,
            _ => panic!("Expected Retry"),
        };
        let delay1 = match policy.should_retry(&job1) {
            RetryDecision::Retry(d) => d,
            _ => panic!("Expected Retry"),
        };
        let delay2 = match policy.should_retry(&job2) {
            RetryDecision::Retry(d) => d,
            _ => panic!("Expected Retry"),
        };

        // Exponential: delay1 ≈ 2 * delay0, delay2 ≈ 4 * delay0 (with jitter)
        // Allow ±20% tolerance for jitter
        let ratio1 = delay1 as f64 / delay0 as f64;
        let ratio2 = delay2 as f64 / delay0 as f64;

        assert!(
            (1.6..=2.4).contains(&ratio1),
            "Expected delay1/delay0 ≈ 2, got {}",
            ratio1
        );
        assert!(
            (3.2..=4.8).contains(&ratio2),
            "Expected delay2/delay0 ≈ 4, got {}",
            ratio2
        );
    }

    #[test]
    fn test_jitter_in_valid_range() {
        let policy = RetryPolicy::new(Arc::new(MockTimeProvider { now_ms: 1000 }), 1000);

        // Test multiple job IDs to verify jitter variance
        for i in 0..20 {
            let job = create_test_job(&format!("job-{:03}", i), 0, 5, 2.0);
            let delay = match policy.should_retry(&job) {
                RetryDecision::Retry(d) => d,
                _ => panic!("Expected Retry"),
            };

            // base_delay = 1000 * 2^0 = 1000
            // With jitter: 900 to 1100
            assert!(
                (900..=1100).contains(&delay),
                "Jitter out of range for job-{}: got {}",
                i,
                delay
            );
        }
    }

    #[test]
    fn test_deadline_exceeded() {
        let policy = RetryPolicy::new(Arc::new(MockTimeProvider { now_ms: 5000 }), 1000);

        let mut job = create_test_job("job-deadline", 0, 5, 2.0);

        // No deadline
        assert!(!policy.is_deadline_exceeded(&job));

        // Deadline in future
        job.deadline = Some(6000);
        assert!(!policy.is_deadline_exceeded(&job));

        // Deadline passed
        job.deadline = Some(4000);
        assert!(policy.is_deadline_exceeded(&job));
    }

    #[test]
    fn test_ttl_exceeded() {
        let policy = RetryPolicy::new(Arc::new(MockTimeProvider { now_ms: 5000 }), 1000);

        let mut job = create_test_job("job-ttl", 0, 5, 2.0);
        job.created_at = 1000; // Age = 5000 - 1000 = 4000ms

        // No TTL
        assert!(!policy.is_ttl_exceeded(&job));

        // TTL not exceeded
        job.ttl_ms = Some(5000);
        assert!(!policy.is_ttl_exceeded(&job));

        // TTL exceeded
        job.ttl_ms = Some(3000);
        assert!(policy.is_ttl_exceeded(&job));
    }

    #[test]
    fn test_prepare_for_retry() {
        let policy = RetryPolicy::new(Arc::new(MockTimeProvider { now_ms: 1000 }), 1000);

        let mut job = create_test_job("job-retry", 1, 5, 2.0);
        job.state = JobState::Failed;
        job.started_at = Some(500);
        job.pid = Some(12345);

        policy.prepare_for_retry(&mut job);

        assert_eq!(job.attempts, 2);
        assert_eq!(job.state, JobState::Queued);
        assert!(job.started_at.is_none());
        assert!(job.pid.is_none());
    }
}
