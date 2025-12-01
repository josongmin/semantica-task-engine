// System resource monitoring port (Phase 2)
// reason: async-trait 필요 (ADR-001)
use async_trait::async_trait;

/// System resource metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_used_gb: u64,
    pub disk_total_gb: u64,
    pub battery_percent: Option<f32>, // None if no battery
    pub is_charging: Option<bool>,    // None if no battery
}

/// System probe port for resource monitoring
///
/// Used for throttling decisions (ADR-002)
#[async_trait]
pub trait SystemProbe: Send + Sync {
    /// Get current system metrics
    ///
    /// # Returns
    /// SystemMetrics with CPU, memory, disk, and battery info
    ///
    /// # Example
    /// ```text
    /// let metrics = probe.get_metrics().await;
    /// if metrics.cpu_usage_percent > 90.0 {
    ///     println!("CPU throttling triggered");
    /// }
    /// ```
    async fn get_metrics(&self) -> SystemMetrics;

    /// Check if system is idle (CPU < threshold for N seconds)
    ///
    /// # Arguments
    /// * `cpu_threshold` - CPU usage threshold (0.0 - 100.0)
    /// * `duration_secs` - How long CPU must be below threshold
    ///
    /// # Returns
    /// true if system has been idle for the specified duration
    async fn is_idle(&self, cpu_threshold: f32, duration_secs: u64) -> bool;
}

// ============================================================================
// Mock Implementations for Testing
// ============================================================================

pub mod mocks {
    use super::*;
    use std::sync::{Arc, Mutex};
    /// Mock SystemProbe for testing
    pub struct MockSystemProbe {
        metrics: Arc<Mutex<SystemMetrics>>,
    }
    impl MockSystemProbe {
        pub fn new(cpu_usage_percent: f32) -> Self {
            Self {
                metrics: Arc::new(Mutex::new(SystemMetrics {
                    cpu_usage_percent,
                    memory_used_mb: 1024,
                    memory_total_mb: 2048,
                    disk_used_gb: 100,
                    disk_total_gb: 500,
                    battery_percent: None,
                    is_charging: None,
                })),
            }
        }
        pub fn set_cpu_usage(&self, cpu_usage_percent: f32) {
            self.metrics.lock().unwrap().cpu_usage_percent = cpu_usage_percent;
        }
    }
    #[async_trait]
    impl SystemProbe for MockSystemProbe {
        async fn get_metrics(&self) -> SystemMetrics {
            self.metrics.lock().unwrap().clone()
        }
        async fn is_idle(&self, _cpu_threshold: f32, _duration_secs: u64) -> bool {
            // For testing, always return false (not idle)
            false
        }
    }
}
