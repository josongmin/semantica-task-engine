// Semantica Core - Domain Logic & Ports
// NO infrastructure dependencies (ADR-001: Hexagonal Architecture)

pub mod application;
pub mod domain;
pub mod error;
pub mod port;

pub use error::{AppError, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
