// Worker constants (ADR: No magic values)
use std::time::Duration;

/// Sleep duration when no jobs are available (100ms)
pub const IDLE_SLEEP_DURATION: Duration = Duration::from_millis(100);

/// Sleep duration after worker error before retry (1s)
pub const ERROR_RECOVERY_SLEEP_DURATION: Duration = Duration::from_secs(1);

/// Mock execution duration for testing (10ms)
/// Note: Real execution uses IN_PROCESS or SUBPROCESS mode (Phase 2)
pub const MOCK_EXECUTION_DURATION: Duration = Duration::from_millis(10);

/// Default retry base delay (1000ms = 1s)
pub const DEFAULT_RETRY_BASE_DELAY_MS: i64 = 1000;

/// Default recovery window for orphaned jobs (5 minutes)
pub const DEFAULT_RECOVERY_WINDOW_MS: i64 = 5 * 60 * 1000;

/// CPU usage threshold for throttling (percent)
/// When CPU usage exceeds this, low-priority queues are paused (ADR-002)
pub const CPU_THROTTLE_THRESHOLD: f32 = 90.0;

/// Graceful process shutdown timeout (5 seconds)
/// From SubprocessExecutor (Phase 2 - ADR-002)
pub const GRACEFUL_SHUTDOWN_TIMEOUT_MS: i64 = 5000;

/// Idle CPU threshold for wait_for_idle condition (30%)
/// From Scheduler (Phase 3 - ADR-050)
pub const IDLE_CPU_THRESHOLD: f32 = 30.0;

/// Maximum samples for idle detection tracking (60 samples = 1 minute at 1 sample/sec)
/// From SystemProbeImpl (Phase 2)
pub const IDLE_TRACKER_MAX_SAMPLES: usize = 60;
