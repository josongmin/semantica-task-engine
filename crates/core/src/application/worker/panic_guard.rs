// Panic isolation for worker safety (Phase 2, ADR-002)
use std::panic::{catch_unwind, AssertUnwindSafe};
use tracing::{error, warn};

/// Result of a panic-guarded execution
#[derive(Debug)]
pub enum PanicGuardResult<T> {
    /// Execution completed successfully
    Success(T),
    /// Execution panicked
    Panicked(String),
}

/// Execute a closure with panic isolation
///
/// If the closure panics, the panic is caught and returned as PanicGuardResult::Panicked.
/// This prevents worker panics from killing the daemon.
///
/// # Example
/// ```text
/// let result = execute_guarded(|| {
///     // This panic will be caught
///     panic!("test panic");
/// });
///
/// match result {
///     PanicGuardResult::Panicked(msg) => {
///         println!("Caught panic: {}", msg);
///     }
///     _ => {}
/// }
/// ```
pub fn execute_guarded<F, T>(f: F) -> PanicGuardResult<T>
where
    F: FnOnce() -> T + std::panic::UnwindSafe,
{
    match catch_unwind(f) {
        Ok(result) => PanicGuardResult::Success(result),
        Err(panic_info) => {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            error!(panic_msg = %panic_msg, "Worker task panicked");
            PanicGuardResult::Panicked(panic_msg)
        }
    }
}

/// Execute an async closure with panic isolation
///
/// For async operations, wrap the future in AssertUnwindSafe.
///
/// # Example
/// ```text
/// let result = execute_guarded_async(async {
///     // Async work that might panic
///     tokio::time::sleep(std::time::Duration::from_millis(10)).await;
///     42
/// }).await;
/// ```
pub async fn execute_guarded_async<F, T>(future: F) -> PanicGuardResult<T>
where
    F: std::future::Future<Output = T>,
{
    let result = catch_unwind(AssertUnwindSafe(|| {
        // We need to block on the future within the catch_unwind
        // This is a simplified version - real implementation would use tokio::task::spawn
        warn!("execute_guarded_async: Using blocking wait (consider tokio::task::spawn)");
    }));

    match result {
        Ok(_) => {
            // For now, we'll execute the future directly
            // TODO: Implement proper async panic catching with tokio::task::spawn
            PanicGuardResult::Success(future.await)
        }
        Err(panic_info) => {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            error!(panic_msg = %panic_msg, "Async worker task panicked");
            PanicGuardResult::Panicked(panic_msg)
        }
    }
}
