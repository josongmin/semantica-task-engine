// Transaction port for atomic operations

use crate::error::Result;
use async_trait::async_trait;

/// Transaction trait for atomic multi-step operations
#[async_trait]
pub trait Transaction: Send {
    /// Commit the transaction
    async fn commit(self: Box<Self>) -> Result<()>;

    /// Rollback the transaction
    async fn rollback(self: Box<Self>) -> Result<()>;
}

/// Transactional JobRepository operations
#[async_trait]
pub trait TransactionalJobRepository: Send + Sync {
    /// Begin a new transaction
    async fn begin_transaction(&self) -> Result<Box<dyn JobRepositoryTransaction>>;
}

/// JobRepository operations within a transaction
#[async_trait]
pub trait JobRepositoryTransaction: Transaction {
    /// Get latest generation (within transaction)
    async fn get_latest_generation(&mut self, subject_key: &str) -> Result<i64>;

    /// Insert job (within transaction)
    async fn insert(&mut self, job: &crate::domain::Job) -> Result<()>;

    /// Mark superseded (within transaction)
    async fn mark_superseded(&mut self, subject_key: &str, below_generation: i64) -> Result<u64>;
}
