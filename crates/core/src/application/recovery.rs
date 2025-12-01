// Crash recovery logic (Phase 2, ADR-002)
use crate::domain::{Job, JobState};
use crate::port::{JobRepository, TaskExecutor, TimeProvider};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::application::worker::constants::DEFAULT_RECOVERY_WINDOW_MS;

/// Crash recovery service
///
/// On daemon startup, detects and recovers jobs that were RUNNING when daemon crashed
pub struct RecoveryService {
    job_repo: Arc<dyn JobRepository>,
    task_executor: Arc<dyn TaskExecutor>,
    time_provider: Arc<dyn TimeProvider>,
    recovery_window_ms: i64,
}

impl RecoveryService {
    /// Create a new recovery service
    ///
    /// # Arguments
    /// * `job_repo` - Job repository
    /// * `task_executor` - Task executor for checking/killing processes
    /// * `time_provider` - Time provider
    /// * `recovery_window_ms` - Optional custom recovery window (default: 5 minutes)
    ///
    /// # Example
    /// ```ignore
    /// let recovery = RecoveryService::new(
    ///     job_repo,
    ///     task_executor,
    ///     time_provider,
    ///     None,
    /// );
    /// recovery.recover_orphaned_jobs().await.unwrap();
    /// ```
    pub fn new(
        job_repo: Arc<dyn JobRepository>,
        task_executor: Arc<dyn TaskExecutor>,
        time_provider: Arc<dyn TimeProvider>,
        recovery_window_ms: Option<i64>,
    ) -> Self {
        Self {
            job_repo,
            task_executor,
            time_provider,
            recovery_window_ms: recovery_window_ms.unwrap_or(DEFAULT_RECOVERY_WINDOW_MS),
        }
    }

    /// Recover orphaned jobs on daemon startup
    ///
    /// Algorithm (ADR-002):
    /// 1. Find all RUNNING jobs with `started_at < now - recovery_window`
    /// 2. For jobs with PID:
    ///    - Check if process is alive
    ///    - If alive: kill process
    ///    - Mark job as FAILED with reason "daemon_crash"
    /// 3. For jobs without PID (in-process):
    ///    - Mark as REQUEUED (can be retried)
    ///
    /// # Returns
    /// Number of jobs recovered
    pub async fn recover_orphaned_jobs(&self) -> crate::error::Result<usize> {
        let now = self.time_provider.now_millis();
        let cutoff = now - self.recovery_window_ms;

        info!(
            cutoff_time = %cutoff,
            recovery_window_ms = %self.recovery_window_ms,
            "Starting orphaned job recovery"
        );

        // Find all RUNNING jobs
        let running_jobs = self.job_repo.find_by_state(JobState::Running).await?;
        let mut recovered_count = 0;

        for mut job in running_jobs {
            // Check if job is orphaned (started_at is too old)
            if let Some(started_at) = job.started_at {
                if started_at < cutoff {
                    info!(
                        job_id = %job.id,
                        started_at = %started_at,
                        cutoff = %cutoff,
                        pid = ?job.pid,
                        "Recovering orphaned job"
                    );

                    self.recover_single_job(&mut job).await?;
                    recovered_count += 1;
                }
            } else {
                // RUNNING job without started_at is inconsistent, mark as FAILED
                warn!(
                    job_id = %job.id,
                    "RUNNING job without started_at, marking as FAILED"
                );

                job.state = JobState::Failed;
                job.finished_at = Some(now);
                self.job_repo.update(&job).await?;
                recovered_count += 1;
            }
        }

        info!(recovered_count = %recovered_count, "Orphaned job recovery complete");
        Ok(recovered_count)
    }

    /// Recover a single orphaned job
    async fn recover_single_job(&self, job: &mut Job) -> crate::error::Result<()> {
        let now = self.time_provider.now_millis();

        if let Some(pid) = job.pid {
            // Subprocess job - check if process is alive
            if self.task_executor.is_alive(pid) {
                warn!(
                    job_id = %job.id,
                    pid = %pid,
                    "Orphaned subprocess still alive, killing"
                );

                // Try to kill the process
                if let Err(e) = self.task_executor.kill(pid).await {
                    error!(
                        job_id = %job.id,
                        pid = %pid,
                        error = %e,
                        "Failed to kill orphaned process"
                    );
                }
            }

            // Mark as FAILED (subprocess jobs are not safe to retry)
            job.state = JobState::Failed;
            job.finished_at = Some(now);
            job.pid = None;

            info!(
                job_id = %job.id,
                pid = %pid,
                "Subprocess job marked as FAILED after recovery"
            );
        } else {
            // In-process job - safe to requeue
            job.state = JobState::Queued;
            job.started_at = None;

            info!(
                job_id = %job.id,
                "In-process job requeued after recovery"
            );
        }

        self.job_repo.update(job).await?;
        Ok(())
    }

    /// Cleanup zombie processes (processes that exist but job is not RUNNING)
    ///
    /// This is a defensive measure to kill any leaked processes
    pub async fn cleanup_zombies(&self) -> crate::error::Result<usize> {
        info!("Starting zombie process cleanup");

        // Check all job states for potential zombies
        let states = vec![
            JobState::Queued,
            JobState::Done,
            JobState::Failed,
            JobState::Superseded,
        ];

        let mut cleaned_count = 0;

        for state in states {
            let jobs = self.job_repo.find_by_state(state).await?;

            for job in jobs {
                // If job has PID but is not RUNNING, process might be zombie
                if let Some(pid) = job.pid {
                    if self.task_executor.is_alive(pid) {
                        warn!(
                            job_id = %job.id,
                            pid = %pid,
                            state = ?job.state,
                            "Found zombie process, killing"
                        );

                        if let Err(e) = self.task_executor.kill(pid).await {
                            error!(
                                job_id = %job.id,
                                pid = %pid,
                                error = %e,
                                "Failed to kill zombie process"
                            );
                        } else {
                            cleaned_count += 1;
                        }
                    }
                }
            }
        }

        info!(cleaned_count = %cleaned_count, "Zombie process cleanup complete");
        Ok(cleaned_count)
    }
}
