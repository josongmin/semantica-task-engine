// Task Executor Port (Phase 2, ADR-002)
// Abstraction for executing external tasks (subprocess or in-process)

use crate::domain::Job;
use async_trait::async_trait;
use thiserror::Error;

/// Result of task execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: ExecutionStatus,
    pub duration_ms: i64,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

/// Execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStatus {
    Success,
    Failed,
    Timeout,
    Killed,
}

/// Execution errors
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Process timeout after {0}ms")]
    Timeout(i64),

    #[error("Process killed: {0}")]
    Killed(String),

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    #[error("IO error: {0}")]
    IoError(String),
}

/// Task Executor trait
///
/// Implementations:
/// - SubprocessExecutor: spawns external process
/// - InProcessExecutor: runs function in current process (future)
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a job and return the result
    ///
    /// # Errors
    /// - ExecutionError::SpawnFailed if process cannot be started
    /// - ExecutionError::Timeout if execution exceeds deadline
    /// - ExecutionError::InvalidPayload if job payload is malformed
    async fn execute(&self, job: &Job) -> Result<ExecutionResult, ExecutionError>;

    /// Kill a running process by PID
    ///
    /// # Arguments
    /// * `pid` - Process ID to kill
    ///
    /// # Errors
    /// - ExecutionError::Killed if process cannot be killed
    async fn kill(&self, pid: i32) -> Result<(), ExecutionError>;

    /// Check if a process is still alive
    ///
    /// # Arguments
    /// * `pid` - Process ID to check
    fn is_alive(&self, pid: i32) -> bool;
}

// ============================================================================
// Mock Implementations for Testing
// ============================================================================

pub mod mocks {
    use super::*;
    use std::sync::{Arc, Mutex};
    /// Mock executor behavior
    #[derive(Debug, Clone)]
    pub enum MockBehavior {
        /// Always succeed
        Success,
        /// Always fail with message
        Fail(String),
        /// Panic with message (for panic isolation testing)
        Panic(String),
        /// Timeout after N ms
        Timeout(i64),
    }
    /// Mock Task Executor for testing
    pub struct MockTaskExecutor {
        behavior: Arc<Mutex<MockBehavior>>,
        call_count: Arc<Mutex<usize>>,
    }
    impl MockTaskExecutor {
        pub fn new(behavior: MockBehavior) -> Self {
            Self {
                behavior: Arc::new(Mutex::new(behavior)),
                call_count: Arc::new(Mutex::new(0)),
            }
        }
        pub fn new_panic_inducing(message: impl Into<String>) -> Self {
            Self::new(MockBehavior::Panic(message.into()))
        }
        pub fn new_success() -> Self {
            Self::new(MockBehavior::Success)
        }
        pub fn new_fail(message: impl Into<String>) -> Self {
            Self::new(MockBehavior::Fail(message.into()))
        }
        pub fn call_count(&self) -> usize {
            *self.call_count.lock().unwrap()
        }
    }
    #[async_trait]
    impl TaskExecutor for MockTaskExecutor {
        async fn execute(&self, _job: &Job) -> Result<ExecutionResult, ExecutionError> {
            *self.call_count.lock().unwrap() += 1;

            let behavior = self.behavior.lock().unwrap().clone();

            match behavior {
                MockBehavior::Success => Ok(ExecutionResult {
                    status: ExecutionStatus::Success,
                    duration_ms: 100,
                    exit_code: Some(0),
                    stdout: Some("mock output".to_string()),
                    stderr: None,
                }),
                MockBehavior::Fail(msg) => Err(ExecutionError::SpawnFailed(msg)),
                MockBehavior::Panic(msg) => {
                    panic!("{}", msg); // Actually panic for panic isolation testing
                }
                MockBehavior::Timeout(ms) => Err(ExecutionError::Timeout(ms)),
            }
        }
        async fn kill(&self, _pid: i32) -> Result<(), ExecutionError> {
            Ok(())
        }
        fn is_alive(&self, _pid: i32) -> bool {
            false
        }
    }
}
