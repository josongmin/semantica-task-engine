// Maintenance Service (Phase 4 - ADR-050)
// Scheduled maintenance operations for DB and artifacts

use crate::error::Result;
use crate::port::{Maintenance, MaintenanceConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info};

/// Maintenance scheduler
///
/// Runs periodic maintenance operations (VACUUM, GC) in the background
pub struct MaintenanceScheduler {
    maintenance: Arc<dyn Maintenance>,
    config: MaintenanceConfig,
    interval_hours: u64,
}

impl MaintenanceScheduler {
    /// Create a new maintenance scheduler
    ///
    /// # Arguments
    /// * `maintenance` - Maintenance implementation
    /// * `config` - Maintenance configuration
    /// * `interval_hours` - How often to run maintenance (hours)
    pub fn new(
        maintenance: Arc<dyn Maintenance>,
        config: MaintenanceConfig,
        interval_hours: u64,
    ) -> Self {
        Self {
            maintenance,
            config,
            interval_hours,
        }
    }

    /// Run maintenance loop (background task)
    ///
    /// Runs full maintenance every interval_hours
    /// Should be spawned in tokio::spawn
    pub async fn run(self) {
        info!(
            interval_hours = self.interval_hours,
            retention_days = self.config.finished_job_retention_days,
            "Maintenance scheduler started"
        );

        let mut tick = interval(Duration::from_secs(self.interval_hours * 3600));

        loop {
            tick.tick().await;

            info!("Running scheduled maintenance...");

            match self.maintenance.run_full_maintenance(&self.config).await {
                Ok(stats) => {
                    info!(
                        db_size_mb = stats.db_size_mb,
                        job_count = stats.job_count,
                        finished_jobs = stats.finished_job_count,
                        artifacts = stats.artifact_count,
                        artifact_size_mb = stats.log_files_size_mb,
                        "Scheduled maintenance completed successfully"
                    );
                }
                Err(e) => {
                    error!(error = ?e, "Scheduled maintenance failed");
                }
            }
        }
    }

    /// Run maintenance immediately (for manual trigger)
    pub async fn run_now(&self) -> Result<()> {
        info!("Running manual maintenance...");

        let stats = self.maintenance.run_full_maintenance(&self.config).await?;

        info!(
            db_size_mb = stats.db_size_mb,
            job_count = stats.job_count,
            "Manual maintenance completed"
        );

        Ok(())
    }
}
