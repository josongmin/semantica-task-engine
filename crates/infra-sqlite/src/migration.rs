// Migration Runner

use sqlx::SqlitePool;
use tracing::info;

/// Run database migrations
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running database migrations...");

    // Check if schema_version table exists
    let table_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
    )
    .fetch_one(pool)
    .await?;

    let current_version: i64 = if table_exists > 0 {
        sqlx::query_scalar("SELECT version FROM schema_version ORDER BY version DESC LIMIT 1")
            .fetch_optional(pool)
            .await?
            .unwrap_or(0)
    } else {
        0
    };

    info!("Current schema version: {}", current_version);

    // Apply migrations sequentially
    if current_version < 1 {
        info!("Applying migration 001: Initial schema");
        apply_migration(pool, include_str!("../migrations/001_initial_schema.sql")).await?;
    }

    if current_version < 2 {
        info!("Applying migration 002: Execution & Retry");
        apply_migration(
            pool,
            include_str!("../migrations/002_add_execution_retry.sql"),
        )
        .await?;
    }

    if current_version < 3 {
        info!("Applying migration 003: Scheduling & Conditions");
        apply_migration(pool, include_str!("../migrations/003_add_scheduling.sql")).await?;
    }

    if current_version < 4 {
        info!("Applying migration 004: UX & Operational fields");
        apply_migration(pool, include_str!("../migrations/004_add_dx_fields.sql")).await?;
    }

    info!("All migrations applied successfully");
    Ok(())
}

/// Apply a single migration SQL file
async fn apply_migration(pool: &SqlitePool, sql: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Execute migration in a transaction
    let mut tx = pool.begin().await?;

    // Split by semicolon and execute each statement
    for statement in sql.split(';') {
        // Remove comments and trim
        let clean_statement: String = statement
            .lines()
            .filter(|line| !line.trim().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        if !clean_statement.is_empty() {
            sqlx::query(&clean_statement).execute(&mut *tx).await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_pool;

    #[tokio::test]
    async fn test_run_migrations() {
        let pool = create_pool("sqlite::memory:").await.unwrap();
        let result = run_migrations(&pool).await;

        if let Err(e) = &result {
            eprintln!("Migration error: {:?}", e);
        }
        assert!(result.is_ok());

        // Check that tables exist
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
