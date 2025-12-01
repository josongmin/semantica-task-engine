//! SDK Error Types

use thiserror::Error;

/// SDK Result type
pub type Result<T> = std::result::Result<T, SdkError>;

/// SDK Error
#[derive(Debug, Error)]
pub enum SdkError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("RPC error ({code}): {message}")]
    Rpc { code: i32, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<jsonrpsee::core::ClientError> for SdkError {
    fn from(e: jsonrpsee::core::ClientError) -> Self {
        match e {
            jsonrpsee::core::ClientError::Call(call_err) => SdkError::Rpc {
                code: call_err.code(),
                message: call_err.message().to_string(),
            },
            jsonrpsee::core::ClientError::Transport(e) => {
                SdkError::Transport(format!("Transport error: {}", e))
            }
            jsonrpsee::core::ClientError::RestartNeeded(_) => {
                SdkError::Connection("Connection restart needed".to_string())
            }
            jsonrpsee::core::ClientError::ParseError(e) => {
                SdkError::Other(format!("Parse error: {}", e))
            }
            _ => SdkError::Other(e.to_string()),
        }
    }
}
