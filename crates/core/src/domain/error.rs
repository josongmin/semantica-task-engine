// Domain Error Types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Invalid job state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Invalid priority: {0}")]
    InvalidPriority(i32),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, DomainError>;
