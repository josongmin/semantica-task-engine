// Subprocess executor implementation (Phase 2)
// reason: async-trait, tokio for async process management (ADR-001)
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{info, warn};

use semantica_core::domain::Job;
use semantica_core::port::task_executor::{
    ExecutionError, ExecutionResult, ExecutionStatus, TaskExecutor,
};
use semantica_core::port::TimeProvider;
use std::sync::Arc;

// Type alias to simplify complex return types (Clippy warning fix)
type ParseResult = Result<
    (
        String,
        Vec<String>,
        HashMap<String, String>,
        String,
        Option<i64>,
    ),
    ExecutionError,
>;

/// Subprocess executor (Phase 2)
/// Spawns isolated child processes with environment allowlisting (ADR-040)
pub struct SubprocessExecutor {
    time_provider: Arc<dyn TimeProvider>,
    env_allowlist: Vec<String>,
}

impl SubprocessExecutor {
    /// Create a new subprocess executor
    ///
    /// # Arguments
    /// * `time_provider` - Time provider for duration tracking
    /// * `env_allowlist` - Allowed environment variables (security constraint, ADR-040)
    ///
    /// # Example
    /// ```ignore
    /// let executor = SubprocessExecutor::new(
    ///     Arc::new(SystemTimeProvider),
    ///     vec!["PATH".to_string(), "HOME".to_string(), "USER".to_string()],
    /// );
    /// ```
    pub fn new(time_provider: Arc<dyn TimeProvider>, env_allowlist: Vec<String>) -> Self {
        Self {
            time_provider,
            env_allowlist,
        }
    }

    /// Filter environment variables to allowlist only (ADR-040)
    fn filter_env(&self, env: &HashMap<String, String>) -> HashMap<String, String> {
        env.iter()
            .filter(|(k, _)| self.env_allowlist.contains(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Parse job payload to extract execution parameters
    fn parse_payload(&self, job: &Job) -> ParseResult {
        let payload = job.payload.as_value();

        let command = payload
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ExecutionError::InvalidPayload("Missing 'command' in payload".to_string())
            })?;

        let args: Vec<String> = payload
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let env: HashMap<String, String> = payload
            .get("env")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let working_dir = payload
            .get("working_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(".")
            .to_string();

        let timeout_ms = job.deadline.map(|d| {
            let now = self.time_provider.now_millis();
            (d - now).max(1000) // At least 1s
        });

        Ok((command.to_string(), args, env, working_dir, timeout_ms))
    }

    /// Spawn child process and wait for output
    async fn spawn_and_wait(
        &self,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        working_dir: &str,
        timeout_ms: Option<i64>,
    ) -> Result<std::process::Output, ExecutionError> {
        let filtered_env = self.filter_env(env);

        let child = Command::new(command)
            .args(args)
            .envs(&filtered_env)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ExecutionError::SpawnFailed(e.to_string()))?;

        if let Some(timeout_ms_val) = timeout_ms {
            match timeout(
                Duration::from_millis(timeout_ms_val as u64),
                child.wait_with_output(),
            )
            .await
            {
                Ok(Ok(output)) => Ok(output),
                Ok(Err(e)) => Err(ExecutionError::IoError(e.to_string())),
                Err(_) => Err(ExecutionError::Timeout(timeout_ms_val)),
            }
        } else {
            child
                .wait_with_output()
                .await
                .map_err(|e| ExecutionError::IoError(e.to_string()))
        }
    }

    /// Build execution result from process output
    fn build_result(&self, output: std::process::Output, duration_ms: i64) -> ExecutionResult {
        let status = if output.status.success() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Failed
        };

        ExecutionResult {
            status,
            exit_code: output.status.code(),
            duration_ms,
            stdout: Some(String::from_utf8_lossy(&output.stdout).to_string()),
            stderr: Some(String::from_utf8_lossy(&output.stderr).to_string()),
        }
    }

    /// Internal execute method (extracted for function length compliance)
    async fn execute_internal(
        &self,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        working_dir: &str,
        timeout_ms: Option<i64>,
    ) -> Result<ExecutionResult, ExecutionError> {
        let start_time = self.time_provider.now_millis();

        info!(
            command = %command,
            args = ?args,
            working_dir = %working_dir,
            timeout_ms = ?timeout_ms,
            "Starting subprocess execution"
        );

        let output = self
            .spawn_and_wait(command, args, env, working_dir, timeout_ms)
            .await?;

        let end_time = self.time_provider.now_millis();
        let duration_ms = end_time - start_time;

        let result = self.build_result(output, duration_ms);

        info!(
            command = %command,
            duration_ms = %duration_ms,
            exit_code = ?result.exit_code,
            status = ?result.status,
            "Subprocess execution completed"
        );

        Ok(result)
    }

    /// Kill process with SIGTERM first, then SIGKILL if needed (ADR-002)
    async fn kill_graceful(&self, pid: i32) -> Result<(), ExecutionError> {
        // Use shared constant (ADR: No magic values)
        const GRACEFUL_TIMEOUT_MS: i64 =
            semantica_core::application::worker::constants::GRACEFUL_SHUTDOWN_TIMEOUT_MS;

        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            // Step 1: Send SIGTERM for graceful shutdown
            info!(pid = %pid, "Sending SIGTERM for graceful shutdown");
            kill(Pid::from_raw(pid), Signal::SIGTERM)
                .map_err(|e| ExecutionError::Killed(format!("SIGTERM failed: {}", e)))?;

            // Step 2: Wait for process to exit (check every 100ms)
            let start_time = self.time_provider.now_millis();
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Check if process is still alive
                if kill(Pid::from_raw(pid), Signal::try_from(0).ok()).is_err() {
                    // Process exited gracefully
                    info!(pid = %pid, "Process exited gracefully after SIGTERM");
                    return Ok(());
                }

                // Timeout: force kill with SIGKILL
                if self.time_provider.now_millis() - start_time > GRACEFUL_TIMEOUT_MS {
                    warn!(pid = %pid, "Process did not exit after SIGTERM, sending SIGKILL");
                    kill(Pid::from_raw(pid), Signal::SIGKILL)
                        .map_err(|e| ExecutionError::Killed(format!("SIGKILL failed: {}", e)))?;
                    return Ok(());
                }
            }
        }

        #[cfg(windows)]
        {
            // Windows: taskkill /PID with /F flag (force kill)
            use std::process::Command;

            info!(pid = %pid, "Killing process on Windows");
            let output = Command::new("taskkill")
                .args(&["/F", "/PID", &pid.to_string()])
                .output()
                .map_err(|e| ExecutionError::Killed(e.to_string()))?;

            if !output.status.success() {
                return Err(ExecutionError::Killed(format!(
                    "taskkill failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            Ok(())
        }
    }
}

#[async_trait]
impl TaskExecutor for SubprocessExecutor {
    async fn execute(&self, job: &Job) -> Result<ExecutionResult, ExecutionError> {
        let (command, args, env, working_dir, timeout_ms) = self.parse_payload(job)?;
        self.execute_internal(&command, &args, &env, &working_dir, timeout_ms)
            .await
    }

    async fn kill(&self, pid: i32) -> Result<(), ExecutionError> {
        self.kill_graceful(pid).await
    }

    fn is_alive(&self, pid: i32) -> bool {
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            // Signal 0 checks if process exists without actually sending a signal
            kill(Pid::from_raw(pid), Signal::try_from(0).ok()).is_ok()
        }

        #[cfg(windows)]
        {
            use std::process::Command;

            let output = Command::new("tasklist")
                .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
                .output();

            if let Ok(output) = output {
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.contains(&pid.to_string())
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semantica_core::domain::{ExecutionMode, Job, JobPayload, JobType};
    use semantica_core::port::time_provider::SystemTimeProvider;

    #[tokio::test]
    async fn test_execute_success() {
        let executor = SubprocessExecutor::new(
            Arc::new(SystemTimeProvider),
            vec!["PATH".to_string(), "HOME".to_string()],
        );

        let mut job = Job::new_test(
            "test_queue",
            JobType::new("TEST"),
            "test::subject",
            1,
            JobPayload::new(serde_json::json!({
                "command": "echo",
                "args": ["hello"]
            })),
        );
        job.execution_mode = Some(ExecutionMode::Subprocess);

        let result = executor.execute(&job).await.unwrap();

        assert_eq!(result.status, ExecutionStatus::Success);
        assert!(result.stdout.unwrap_or_default().contains("hello"));
    }

    #[tokio::test]
    async fn test_execute_timeout() {
        let executor = SubprocessExecutor::new(Arc::new(SystemTimeProvider), vec![]);

        let mut job = Job::new_test(
            "test_queue",
            JobType::new("TEST"),
            "test::subject",
            1,
            JobPayload::new(serde_json::json!({
                "command": "sleep",
                "args": ["10"]
            })),
        );
        job.execution_mode = Some(ExecutionMode::Subprocess);
        job.deadline = Some(SystemTimeProvider.now_millis() + 100); // 100ms deadline

        let result = executor.execute(&job).await;

        assert!(matches!(result, Err(ExecutionError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_env_filtering() {
        let executor = SubprocessExecutor::new(
            Arc::new(SystemTimeProvider),
            vec!["ALLOWED_VAR".to_string()],
        );

        let mut env = HashMap::new();
        env.insert("ALLOWED_VAR".to_string(), "value1".to_string());
        env.insert("BLOCKED_VAR".to_string(), "value2".to_string());

        let filtered = executor.filter_env(&env);

        assert_eq!(filtered.len(), 1);
        assert!(filtered.contains_key("ALLOWED_VAR"));
        assert!(!filtered.contains_key("BLOCKED_VAR"));
    }
}
