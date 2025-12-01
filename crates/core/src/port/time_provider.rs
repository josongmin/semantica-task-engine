// Time Provider Port (for testability)

/// Time provider interface (allows mocking in tests)
pub trait TimeProvider: Send + Sync {
    /// Get current time in milliseconds since epoch
    fn now_millis(&self) -> i64;
}

/// System time provider (production)
pub struct SystemTimeProvider;

impl TimeProvider for SystemTimeProvider {
    fn now_millis(&self) -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
}
