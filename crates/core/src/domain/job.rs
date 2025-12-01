// Job Domain Model (Phase 1)

use serde::{Deserialize, Serialize};

/// Job ID (UUID v4)
pub type JobId = String;

/// Queue identifier
pub type QueueId = String;

/// Job State (Phase 1: Minimal set)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed,
    Superseded,
    Cancelled,
    Requeued,
}

/// Execution Mode (Phase 2)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionMode {
    InProcess,
    Subprocess,
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionMode::InProcess => write!(f, "IN_PROCESS"),
            ExecutionMode::Subprocess => write!(f, "SUBPROCESS"),
        }
    }
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobState::Queued => write!(f, "QUEUED"),
            JobState::Running => write!(f, "RUNNING"),
            JobState::Done => write!(f, "DONE"),
            JobState::Failed => write!(f, "FAILED"),
            JobState::Superseded => write!(f, "SUPERSEDED"),
            JobState::Cancelled => write!(f, "CANCELLED"),
            JobState::Requeued => write!(f, "REQUEUED"),
        }
    }
}

/// Job Type (example types for Phase 1)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobType(String);

impl JobType {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Priority (higher number = higher priority)
pub type Priority = i32;

/// Subject Key (for supersede logic)
pub type SubjectKey = String;

/// Generation (for supersede logic)
pub type Generation = i64;

/// Job Payload (JSON serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPayload(serde_json::Value);

impl JobPayload {
    pub fn new(value: serde_json::Value) -> Self {
        Self(value)
    }

    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }
}

/// Job Entity (Phase 1 + Phase 2 + Phase 3 fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    // Phase 1: Core Identity
    pub id: JobId,
    pub queue: QueueId,
    pub job_type: JobType,
    pub subject_key: SubjectKey,
    pub generation: Generation,

    pub priority: Priority,
    pub state: JobState,

    pub created_at: i64, // epoch ms
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,

    pub payload: JobPayload,
    pub log_path: Option<String>,

    // Phase 2: Execution & Resilience
    pub execution_mode: Option<ExecutionMode>,
    pub pid: Option<i32>,
    pub env_vars: Option<serde_json::Value>, // JSON

    // Phase 2: Retry Logic
    pub attempts: i32,
    pub max_attempts: i32,
    pub backoff_factor: f64,

    // Phase 2: Timeouts
    pub deadline: Option<i64>, // Epoch ms
    pub ttl_ms: Option<i64>,   // Milliseconds

    // Phase 2: Tracing
    pub trace_id: Option<String>,

    // Phase 3: Scheduling & Conditions
    pub schedule_at: Option<i64>, // Unix timestamp (ms) when job should run
    pub wait_for_idle: bool,      // Wait for system idle
    pub require_charging: bool,   // Require device charging
    pub wait_for_event: Option<String>, // Event name to wait for (e.g., "pr_merged")

    // Phase 4: UX & Operational
    pub user_tag: Option<String>,       // User-defined tag for filtering
    pub parent_job_id: Option<String>,  // Parent job ID for chains
    pub chain_group_id: Option<String>, // Chain/batch group identifier
    pub result_summary: Option<String>, // JSON result summary
    pub artifacts: Option<String>,      // Comma-separated artifact paths
}

impl Job {
    /// Create a test job with auto-generated ID and timestamp (for tests only)
    ///
    /// Create a test job with deterministic ID and timestamp.
    ///
    /// Uses a simple counter for deterministic test IDs (test-1, test-2, ...).
    /// Timestamps start at 1000 and increment by 1000.
    ///
    /// **Note**: This method should only be used in tests. For production code,
    /// always inject ID and time via providers.
    pub fn new_test(
        queue: impl Into<String>,
        job_type: JobType,
        subject_key: impl Into<String>,
        generation: Generation,
        payload: JobPayload,
    ) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = format!("test-{}", counter);
        let created_at = (counter * 1000) as i64;

        Self::new(
            id,
            created_at,
            queue,
            job_type,
            subject_key,
            generation,
            payload,
        )
    }
}

impl Job {
    /// Create a new job with default values (for testing)
    /// Production code should use builder pattern or factory with TimeProvider
    /// Create a new Job
    ///
    /// # Arguments
    ///
    /// * `id` - Unique job ID (injected, not generated)
    /// * `created_at` - Creation timestamp in epoch ms (injected, not system time)
    /// * `queue` - Queue name
    /// * `job_type` - Job type
    /// * `subject_key` - Subject key for supersede logic
    /// * `generation` - Generation number
    /// * `payload` - Job payload
    pub fn new(
        id: impl Into<String>,
        created_at: i64,
        queue: impl Into<String>,
        job_type: JobType,
        subject_key: impl Into<String>,
        generation: Generation,
        payload: JobPayload,
    ) -> Self {
        Self {
            // Phase 1 fields
            id: id.into(),
            queue: queue.into(),
            job_type,
            subject_key: subject_key.into(),
            generation,
            priority: 0,
            state: JobState::Queued,
            created_at,
            started_at: None,
            finished_at: None,
            payload,
            log_path: None,

            // Phase 2 defaults
            execution_mode: Some(ExecutionMode::InProcess), // Default to in-process
            pid: None,
            env_vars: None,
            attempts: 0,
            max_attempts: 3, // Default retry count
            backoff_factor: 2.0,
            deadline: None,
            ttl_ms: None,
            trace_id: None,

            // Phase 3 defaults
            schedule_at: None,
            wait_for_idle: false,
            require_charging: false,
            wait_for_event: None,

            // Phase 4 defaults
            user_tag: None,
            parent_job_id: None,
            chain_group_id: None,
            result_summary: None,
            artifacts: None,
        }
    }

    /// Transition to Running state with explicit timestamp
    pub fn start(&mut self, now_millis: i64) -> crate::domain::error::Result<()> {
        if self.state != JobState::Queued {
            return Err(crate::domain::error::DomainError::InvalidStateTransition {
                from: self.state.to_string(),
                to: "RUNNING".to_string(),
            });
        }
        self.state = JobState::Running;
        self.started_at = Some(now_millis);
        Ok(())
    }

    /// Transition to Done state with explicit timestamp
    pub fn complete(&mut self, now_millis: i64) -> crate::domain::error::Result<()> {
        if self.state != JobState::Running {
            return Err(crate::domain::error::DomainError::InvalidStateTransition {
                from: self.state.to_string(),
                to: "DONE".to_string(),
            });
        }
        self.state = JobState::Done;
        self.finished_at = Some(now_millis);
        Ok(())
    }

    /// Mark as Superseded with explicit timestamp
    pub fn supersede(&mut self, now_millis: i64) {
        self.state = JobState::Superseded;
        self.finished_at = Some(now_millis);
    }

    /// Mark as Failed with explicit timestamp
    pub fn fail(&mut self, now_millis: i64) {
        self.state = JobState::Failed;
        self.finished_at = Some(now_millis);
    }
}
