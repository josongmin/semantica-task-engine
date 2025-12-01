//! Semantica Client Implementation

use crate::error::{Result, SdkError};
use crate::types::{
    CancelRequest, CancelResponse, EnqueueRequest, EnqueueResponse, TailLogsRequest,
    TailLogsResponse,
};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use std::time::Duration;

/// SemanticaTask Engine Client
///
/// Provides a high-level interface to interact with the SemanticaTask daemon.
///
/// # Example
///
/// ```no_run
/// use semantica_task_sdk::SemanticaTaskClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = SemanticaTaskClient::connect("http://127.0.0.1:9527").await?;
/// # Ok(())
/// # }
/// ```
pub struct SemanticaTaskClient {
    client: HttpClient,
}

impl SemanticaTaskClient {
    /// Connect to SemanticaTask daemon
    ///
    /// # Arguments
    ///
    /// * `url` - RPC endpoint URL (e.g., `http://127.0.0.1:9527`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use semantica_sdk::SemanticaTaskClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = SemanticaTaskClient::connect("http://127.0.0.1:9527").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(url: impl AsRef<str>) -> Result<Self> {
        let url = url.as_ref();

        let client = HttpClientBuilder::default()
            .request_timeout(Duration::from_secs(30))
            .build(url)
            .map_err(|e| SdkError::Connection(format!("Failed to create client: {}", e)))?;

        Ok(Self { client })
    }

    /// Enqueue a new job
    ///
    /// # Arguments
    ///
    /// * `request` - Job enqueue parameters
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use semantica_task_sdk::{SemanticaTaskClient, EnqueueRequest};
    /// # use serde_json::json;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = SemanticaTaskClient::connect("http://127.0.0.1:9527").await?;
    /// let response = client.enqueue(EnqueueRequest {
    ///     job_type: "INDEX_FILE".to_string(),
    ///     queue: "default".to_string(),
    ///     subject_key: "src/main.rs".to_string(),
    ///     priority: 0,
    ///     payload: json!({"path": "src/main.rs"}),
    /// }).await?;
    ///
    /// println!("Job ID: {}", response.job_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn enqueue(&self, request: EnqueueRequest) -> Result<EnqueueResponse> {
        let params = rpc_params![request];
        let response: EnqueueResponse = self.client.request("dev.enqueue.v1", params).await?;

        Ok(response)
    }

    /// Cancel a job
    ///
    /// # Arguments
    ///
    /// * `job_id` - ID of the job to cancel
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use semantica_sdk::SemanticaTaskClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = SemanticaTaskClient::connect("http://127.0.0.1:9527").await?;
    /// let response = client.cancel("job-123").await?;
    /// assert!(response.cancelled);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(&self, job_id: impl Into<String>) -> Result<CancelResponse> {
        let request = CancelRequest {
            job_id: job_id.into(),
        };
        let params = rpc_params![request];
        let response: CancelResponse = self.client.request("dev.cancel.v1", params).await?;

        Ok(response)
    }

    /// Tail job logs
    ///
    /// # Arguments
    ///
    /// * `job_id` - ID of the job
    /// * `lines` - Number of lines to retrieve (default: 50)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use semantica_sdk::SemanticaTaskClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = SemanticaTaskClient::connect("http://127.0.0.1:9527").await?;
    /// let response = client.tail_logs("job-123", Some(100)).await?;
    /// for line in response.lines {
    ///     println!("{}", line);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn tail_logs(
        &self,
        job_id: impl Into<String>,
        lines: Option<usize>,
    ) -> Result<TailLogsResponse> {
        let request = TailLogsRequest {
            job_id: job_id.into(),
            lines: lines.unwrap_or(50),
        };
        let params = rpc_params![request];
        let response: TailLogsResponse = self.client.request("logs.tail.v1", params).await?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sdk_types() {
        // Basic smoke test to ensure SDK compiles
        // Integration tests require running daemon
    }
}
