// Central Error Type for the Application

use thiserror::Error;

/// Application-level error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Domain error: {0}")]
    Domain(#[from] crate::domain::DomainError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Execution error: {0}")]
    Execution(#[from] crate::port::ExecutionError),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias using AppError
pub type Result<T> = std::result::Result<T, AppError>;

// From implementations for infra crates (to avoid circular dependency)
impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError::Database(err)
    }
}

// Note: sqlx::Error conversion is handled in infra-sqlite crate
// by converting to AppError::Database(String)
