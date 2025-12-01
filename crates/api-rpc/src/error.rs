//! RPC Error Types
//!
//! Maps application errors to JSON-RPC error codes (ADR-020).

use jsonrpsee::types::ErrorObjectOwned;
use semantica_core::error::AppError;

/// RPC Error Codes (ADR-020)
pub mod code {
    pub const VALIDATION_ERROR: i32 = 4000;
    pub const NOT_FOUND: i32 = 4001;
    pub const CONFLICT: i32 = 4002;
    pub const THROTTLED: i32 = 4003;
    pub const INTERNAL_ERROR: i32 = 5000;
    pub const DB_ERROR: i32 = 5001;
    pub const SYSTEM_ERROR: i32 = 5002;
}

/// Convert AppError to JSON-RPC ErrorObject
pub fn to_rpc_error(err: AppError) -> ErrorObjectOwned {
    match err {
        AppError::Validation(msg) => {
            ErrorObjectOwned::owned(code::VALIDATION_ERROR, msg, None::<()>)
        }
        AppError::NotFound(msg) => ErrorObjectOwned::owned(code::NOT_FOUND, msg, None::<()>),
        AppError::Conflict(msg) => ErrorObjectOwned::owned(code::CONFLICT, msg, None::<()>),
        AppError::Database(msg) => ErrorObjectOwned::owned(code::DB_ERROR, msg, None::<()>),
        AppError::Execution(e) => {
            ErrorObjectOwned::owned(code::SYSTEM_ERROR, e.to_string(), None::<()>)
        }
        AppError::Internal(msg) => ErrorObjectOwned::owned(code::INTERNAL_ERROR, msg, None::<()>),
        AppError::Domain(e) => {
            ErrorObjectOwned::owned(code::VALIDATION_ERROR, e.to_string(), None::<()>)
        }
        AppError::Io(e) => ErrorObjectOwned::owned(code::SYSTEM_ERROR, e.to_string(), None::<()>),
        AppError::Serialization(e) => {
            ErrorObjectOwned::owned(code::VALIDATION_ERROR, e.to_string(), None::<()>)
        }
        AppError::Config(msg) => ErrorObjectOwned::owned(code::INTERNAL_ERROR, msg, None::<()>),
        AppError::InvalidState(msg) => ErrorObjectOwned::owned(code::CONFLICT, msg, None::<()>),
    }
}
