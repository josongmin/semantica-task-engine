// Port Layer - Interfaces for external dependencies

pub mod id_provider; // For deterministic testing
pub mod job_repository;
pub mod maintenance;
pub mod system_probe;
pub mod task_executor; // Phase 2
pub mod time_provider;
pub mod transaction; // Phase 2 // Phase 4

// Re-exports
pub use id_provider::IdProvider;
pub use job_repository::JobRepository;
pub use maintenance::{Maintenance, MaintenanceConfig, MaintenanceStats};
pub use system_probe::{SystemMetrics, SystemProbe};
pub use task_executor::{ExecutionError, ExecutionResult, ExecutionStatus, TaskExecutor};
pub use time_provider::TimeProvider;
pub use transaction::{JobRepositoryTransaction, Transaction, TransactionalJobRepository};
