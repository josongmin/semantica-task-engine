//! JSON-RPC Server
//!
//! Implements the JSON-RPC 2.0 server over Unix Domain Socket (macOS/Linux).

use crate::handler::RpcHandler;
use crate::types::{
    CancelRequest, EnqueueRequest, MaintenanceRequest, StatsRequest, TailLogsRequest,
};
use jsonrpsee::server::{Server, ServerHandle};
use jsonrpsee::RpcModule;
use semantica_core::port::job_repository::JobRepository;
use semantica_core::port::{IdProvider, Maintenance, TimeProvider, TransactionalJobRepository};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

// ADR-020: RPC Server Configuration
// Note: jsonrpsee doesn't support Unix sockets directly (hyper limitation)
// Using TCP on localhost as secure alternative (no external access)
const DEFAULT_SOCKET_PATH: &str = "~/.semantica/semantica.sock";
const DEFAULT_RPC_HOST: &str = "127.0.0.1";
const DEFAULT_RPC_PORT: u16 = 9527;

/// RPC Server Configuration
pub struct RpcServerConfig {
    pub socket_path: PathBuf, // Reserved for future UDS support
    pub host: String,
    pub port: u16,
}

impl Default for RpcServerConfig {
    fn default() -> Self {
        Self {
            socket_path: shellexpand::tilde(DEFAULT_SOCKET_PATH).into_owned().into(),
            host: DEFAULT_RPC_HOST.to_string(),
            port: DEFAULT_RPC_PORT,
        }
    }
}

/// RPC Server
pub struct RpcServer {
    config: RpcServerConfig,
    handler: Arc<RpcHandler>,
}

impl RpcServer {
    pub fn new(
        config: RpcServerConfig,
        tx_job_repo: Arc<dyn TransactionalJobRepository>,
        job_repo: Arc<dyn JobRepository>,
        id_provider: Arc<dyn IdProvider>,
        time_provider: Arc<dyn TimeProvider>,
        maintenance: Arc<dyn Maintenance>,
    ) -> Self {
        Self {
            config,
            handler: Arc::new(RpcHandler::new(
                tx_job_repo,
                job_repo,
                id_provider,
                time_provider,
                maintenance,
            )),
        }
    }

    /// Start the JSON-RPC server
    ///
    /// Note: Uses TCP on localhost (not Unix socket) due to jsonrpsee/hyper limitations
    /// Security: Only binds to 127.0.0.1 (no external access)
    pub async fn start(self) -> Result<ServerHandle, String> {
        let addr = format!("{}:{}", self.config.host, self.config.port);

        info!(
            host = %self.config.host,
            port = %self.config.port,
            "Starting JSON-RPC server on TCP (localhost only)"
        );

        // Build server with localhost-only binding
        let server = Server::builder()
            .build(&addr)
            .await
            .map_err(|e| format!("Failed to build server on {}: {}", addr, e))?;

        let mut module = RpcModule::new(());

        // Register methods
        let handler = self.handler.clone();
        module
            .register_async_method("dev.enqueue.v1", move |params, _, _| {
                let handler = handler.clone();
                async move {
                    let req: EnqueueRequest = params.parse()?;
                    handler.enqueue(req).await
                }
            })
            .map_err(|e| e.to_string())?;

        let handler = self.handler.clone();
        module
            .register_async_method("dev.cancel.v1", move |params, _, _| {
                let handler = handler.clone();
                async move {
                    let req: CancelRequest = params.parse()?;
                    handler.cancel(req).await
                }
            })
            .map_err(|e| e.to_string())?;

        let handler = self.handler.clone();
        module
            .register_async_method("logs.tail.v1", move |params, _, _| {
                let handler = handler.clone();
                async move {
                    let req: TailLogsRequest = params.parse()?;
                    handler.tail_logs(req).await
                }
            })
            .map_err(|e| e.to_string())?;

        // Admin APIs (Phase 4)
        let handler = self.handler.clone();
        module
            .register_async_method("admin.stats.v1", move |params, _, _| {
                let handler = handler.clone();
                async move {
                    let req: StatsRequest = params.parse()?;
                    handler.stats(req).await
                }
            })
            .map_err(|e| e.to_string())?;

        let handler = self.handler.clone();
        module
            .register_async_method("admin.maintenance.v1", move |params, _, _| {
                let handler = handler.clone();
                async move {
                    let req: MaintenanceRequest = params.parse()?;
                    handler.maintenance(req).await
                }
            })
            .map_err(|e| e.to_string())?;

        info!("JSON-RPC server started successfully");

        let handle = server.start(module);
        Ok(handle)
    }
}
