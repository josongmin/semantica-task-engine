// SQLite Connection Pool Setup

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use std::time::Duration;

/// Create SQLite connection pool with WAL mode and optimizations
///
/// # Configuration
/// - `SEMANTICA_POOL_SIZE`: Max connections (default: 20)
/// - `SEMANTICA_POOL_TIMEOUT`: Connection timeout in seconds (default: 5)
pub async fn create_pool(database_url: &str) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    // Read pool configuration from environment
    let max_connections: u32 = std::env::var("SEMANTICA_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20); // Default 20 (was 10)

    let busy_timeout_secs: u64 = std::env::var("SEMANTICA_POOL_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5); // Default 5 seconds

    let options = SqliteConnectOptions::from_str(database_url)?
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(busy_timeout_secs))
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await
        .map_err(|e| semantica_core::error::AppError::Database(e.to_string()))?;

    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .map_err(|e| semantica_core::error::AppError::Database(e.to_string()))?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_pool() {
        let pool = create_pool("sqlite::memory:").await.unwrap();
        assert!(pool.acquire().await.is_ok());
    }
}
