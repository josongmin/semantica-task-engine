//! SDK Request/Response Types
//!
//! Mirrors the JSON-RPC types from api-rpc crate.

use serde::{Deserialize, Serialize};

/// Request to enqueue a new job
#[derive(Debug, Clone, Serialize)]
pub struct EnqueueRequest {
    pub job_type: String,
    pub queue: String,
    pub subject_key: String,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub priority: i32,
}

/// Response from enqueue operation
#[derive(Debug, Clone, Deserialize)]
pub struct EnqueueResponse {
    pub job_id: String,
    pub state: String,
    pub queue: String,
}

/// Request to cancel a job
#[derive(Debug, Clone, Serialize)]
pub struct CancelRequest {
    pub job_id: String,
}

/// Response from cancel operation
#[derive(Debug, Clone, Deserialize)]
pub struct CancelResponse {
    pub job_id: String,
    pub cancelled: bool,
}

/// Request to tail job logs
#[derive(Debug, Clone, Serialize)]
pub struct TailLogsRequest {
    pub job_id: String,
    #[serde(default = "default_lines")]
    pub lines: usize,
}

#[allow(dead_code)] // Used by serde via #[serde(default)]
fn default_lines() -> usize {
    50
}

/// Response from tail logs operation
#[derive(Debug, Clone, Deserialize)]
pub struct TailLogsResponse {
    pub job_id: String,
    pub log_path: Option<String>,
    pub lines: Vec<String>,
}
