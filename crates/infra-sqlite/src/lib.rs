// Semantica Infrastructure - SQLite Adapter
// Implements: JobRepository, TransactionalJobRepository (ADR-010), Maintenance (Phase 4)

mod connection;
mod job_repository;
mod maintenance_impl;
mod migration;
mod transaction; // Phase 4

pub use connection::create_pool;
pub use job_repository::SqliteJobRepository;
pub use maintenance_impl::SqliteMaintenance;
pub use migration::run_migrations;
pub use transaction::SqliteJobTransaction; // Phase 4

// Note: sqlx::Error conversion is handled by wrapping in helper functions
// due to Rust's orphan rules (cannot implement From<sqlx::Error> for AppError here)
