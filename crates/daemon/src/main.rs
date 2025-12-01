//! Semantica Task Engine - Main Entry Point
//! Phase 1: MVP with JSON-RPC Server + Worker

mod telemetry;

use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// Import workspace crates
use semantica_api_rpc::{server::RpcServerConfig, RpcServer};
use semantica_core::application::recovery::RecoveryService;
use semantica_core::application::retry::RetryPolicy;
use semantica_core::application::worker::{shutdown_channel, Worker};
use semantica_core::application::MaintenanceScheduler; // Phase 4
use semantica_core::port::id_provider::UuidProvider;
use semantica_core::port::time_provider::SystemTimeProvider;
use semantica_core::port::MaintenanceConfig; // Phase 4
use semantica_infra_sqlite::{create_pool, run_migrations, SqliteJobRepository, SqliteMaintenance}; // Phase 4
use semantica_infra_system::{SubprocessExecutor, SystemProbeImpl};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_DB_PATH: &str = "~/.semantica/meta.db";
const DEFAULT_QUEUE: &str = "default";

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize logging (JSON format for Phase 4 - ADR-050)
    let log_format = std::env::var("SEMANTICA_LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("semantica=info"))
        .expect("Failed to create env filter");

    match log_format.as_str() {
        "json" => {
            // Production: JSON structured logging
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().json())
                .init();
        }
        _ => {
            // Development: Pretty formatting with colors
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().pretty())
                .init();
        }
    }

    info!("Semantica Task Engine v{} starting...", VERSION);

    // 1.1. Initialize OpenTelemetry (optional, Phase 4)
    if let Err(e) = telemetry::init_telemetry() {
        tracing::warn!(error = ?e, "Failed to initialize OpenTelemetry (continuing without it)");
    }

    // 2. Load configuration
    let db_path = std::env::var("SEMANTICA_DB_PATH")
        .unwrap_or_else(|_| shellexpand::tilde(DEFAULT_DB_PATH).into_owned());

    let rpc_port: u16 = std::env::var("SEMANTICA_RPC_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9527);

    info!(db_path = %db_path, "Initializing database...");

    // 3. Initialize database
    let pool = create_pool(&db_path)
        .await
        .map_err(|e| anyhow::anyhow!("DB pool creation failed: {}", e))?;
    run_migrations(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Migration failed: {}", e))?;

    // 4. Setup dependencies (DI wiring)
    let time_provider = Arc::new(SystemTimeProvider);
    let id_provider = Arc::new(UuidProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(
        pool.clone(),
        time_provider.clone(),
    ));
    let tx_job_repo = Arc::new(SqliteJobRepository::new(
        pool.clone(),
        time_provider.clone(),
    ));

    let task_executor = Arc::new(SubprocessExecutor::new(
        time_provider.clone(),
        vec!["PATH".to_string(), "HOME".to_string(), "USER".to_string()],
    ));

    let system_probe = Arc::new(SystemProbeImpl::new());
    let retry_policy = Arc::new(RetryPolicy::new(time_provider.clone(), 1000));

    // Phase 3: Create Scheduler
    let scheduler = Arc::new(semantica_core::application::scheduler::Scheduler::new(
        system_probe.clone(),
        time_provider.clone(),
    ));

    // 5. Run crash recovery (Phase 2)
    info!("Running crash recovery...");
    let recovery_service = RecoveryService::new(
        job_repo.clone(),
        task_executor.clone(),
        time_provider.clone(),
        None, // Use default recovery window
    );

    match recovery_service.recover_orphaned_jobs().await {
        Ok(count) => info!(recovered_jobs = count, "Crash recovery completed"),
        Err(e) => tracing::error!(error = ?e, "Crash recovery failed"),
    }

    // 6. Initialize maintenance service (needed for RPC server)
    let maintenance = Arc::new(SqliteMaintenance::new(pool.clone(), time_provider.clone()));

    // 7. Start JSON-RPC server
    info!("Starting JSON-RPC server...");
    let rpc_config = RpcServerConfig {
        port: rpc_port,
        ..Default::default()
    };
    let rpc_server = RpcServer::new(
        rpc_config,
        tx_job_repo,
        job_repo.clone(),
        id_provider.clone(),
        time_provider.clone(),
        maintenance.clone(),
    );
    let rpc_handle = rpc_server
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("RPC server start failed: {}", e))?;

    // 7. Start Worker (job processing loop)
    info!("Starting worker...");
    let (shutdown_tx, shutdown_rx) = shutdown_channel();

    let worker = Worker::new(
        DEFAULT_QUEUE,
        job_repo.clone(),
        task_executor,
        system_probe,
        retry_policy,
        scheduler, // Phase 3
        time_provider.clone(),
    );

    let worker_handle = tokio::spawn(async move {
        if let Err(e) = worker.run(shutdown_rx).await {
            tracing::error!(error = ?e, "Worker failed");
        }
    });

    // 8. Start Maintenance Scheduler (Phase 4)
    info!("Starting maintenance scheduler...");
    let maintenance_config = MaintenanceConfig::default(); // 7 days retention
    let maintenance_scheduler = MaintenanceScheduler::new(
        maintenance,
        maintenance_config,
        24, // Run every 24 hours
    );

    tokio::spawn(async move {
        maintenance_scheduler.run().await;
    });

    info!("✅ System ready. Waiting for tasks...");
    info!("Press Ctrl+C to shutdown");

    // 9. Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    info!("Shutdown signal received. Exiting gracefully...");

    // 10. Graceful shutdown
    shutdown_tx.shutdown();
    rpc_handle
        .stop()
        .map_err(|e| anyhow::anyhow!("RPC server stop failed: {}", e))?;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), worker_handle).await;

    info!("Shutdown complete.");

    Ok(())
}
