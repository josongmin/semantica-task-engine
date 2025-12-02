//! RPC Method Handlers
//!
//! Implements the business logic for each JSON-RPC method.

use crate::error::to_rpc_error;
use crate::rate_limiter::RateLimiter;
use crate::types::{
    CancelRequest, CancelResponse, EnqueueRequest, EnqueueResponse, MaintenanceRequest,
    MaintenanceResponse, StatsRequest, StatsResponse, TailLogsRequest, TailLogsResponse,
};
use jsonrpsee::types::ErrorObjectOwned;
use semantica_core::application::dev_task::enqueue;
use semantica_core::domain::JobState;
use semantica_core::port::job_repository::JobRepository;
use semantica_core::port::{IdProvider, Maintenance, TimeProvider, TransactionalJobRepository};
use std::sync::Arc;

/// RPC Handler with injected dependencies
pub struct RpcHandler {
    tx_job_repo: Arc<dyn TransactionalJobRepository>,
    job_repo: Arc<dyn JobRepository>,
    id_provider: Arc<dyn IdProvider>,
    time_provider: Arc<dyn TimeProvider>,
    maintenance: Arc<dyn Maintenance>,
    rate_limiter: Arc<RateLimiter>,
    start_time: std::time::Instant,
}

impl RpcHandler {
    pub fn new(
        tx_job_repo: Arc<dyn TransactionalJobRepository>,
        job_repo: Arc<dyn JobRepository>,
        id_provider: Arc<dyn IdProvider>,
        time_provider: Arc<dyn TimeProvider>,
        maintenance: Arc<dyn Maintenance>,
    ) -> Self {
        // Default: 200 burst, 100 req/sec (configurable via env)
        let max_burst: u32 = std::env::var("SEMANTICA_RATE_LIMIT_BURST")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(200);

        let rate_per_sec: u32 = std::env::var("SEMANTICA_RATE_LIMIT_RATE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        Self {
            tx_job_repo,
            job_repo,
            id_provider,
            time_provider,
            maintenance,
            rate_limiter: Arc::new(RateLimiter::new(max_burst, rate_per_sec)),
            start_time: std::time::Instant::now(),
        }
    }

    /// dev.enqueue.v1
    pub async fn enqueue(
        &self,
        params: EnqueueRequest,
    ) -> Result<EnqueueResponse, ErrorObjectOwned> {
        // Rate limiting check (DoS protection)
        if !self.rate_limiter.check().await {
            return Err(jsonrpsee::types::error::ErrorObject::owned(
                4003, // THROTTLED
                "Rate limit exceeded. Please slow down.",
                None::<()>,
            ));
        }

        let req = enqueue::EnqueueRequest {
            job_type: params.job_type,
            queue: params.queue.clone(),
            subject_key: params.subject_key,
            payload: params.payload,
            priority: params.priority,
        };

        let job_id = enqueue::execute(
            self.tx_job_repo.as_ref(),
            self.id_provider.as_ref(),
            self.time_provider.as_ref(),
            req,
        )
        .await
        .map_err(to_rpc_error)?;

        Ok(EnqueueResponse {
            job_id,
            state: "QUEUED".to_string(),
            queue: params.queue,
        })
    }

    /// dev.cancel.v1
    pub async fn cancel(&self, params: CancelRequest) -> Result<CancelResponse, ErrorObjectOwned> {
        // Rate limiting check (DoS protection)
        if !self.rate_limiter.check().await {
            return Err(jsonrpsee::types::error::ErrorObject::owned(
                4003, // THROTTLED
                "Rate limit exceeded. Please slow down.",
                None::<()>,
            ));
        }

        // Check if job exists
        let _job = self
            .job_repo
            .find_by_id(&params.job_id)
            .await
            .map_err(to_rpc_error)?
            .ok_or_else(|| {
                to_rpc_error(semantica_core::error::AppError::NotFound(format!(
                    "Job {} not found",
                    params.job_id
                )))
            })?;

        // Cancel logic: Partial update (optimization - only update state)
        let now = self.time_provider.now_millis();
        self.job_repo
            .update_state(&params.job_id, JobState::Cancelled, Some(now))
            .await
            .map_err(to_rpc_error)?;

        Ok(CancelResponse {
            job_id: params.job_id,
            cancelled: true,
        })
    }

    /// logs.tail.v1
    pub async fn tail_logs(
        &self,
        params: TailLogsRequest,
    ) -> Result<TailLogsResponse, ErrorObjectOwned> {
        let job = self
            .job_repo
            .find_by_id(&params.job_id)
            .await
            .map_err(to_rpc_error)?
            .ok_or_else(|| {
                to_rpc_error(semantica_core::error::AppError::NotFound(format!(
                    "Job {} not found",
                    params.job_id
                )))
            })?;

        // Read log file if exists
        let lines = if let Some(log_path) = &job.log_path {
            std::fs::read_to_string(log_path)
                .ok()
                .map(|content| {
                    let all_lines: Vec<&str> = content.lines().collect();
                    let start = all_lines.len().saturating_sub(params.lines);
                    all_lines[start..].iter().map(|s| s.to_string()).collect()
                })
                .unwrap_or_default()
        } else {
            vec![]
        };

        Ok(TailLogsResponse {
            job_id: params.job_id,
            log_path: job.log_path,
            lines,
        })
    }

    /// admin.stats.v1
    pub async fn stats(&self, _params: StatsRequest) -> Result<StatsResponse, ErrorObjectOwned> {
        const DEFAULT_QUEUE: &str = "default";

        // Get job counts by state using count_by_state
        let queued = self
            .job_repo
            .count_by_state(DEFAULT_QUEUE, JobState::Queued)
            .await
            .map_err(to_rpc_error)?;

        let running = self
            .job_repo
            .count_by_state(DEFAULT_QUEUE, JobState::Running)
            .await
            .map_err(to_rpc_error)?;

        let done = self
            .job_repo
            .count_by_state(DEFAULT_QUEUE, JobState::Done)
            .await
            .map_err(to_rpc_error)?;

        let failed = self
            .job_repo
            .count_by_state(DEFAULT_QUEUE, JobState::Failed)
            .await
            .map_err(to_rpc_error)?;

        // Get maintenance stats for DB size
        let stats = self.maintenance.get_stats().await.map_err(to_rpc_error)?;

        let total_jobs = stats.job_count;

        Ok(StatsResponse {
            total_jobs,
            queued_jobs: queued,
            running_jobs: running,
            done_jobs: done,
            failed_jobs: failed,
            db_size_bytes: stats.db_size_bytes,
            uptime_seconds: self.start_time.elapsed().as_secs() as i64,
        })
    }

    /// admin.maintenance.v1
    pub async fn maintenance(
        &self,
        params: MaintenanceRequest,
    ) -> Result<MaintenanceResponse, ErrorObjectOwned> {
        let stats_before = self.maintenance.get_stats().await.map_err(to_rpc_error)?;

        // Run VACUUM if forced or needed
        let vacuum_run = if params.force_vacuum || stats_before.fragmentation_percent > 10.0 {
            self.maintenance.vacuum().await.map_err(to_rpc_error)?;
            true
        } else {
            false
        };

        // Run garbage collection
        let jobs_deleted = self
            .maintenance
            .gc_finished_jobs(30) // 30 days
            .await
            .map_err(to_rpc_error)?;

        let artifacts_deleted = self
            .maintenance
            .gc_artifacts(30) // 30 days
            .await
            .map_err(to_rpc_error)?;

        let stats_after = self.maintenance.get_stats().await.map_err(to_rpc_error)?;

        Ok(MaintenanceResponse {
            vacuum_run,
            jobs_deleted,
            artifacts_deleted: artifacts_deleted as i64,
            db_size_before: stats_before.db_size_bytes,
            db_size_after: stats_after.db_size_bytes,
        })
    }
}
