// Dev Task Service - Core use cases for job management

pub mod enqueue;

pub use enqueue::EnqueueRequest;

use crate::error::Result;
use crate::port::{IdProvider, TimeProvider, TransactionalJobRepository};
use std::sync::Arc;

/// Dev Task Service (Phase 1)
pub struct DevTaskService {
    job_repo: Arc<dyn TransactionalJobRepository>,
    id_provider: Arc<dyn IdProvider>,
    time_provider: Arc<dyn TimeProvider>,
}

impl DevTaskService {
    pub fn new(
        job_repo: Arc<dyn TransactionalJobRepository>,
        id_provider: Arc<dyn IdProvider>,
        time_provider: Arc<dyn TimeProvider>,
    ) -> Self {
        Self {
            job_repo,
            id_provider,
            time_provider,
        }
    }

    /// Enqueue a new job
    pub async fn enqueue(&self, req: EnqueueRequest) -> Result<String> {
        enqueue::execute(
            self.job_repo.as_ref(),
            self.id_provider.as_ref(),
            self.time_provider.as_ref(),
            req,
        )
        .await
    }
}
