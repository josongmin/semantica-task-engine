//! Semantica CLI - Command-line interface for Semantica Task Engine
//! Phase 4: User experience improvements

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tabled::{Table, Tabled};

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:9527";

#[derive(Parser)]
#[command(name = "semantica")]
#[command(about = "Semantica Task Engine CLI", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// RPC server URL
    #[arg(long, env = "SEMANTICA_RPC_URL", default_value = DEFAULT_RPC_URL)]
    rpc_url: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Enqueue a new job
    Enqueue {
        /// Job type (e.g., INDEX_FILE, BUILD, TEST)
        #[arg(short, long)]
        job_type: String,

        /// Queue name (default: "default")
        #[arg(short, long, default_value = "default")]
        queue: String,

        /// Subject key for supersede logic
        #[arg(short, long)]
        subject: String,

        /// Priority (higher = more urgent)
        #[arg(short, long, default_value = "0")]
        priority: i32,

        /// Payload as JSON string
        #[arg(long)]
        payload: String,
    },

    /// Cancel a job
    Cancel {
        /// Job ID
        job_id: String,
    },

    /// Get job logs
    Logs {
        /// Job ID
        job_id: String,

        /// Number of lines to tail
        #[arg(short = 'n', long, default_value = "100")]
        lines: usize,
    },

    /// Show system status
    Status,

    /// Run maintenance operations
    Maintenance {
        /// Force VACUUM even if not needed
        #[arg(long)]
        force_vacuum: bool,
    },
}

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Deserialize, Tabled)]
struct EnqueueResult {
    job_id: String,
    state: String,
    queue: String,
}

async fn call_rpc(url: &str, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: method.to_string(),
        params,
        id: 1,
    };

    let client = reqwest::Client::new();
    let response: JsonRpcResponse = client
        .post(url)
        .json(&request)
        .send()
        .await
        .context("Failed to connect to daemon")?
        .json()
        .await
        .context("Failed to parse response")?;

    if let Some(error) = response.error {
        anyhow::bail!("RPC error ({}): {}", error.code, error.message);
    }

    response
        .result
        .ok_or_else(|| anyhow::anyhow!("No result in response"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Enqueue {
            job_type,
            queue,
            subject,
            priority,
            payload,
        } => {
            let payload_json: serde_json::Value =
                serde_json::from_str(&payload).context("Invalid JSON payload")?;

            let params = json!({
                "job_type": job_type,
                "queue": queue,
                "subject_key": subject,
                "priority": priority,
                "payload": payload_json,
            });

            let result = call_rpc(&cli.rpc_url, "dev.enqueue.v1", params).await?;
            let enqueue_result: EnqueueResult = serde_json::from_value(result)?;

            println!("{}", "✓ Job enqueued successfully".green().bold());
            println!();

            let table = Table::new(vec![enqueue_result]).to_string();
            println!("{}", table);
        }

        Commands::Cancel { job_id } => {
            let params = json!({
                "job_id": job_id,
            });

            call_rpc(&cli.rpc_url, "dev.cancel.v1", params).await?;

            println!("{}", format!("✓ Job {} cancelled", job_id).green().bold());
        }

        Commands::Logs { job_id, lines } => {
            let params = json!({
                "job_id": job_id,
                "lines": lines,
            });

            let result = call_rpc(&cli.rpc_url, "logs.tail.v1", params).await?;

            if let Some(logs) = result.get("logs").and_then(|v| v.as_str()) {
                println!("{}", format!("Logs for job {}:", job_id).cyan().bold());
                println!("{}", logs);
            } else {
                println!("{}", "No logs available".yellow());
            }
        }

        Commands::Status => {
            println!("{}", "System Status".cyan().bold());
            println!();

            match call_rpc(&cli.rpc_url, "admin.stats.v1", json!({})).await {
                Ok(stats) => {
                    println!("  {} {}", "RPC URL:".bold(), cli.rpc_url);
                    println!("  {} {}", "Status:".bold(), "ONLINE".green());
                    println!();
                    println!("  {} {}", "Total Jobs:".bold(), stats["total_jobs"]);
                    println!("  {} {}", "Queued:".bold(), stats["queued_jobs"]);
                    println!("  {} {}", "Running:".bold(), stats["running_jobs"]);
                    println!("  {} {}", "Done:".bold(), stats["done_jobs"]);
                    println!("  {} {}", "Failed:".bold(), stats["failed_jobs"]);
                    println!();
                    let db_mb =
                        stats["db_size_bytes"].as_i64().unwrap_or(0) as f64 / (1024.0 * 1024.0);
                    println!("  {} {:.2} MB", "DB Size:".bold(), db_mb);
                    println!("  {} {} seconds", "Uptime:".bold(), stats["uptime_seconds"]);
                }
                Err(e) => {
                    println!("  {} {}", "Status:".bold(), "ERROR".red());
                    println!("  {} {}", "Error:".bold(), e);
                }
            }
        }

        Commands::Maintenance { force_vacuum } => {
            println!("{}", "Running maintenance...".cyan().bold());
            println!();

            if force_vacuum {
                println!("  {} Force VACUUM enabled", "•".bold());
            }

            let params = json!({ "force_vacuum": force_vacuum });

            match call_rpc(&cli.rpc_url, "admin.maintenance.v1", params).await {
                Ok(result) => {
                    println!("  ✓ Maintenance completed");
                    println!();
                    if result["vacuum_run"].as_bool().unwrap_or(false) {
                        println!("  {} VACUUM executed", "✓".green());
                    } else {
                        println!("  ○ VACUUM skipped (not needed)");
                    }
                    println!("  {} {} jobs deleted", "✓".green(), result["jobs_deleted"]);
                    println!(
                        "  {} {} artifacts deleted",
                        "✓".green(),
                        result["artifacts_deleted"]
                    );
                    println!();
                    let size_before_mb =
                        result["db_size_before"].as_i64().unwrap_or(0) as f64 / (1024.0 * 1024.0);
                    let size_after_mb =
                        result["db_size_after"].as_i64().unwrap_or(0) as f64 / (1024.0 * 1024.0);
                    println!(
                        "  {} {:.2} MB → {:.2} MB",
                        "DB Size:".bold(),
                        size_before_mb,
                        size_after_mb
                    );
                    let saved_mb = size_before_mb - size_after_mb;
                    if saved_mb > 0.0 {
                        println!("  {} {:.2} MB saved", "💾".bold(), saved_mb);
                    }
                }
                Err(e) => {
                    println!("  {} Maintenance failed: {}", "✗".red(), e);
                }
            }
        }
    }

    Ok(())
}
