// System probe implementation (Phase 2)
// reason: sysinfo for cross-platform system monitoring (ADR-001)
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{Disks, System};
use tracing::debug;

use semantica_core::application::worker::constants::IDLE_TRACKER_MAX_SAMPLES;
use semantica_core::port::system_probe::{SystemMetrics, SystemProbe};

/// System probe implementation using sysinfo
///
/// Tracks CPU usage over time to detect idle state
pub struct SystemProbeImpl {
    system: Arc<Mutex<System>>,
    idle_tracker: Arc<Mutex<IdleTracker>>,
}

/// Tracks CPU usage history for idle detection
struct IdleTracker {
    samples: Vec<(Instant, f32)>,
    max_samples: usize,
}

impl IdleTracker {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::new(),
            max_samples,
        }
    }

    /// Record a CPU usage sample
    fn record(&mut self, cpu_usage: f32) {
        let now = Instant::now();
        self.samples.push((now, cpu_usage));

        // Keep only recent samples
        if self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }
    }

    /// Check if CPU has been below threshold for given duration
    fn is_idle(&self, threshold: f32, duration: Duration) -> bool {
        let cutoff = Instant::now() - duration;

        // Check all samples within the time window
        let recent_samples: Vec<&(Instant, f32)> = self
            .samples
            .iter()
            .filter(|(time, _)| *time >= cutoff)
            .collect();

        if recent_samples.is_empty() {
            return false;
        }

        // All samples must be below threshold
        recent_samples.iter().all(|(_, cpu)| *cpu < threshold)
    }
}

impl SystemProbeImpl {
    /// Create a new system probe
    ///
    /// # Example
    /// ```ignore
    /// let probe = SystemProbeImpl::new();
    /// ```
    pub fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new_all())),
            idle_tracker: Arc::new(Mutex::new(IdleTracker::new(IDLE_TRACKER_MAX_SAMPLES))),
        }
    }
}

impl Default for SystemProbeImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SystemProbe for SystemProbeImpl {
    async fn get_metrics(&self) -> SystemMetrics {
        let mut sys = self.system.lock().unwrap();

        // Refresh system information
        sys.refresh_all();

        // CPU usage (global average)
        let cpu_usage_percent = sys.global_cpu_info().cpu_usage();

        // Memory
        let memory_used_mb = sys.used_memory() / 1024 / 1024;
        let memory_total_mb = sys.total_memory() / 1024 / 1024;

        // Disk (first disk)
        let disks = Disks::new_with_refreshed_list();
        let (disk_used_gb, disk_total_gb) = if let Some(disk) = disks.first() {
            let total = disk.total_space() / 1024 / 1024 / 1024;
            let available = disk.available_space() / 1024 / 1024 / 1024;
            let used = total - available;
            (used, total)
        } else {
            (0, 0)
        };

        // Battery (not supported by sysinfo yet, placeholder)
        let battery_percent = None;
        let is_charging = None;

        // Record CPU sample for idle tracking
        let mut tracker = self.idle_tracker.lock().unwrap();
        tracker.record(cpu_usage_percent);

        debug!(
            cpu = %cpu_usage_percent,
            mem_used_mb = %memory_used_mb,
            mem_total_mb = %memory_total_mb,
            disk_used_gb = %disk_used_gb,
            "System metrics collected"
        );

        SystemMetrics {
            cpu_usage_percent,
            memory_used_mb,
            memory_total_mb,
            disk_used_gb,
            disk_total_gb,
            battery_percent,
            is_charging,
        }
    }

    async fn is_idle(&self, cpu_threshold: f32, duration_secs: u64) -> bool {
        // First, ensure we have a fresh metric
        self.get_metrics().await;

        let tracker = self.idle_tracker.lock().unwrap();
        let is_idle = tracker.is_idle(cpu_threshold, Duration::from_secs(duration_secs));

        debug!(
            threshold = %cpu_threshold,
            duration_secs = %duration_secs,
            is_idle = %is_idle,
            "Idle check completed"
        );

        is_idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_metrics() {
        let probe = SystemProbeImpl::new();
        let metrics = probe.get_metrics().await;

        // Basic sanity checks
        assert!(metrics.cpu_usage_percent >= 0.0);
        assert!(metrics.cpu_usage_percent <= 100.0);
        assert!(metrics.memory_total_mb > 0);
    }

    #[tokio::test]
    async fn test_idle_detection() {
        let probe = SystemProbeImpl::new();

        // Collect some samples
        for _ in 0..3 {
            probe.get_metrics().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Check idle status (threshold very high, should be idle)
        let _is_idle = probe.is_idle(99.0, 1).await;

        // We can't assert true/false since it depends on actual CPU usage
        // Just verify it doesn't panic
    }

    #[test]
    fn test_idle_tracker() {
        let mut tracker = IdleTracker::new(10);

        // Record low CPU usage samples
        for _ in 0..5 {
            tracker.record(5.0);
            std::thread::sleep(Duration::from_millis(10));
        }

        // Should be idle with threshold 10.0 and duration 1s
        assert!(tracker.is_idle(10.0, Duration::from_secs(1)));

        // Add high CPU sample
        tracker.record(95.0);

        // Should NOT be idle anymore
        assert!(!tracker.is_idle(10.0, Duration::from_secs(1)));
    }
}
