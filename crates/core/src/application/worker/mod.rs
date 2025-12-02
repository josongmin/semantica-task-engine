// Worker - Job execution loop

pub mod constants;
mod panic_guard;
mod shutdown; // Public for use in other modules

use constants::*;
pub use panic_guard::{execute_guarded, execute_guarded_async, PanicGuardResult};
pub use shutdown::{shutdown_channel, ShutdownSender, ShutdownToken};

// Note: This helper is replaced by RetryPolicy in Phase 2
// Removed as dead code

use crate::application::retry::RetryPolicy;
use crate::domain::Job;
use crate::error::Result;
use crate::port::{JobRepository, SystemProbe, TaskExecutor};
use std::sync::Arc;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Worker processes jobs from a queue (Phase 1 + Phase 2 + Phase 3)
pub struct Worker {
    queue: String,
    job_repo: Arc<dyn JobRepository>,
    task_executor: Arc<dyn TaskExecutor>,
    system_probe: Arc<dyn SystemProbe>,
    retry_policy: Arc<RetryPolicy>,
    scheduler: Arc<crate::application::scheduler::Scheduler>, // Phase 3
    time_provider: Arc<dyn crate::port::TimeProvider>,        // For deterministic testing
}

impl Worker {
    /// Create a new worker with all Phase 2 + Phase 3 dependencies
    pub fn new(
        queue: impl Into<String>,
        job_repo: Arc<dyn JobRepository>,
        task_executor: Arc<dyn TaskExecutor>,
        system_probe: Arc<dyn SystemProbe>,
        retry_policy: Arc<RetryPolicy>,
        scheduler: Arc<crate::application::scheduler::Scheduler>, // Phase 3
        time_provider: Arc<dyn crate::port::TimeProvider>,
    ) -> Self {
        Self {
            queue: queue.into(),
            job_repo,
            task_executor,
            system_probe,
            retry_policy,
            scheduler,
            time_provider,
        }
    }

    /// Create a Phase 1 compatible worker (for backward compatibility in tests)
    pub fn new_phase1(queue: impl Into<String>, job_repo: Arc<dyn JobRepository>) -> Self {
        // Use mock implementations (core crate cannot depend on infrastructure)
        use crate::port::time_provider::SystemTimeProvider;
        use crate::port::TimeProvider;

        let time_provider: Arc<dyn TimeProvider> = Arc::new(SystemTimeProvider);
        let task_executor: Arc<dyn TaskExecutor> =
            Arc::new(crate::port::task_executor::mocks::MockTaskExecutor::new_success());
        let system_probe: Arc<dyn SystemProbe> =
            Arc::new(crate::port::system_probe::mocks::MockSystemProbe::new(50.0));
        let retry_policy = Arc::new(RetryPolicy::new(
            Arc::clone(&time_provider),
            DEFAULT_RETRY_BASE_DELAY_MS,
        ));

        let scheduler = Arc::new(crate::application::scheduler::Scheduler::new(
            system_probe.clone(),
            time_provider.clone(),
        ));
        Self::new(
            queue,
            job_repo,
            task_executor,
            system_probe,
            retry_policy,
            scheduler,
            time_provider,
        )
    }
    /// Run worker loop with graceful shutdown support
    pub async fn run(&self, mut shutdown: ShutdownToken) -> Result<()> {
        info!("Worker started for queue: {}", self.queue);
        loop {
            // Check for shutdown signal
            if shutdown.is_shutdown() {
                info!("Worker shutting down for queue: {}", self.queue);
                break;
            }
            match self.process_next_job().await {
                Ok(processed) => {
                    if !processed {
                        // No job available, sleep briefly (or wait for shutdown)
                        tokio::select! {
                            _ = sleep(IDLE_SLEEP_DURATION) => {},
                            _ = shutdown.wait() => {
                                info!("Worker interrupted during idle");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Worker error: {}", e);
                    tokio::select! {
                        _ = sleep(ERROR_RECOVERY_SLEEP_DURATION) => {},
                        _ = shutdown.wait() => {
                            info!("Worker interrupted during error recovery");
                            break;
                        }
                    }
                }
            }
        }
        info!("Worker stopped for queue: {}", self.queue);
        Ok(())
    }
    /// Process next job from queue (returns true if job was processed)
    pub async fn process_next_job(&self) -> Result<bool> {
        // Phase 2: Check system throttling before popping job (ADR-002)
        let metrics = self.system_probe.get_metrics().await;
        if metrics.cpu_usage_percent > CPU_THROTTLE_THRESHOLD {
            warn!(
                cpu_usage = %metrics.cpu_usage_percent,
                threshold = %CPU_THROTTLE_THRESHOLD,
                "System throttling: CPU > threshold, skipping job processing"
            );
            return Ok(false); // Don't process, system is overloaded
        }

        // Pop next job (already atomically set to RUNNING in DB)
        let mut job = match self.job_repo.pop_next(&self.queue).await? {
            Some(j) => j,
            None => return Ok(false), // No job available
        };

        // Phase 3: Check if job is ready based on scheduling conditions (ADR-050)
        if !self.scheduler.is_ready(&job).await {
            info!(
                job_id = %job.id,
                "Job not ready due to scheduling conditions, re-queuing"
            );
            // Re-queue the job (set back to QUEUED state)
            job.state = crate::domain::JobState::Queued;
            job.started_at = None;
            self.job_repo.update(&job).await?;
            return Ok(false);
        }

        info!("Processing job: {} ({})", job.id, job.job_type.as_str());

        // Execute job with panic isolation (ADR-002: Worker panic must not kill daemon)
        // Using tokio::task::spawn to isolate panics
        // 
        // Optimization: Use Arc to avoid cloning large payloads
        let job_arc = Arc::new(job);
        let job_for_exec = Arc::clone(&job_arc);
        let task_executor = Arc::clone(&self.task_executor);

        let handle = tokio::task::spawn(async move {
            // Execute directly without creating new Worker
            Self::execute_job_static(&task_executor, &job_for_exec).await
        });

        // Await the spawned task - panics will be caught by JoinHandle
        let execution_result = handle.await;

        // Extract job from Arc for mutation (try_unwrap to avoid clone if possible)
        let mut job = Arc::try_unwrap(job_arc)
            .unwrap_or_else(|arc| (*arc).clone()); // Fallback to clone if still referenced

        // Update job based on result (with retry logic - Phase 2, ADR-002)
        use crate::application::retry::RetryDecision;

        match execution_result {
            Ok(Ok(_)) => {
                // Task succeeded
                let now = self.time_provider.now_millis();
                job.complete(now)?;
                info!("Job completed: {}", job.id);
                self.job_repo.update(&job).await?;
            }
            Ok(Err(e)) => {
                // Task failed gracefully - check if we should retry
                match self.retry_policy.should_retry(&job) {
                    RetryDecision::Retry(delay_ms) => {
                        info!(
                            job_id = %job.id,
                            attempt = %job.attempts,
                            delay_ms = %delay_ms,
                            error = %e,
                            "Retrying job after failure"
                        );

                        self.retry_policy.prepare_for_retry(&mut job);
                        self.job_repo.update(&job).await?;
                    }
                    RetryDecision::Failed => {
                        error!("Job failed {} after max retries: {}", job.id, e);
                        let now = self.time_provider.now_millis();
                        job.fail(now);
                        self.job_repo.update(&job).await?;
                    }
                }
            }
            Err(join_err) => {
                // Task panicked or was cancelled (non-retryable)
                if join_err.is_panic() {
                    error!("Job panicked {}: {:?}", job.id, join_err);
                } else {
                    error!("Job cancelled {}: {:?}", job.id, join_err);
                }
                let now = self.time_provider.now_millis();
                job.fail(now);
                self.job_repo.update(&job).await?;
            }
        }
        Ok(true)
    }
    /// Execute job with real TaskExecutor (Phase 2)
    /// Static method to avoid unnecessary Worker cloning in spawn
    /// 
    /// Accepts Arc<Job> to avoid cloning large payloads
    async fn execute_job_static(task_executor: &Arc<dyn TaskExecutor>, job: &Arc<Job>) -> Result<()> {
        use crate::domain::ExecutionMode;

        // Check execution mode
        match &job.execution_mode {
            Some(ExecutionMode::Subprocess) | Some(ExecutionMode::InProcess) | None => {
                // Use TaskExecutor trait (works for both subprocess and in-process)
                info!(job_id = %job.id, execution_mode = ?job.execution_mode, "Executing job");

                let result = task_executor.execute(job).await?;

                if result.status != crate::port::task_executor::ExecutionStatus::Success {
                    return Err(crate::error::AppError::Internal(format!(
                        "Job execution failed: {:?}",
                        result.status
                    )));
                }

                info!(job_id = %job.id, duration_ms = %result.duration_ms, "Job executed successfully");
                Ok(())
            }
        }
    }
}
