// Queue Domain Model

/// Queue identifier
pub type QueueId = String;

/// Queue configuration (Phase 1: minimal)
#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub name: QueueId,
    pub max_workers: usize,
}

impl QueueConfig {
    pub fn new(name: impl Into<String>, max_workers: usize) -> Self {
        Self {
            name: name.into(),
            max_workers,
        }
    }
}
