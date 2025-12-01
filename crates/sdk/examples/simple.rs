//! Simple SDK Example
//!
//! Demonstrates basic usage of the Semantica SDK.
//!
//! # Usage
//!
//! 1. Start the daemon:
//!    ```bash
//!    cargo run --package semantica-daemon
//!    ```
//!
//! 2. Run this example:
//!    ```bash
//!    cargo run --example simple
//!    ```

use semantica_sdk::{EnqueueRequest, SematicaClient};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Semantica SDK - Simple Example");
    println!("================================\n");

    // 1. Connect to daemon
    println!("1. Connecting to daemon...");
    let client = SematicaClient::connect("http://127.0.0.1:9527").await?;
    println!("   ✓ Connected\n");

    // 2. Enqueue a job
    println!("2. Enqueuing a job...");
    let enqueue_response = client
        .enqueue(EnqueueRequest {
            job_type: "INDEX_FILE".to_string(),
            queue: "default".to_string(),
            subject_key: "examples/simple.rs".to_string(),
            priority: 5,
            payload: json!({
                "path": "examples/simple.rs",
                "mode": "full_index"
            }),
        })
        .await?;

    println!("   ✓ Job enqueued:");
    println!("     - ID: {}", enqueue_response.job_id);
    println!("     - State: {}", enqueue_response.state);
    println!("     - Queue: {}\n", enqueue_response.queue);

    // 3. Wait a bit for processing
    println!("3. Waiting 2 seconds...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    println!("   ✓ Done\n");

    // 4. Tail logs
    println!("4. Fetching job logs...");
    let logs_response = client.tail_logs(&enqueue_response.job_id, Some(10)).await?;

    println!("   ✓ Logs retrieved:");
    if let Some(log_path) = &logs_response.log_path {
        println!("     - Path: {}", log_path);
    }
    println!("     - Lines: {}", logs_response.lines.len());

    if !logs_response.lines.is_empty() {
        println!("\n   Last {} lines:", logs_response.lines.len());
        for line in &logs_response.lines {
            println!("     | {}", line);
        }
    }
    println!();

    // 5. Cancel the job (if still running)
    println!("5. Cancelling job...");
    let cancel_response = client.cancel(&enqueue_response.job_id).await?;

    if cancel_response.cancelled {
        println!("   ✓ Job cancelled");
    } else {
        println!("   ⚠ Job was already finished");
    }

    println!("\n✓ Example completed successfully!");

    Ok(())
}
