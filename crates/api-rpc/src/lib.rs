//! JSON-RPC API Layer
//!
//! Implements the JSON-RPC 2.0 server for Semantica Task Engine.
//! Adheres to ADR-020 (API Contract).

pub mod error;
pub mod handler;
mod rate_limiter;
pub mod server;
pub mod types;

pub use server::RpcServer;
