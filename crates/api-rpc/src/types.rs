//! RPC Request/Response Types
//!
//! Defines the JSON-RPC method parameters and results (ADR-020).

use serde::{Deserialize, Serialize};

/// dev.enqueue.v1 - Enqueue a job
#[derive(Debug, Deserialize)]
pub struct EnqueueRequest {
    pub job_type: String,
    pub queue: String,
    pub subject_key: String,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnqueueResponse {
    pub job_id: String,
    pub state: String,
    pub queue: String,
}

/// dev.cancel.v1 - Cancel a job
#[derive(Debug, Deserialize)]
pub struct CancelRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CancelResponse {
    pub job_id: String,
    pub cancelled: bool,
}

/// logs.tail.v1 - Tail job logs
#[derive(Debug, Deserialize)]
pub struct TailLogsRequest {
    pub job_id: String,
    #[serde(default = "default_lines")]
    pub lines: usize,
}

fn default_lines() -> usize {
    50
}

#[derive(Debug, Clone, Serialize)]
pub struct TailLogsResponse {
    pub job_id: String,
    pub log_path: Option<String>,
    pub lines: Vec<String>,
}

/// admin.stats.v1 - Get system statistics
#[derive(Debug, Deserialize)]
pub struct StatsRequest {
    // No parameters needed
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsResponse {
    pub total_jobs: i64,
    pub queued_jobs: i64,
    pub running_jobs: i64,
    pub done_jobs: i64,
    pub failed_jobs: i64,
    pub db_size_bytes: i64,
    pub uptime_seconds: i64,
}

/// admin.maintenance.v1 - Run manual maintenance
#[derive(Debug, Deserialize)]
pub struct MaintenanceRequest {
    #[serde(default)]
    pub force_vacuum: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MaintenanceResponse {
    pub vacuum_run: bool,
    pub jobs_deleted: i64,
    pub artifacts_deleted: i64,
    pub db_size_before: i64,
    pub db_size_after: i64,
}
