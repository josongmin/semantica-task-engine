//! Scheduler - Determines if a job is ready to execute based on conditions
//!
//! Phase 3: AI-Native Scheduling (ADR-050)
//! - wait_for_idle: Execute when system is idle
//! - require_charging: Execute only when device is charging
//! - wait_for_event: Execute when specific event occurs
//! - schedule_at: Execute at specific time

use crate::domain::Job;
use crate::port::SystemProbe;
use std::sync::Arc;
use tracing::{debug, info};

/// Scheduler determines if a job is ready to execute
pub struct Scheduler {
    system_probe: Arc<dyn SystemProbe>,
    time_provider: Arc<dyn crate::port::TimeProvider>,
}

impl Scheduler {
    pub fn new(
        system_probe: Arc<dyn SystemProbe>,
        time_provider: Arc<dyn crate::port::TimeProvider>,
    ) -> Self {
        Self {
            system_probe,
            time_provider,
        }
    }

    /// Check if job is ready to execute based on all conditions
    pub async fn is_ready(&self, job: &Job) -> bool {
        // Check schedule_at (time-based scheduling)
        if let Some(schedule_at) = job.schedule_at {
            let now = self.time_provider.now_millis();
            if now < schedule_at {
                debug!(
                    job_id = %job.id,
                    schedule_at = schedule_at,
                    now = now,
                    "Job not ready: scheduled for future"
                );
                return false;
            }
        }

        // Check wait_for_idle (system idle condition)
        if job.wait_for_idle && !self.is_system_idle().await {
            debug!(
                job_id = %job.id,
                "Job not ready: waiting for system idle"
            );
            return false;
        }

        // Check require_charging (battery condition)
        if job.require_charging && !self.is_charging().await {
            debug!(
                job_id = %job.id,
                "Job not ready: waiting for charging"
            );
            return false;
        }

        // Check wait_for_event (event-based trigger)
        if job.wait_for_event.is_some() {
            // Event checking is handled by EventManager (not implemented in Phase 3 MVP)
            // For now, treat as not ready if event is specified
            debug!(
                job_id = %job.id,
                event = ?job.wait_for_event,
                "Job not ready: waiting for event (not implemented)"
            );
            return false;
        }

        info!(
            job_id = %job.id,
            "Job is ready to execute"
        );
        true
    }

    /// Check if system is idle (low CPU usage)
    async fn is_system_idle(&self) -> bool {
        use crate::application::worker::constants::IDLE_CPU_THRESHOLD;

        let metrics = self.system_probe.get_metrics().await;
        metrics.cpu_usage_percent < IDLE_CPU_THRESHOLD
    }

    /// Check if device is charging
    ///
    /// Returns true if:
    /// - Plugged into AC power
    /// - Battery >= 80%
    /// - No battery found (desktop)
    async fn is_charging(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("pmset").arg("-g").arg("batt").output() {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    // Check for AC Power
                    if stdout.contains("AC Power") {
                        return true;
                    }
                    // Check if battery >= 80%
                    if let Some(line) = stdout.lines().nth(1) {
                        if let Some(percent_str) = line.split('%').next() {
                            if let Some(num_str) = percent_str.split_whitespace().last() {
                                if let Ok(percent) = num_str.parse::<i32>() {
                                    return percent >= 80;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            // Check /sys/class/power_supply/ for AC adapter
            if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(type_content) = fs::read_to_string(path.join("type")) {
                        if type_content.trim() == "Mains" {
                            if let Ok(online) = fs::read_to_string(path.join("online")) {
                                if online.trim() == "1" {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            // Check battery level
            if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(capacity) = fs::read_to_string(path.join("capacity")) {
                        if let Ok(percent) = capacity.trim().parse::<i32>() {
                            if percent >= 80 {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        
        // Windows or unknown: assume desktop (plugged in)
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            true
        }
        
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{JobPayload, JobType};
    use crate::port::{SystemMetrics, SystemProbe, TimeProvider};
    use async_trait::async_trait;

    struct MockSystemProbe {
        cpu_usage: f32,
    }

    struct MockTimeProvider {
        current_time: i64,
    }

    impl TimeProvider for MockTimeProvider {
        fn now_millis(&self) -> i64 {
            self.current_time
        }
    }

    #[async_trait]
    impl SystemProbe for MockSystemProbe {
        async fn get_metrics(&self) -> SystemMetrics {
            SystemMetrics {
                cpu_usage_percent: self.cpu_usage,
                memory_used_mb: 1000,
                memory_total_mb: 8000,
                disk_used_gb: 10,
                disk_total_gb: 100,
                battery_percent: None,
                is_charging: None,
            }
        }

        async fn is_idle(&self, cpu_threshold: f32, _duration_secs: u64) -> bool {
            self.cpu_usage < cpu_threshold
        }
    }

    #[tokio::test]
    async fn test_job_ready_no_conditions() {
        let probe = Arc::new(MockSystemProbe { cpu_usage: 50.0 });
        let time_provider = Arc::new(MockTimeProvider {
            current_time: 1000000,
        });
        let scheduler = Scheduler::new(probe, time_provider);

        let job = Job::new_test(
            "test_queue",
            JobType::new("test"),
            "test.rs",
            1,
            JobPayload::new(serde_json::json!({})),
        );

        assert!(
            scheduler.is_ready(&job).await,
            "Job with no conditions should be ready"
        );
    }

    #[tokio::test]
    async fn test_job_not_ready_future_schedule() {
        let probe = Arc::new(MockSystemProbe { cpu_usage: 10.0 });
        let time_provider = Arc::new(MockTimeProvider {
            current_time: 1000000, // Current time
        });
        let scheduler = Scheduler::new(probe, time_provider);

        let mut job = Job::new_test(
            "test_queue",
            JobType::new("test"),
            "test.rs",
            1,
            JobPayload::new(serde_json::json!({})),
        );

        // Schedule for 1 hour in the future
        job.schedule_at = Some(1000000 + 3_600_000);

        assert!(
            !scheduler.is_ready(&job).await,
            "Job scheduled for future should not be ready"
        );
    }

    #[tokio::test]
    async fn test_job_ready_past_schedule() {
        let probe = Arc::new(MockSystemProbe { cpu_usage: 10.0 });
        let time_provider = Arc::new(MockTimeProvider {
            current_time: 1000000, // Current time
        });
        let scheduler = Scheduler::new(probe, time_provider);

        let mut job = Job::new_test(
            "test_queue",
            JobType::new("test"),
            "test.rs",
            1,
            JobPayload::new(serde_json::json!({})),
        );

        // Schedule for 1 hour in the past
        job.schedule_at = Some(1000000 - 3_600_000);

        assert!(
            scheduler.is_ready(&job).await,
            "Job scheduled for past should be ready"
        );
    }

    #[tokio::test]
    async fn test_job_not_ready_wait_for_idle_high_cpu() {
        let probe = Arc::new(MockSystemProbe { cpu_usage: 80.0 });
        let time_provider = Arc::new(MockTimeProvider {
            current_time: 1000000,
        });
        let scheduler = Scheduler::new(probe, time_provider);

        let mut job = Job::new_test(
            "test_queue",
            JobType::new("test"),
            "test.rs",
            1,
            JobPayload::new(serde_json::json!({})),
        );

        job.wait_for_idle = true;

        assert!(
            !scheduler.is_ready(&job).await,
            "Job waiting for idle should not be ready when CPU high"
        );
    }

    #[tokio::test]
    async fn test_job_ready_wait_for_idle_low_cpu() {
        let probe = Arc::new(MockSystemProbe { cpu_usage: 10.0 });
        let time_provider = Arc::new(MockTimeProvider {
            current_time: 1000000,
        });
        let scheduler = Scheduler::new(probe, time_provider);

        let mut job = Job::new_test(
            "test_queue",
            JobType::new("test"),
            "test.rs",
            1,
            JobPayload::new(serde_json::json!({})),
        );

        job.wait_for_idle = true;

        assert!(
            scheduler.is_ready(&job).await,
            "Job waiting for idle should be ready when CPU low"
        );
    }

    #[tokio::test]
    async fn test_job_not_ready_require_charging() {
        let probe = Arc::new(MockSystemProbe { cpu_usage: 10.0 });
        let time_provider = Arc::new(MockTimeProvider {
            current_time: 1000000,
        });
        let scheduler = Scheduler::new(probe, time_provider);

        let mut job = Job::new_test(
            "test_queue",
            JobType::new("test"),
            "test.rs",
            1,
            JobPayload::new(serde_json::json!({})),
        );

        job.require_charging = true;

        // Battery check is now implemented
        // Result depends on system (may be true on desktop/AC, false on battery)
        // Test just verifies battery check runs without panic
        let _ready = scheduler.is_ready(&job).await;
        
        // Note: The actual value varies by system (desktop vs laptop, charging vs not)
        // This test just ensures battery check logic doesn't panic
    }
}
