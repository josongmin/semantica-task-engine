//! SemanticaTask SDK - Rust Client Library
//!
//! Provides a convenient client for interacting with SemanticaTask Engine daemon.
//!
//! # Example
//!
//! ```no_run
//! use semantica_task_sdk::{SemanticaTaskClient, EnqueueRequest};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to daemon
//!     let client = SemanticaTaskClient::connect("http://127.0.0.1:9527").await?;
//!
//!     // Enqueue a job
//!     let response = client.enqueue(EnqueueRequest {
//!         job_type: "INDEX_FILE".to_string(),
//!         queue: "default".to_string(),
//!         subject_key: "src/main.rs".to_string(),
//!         priority: 0,
//!         payload: json!({"path": "src/main.rs"}),
//!     }).await?;
//!
//!     println!("Job enqueued: {}", response.job_id);
//!
//!     Ok(())
//! }
//! ```

mod client;
mod error;
mod types;

pub use client::SemanticaTaskClient;
pub use error::{Result, SdkError};
pub use types::{
    CancelRequest, CancelResponse, EnqueueRequest, EnqueueResponse, TailLogsRequest,
    TailLogsResponse,
};
