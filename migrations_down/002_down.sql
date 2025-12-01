-- Rollback Phase 2 migration
-- WARNING: This will lose data in Phase 2 columns

-- Remove Phase 2 columns (SQLite doesn't support DROP COLUMN directly in older versions)
-- For production, this would require creating a new table and migrating data

-- For development/testing, we can use a newer SQLite with DROP COLUMN support
ALTER TABLE jobs DROP COLUMN trace_id;
ALTER TABLE jobs DROP COLUMN ttl_ms;
ALTER TABLE jobs DROP COLUMN deadline;
ALTER TABLE jobs DROP COLUMN backoff_factor;
ALTER TABLE jobs DROP COLUMN max_attempts;
ALTER TABLE jobs DROP COLUMN attempts;
ALTER TABLE jobs DROP COLUMN env_vars;
ALTER TABLE jobs DROP COLUMN pid;
ALTER TABLE jobs DROP COLUMN execution_mode;

-- Remove schema version entry
DELETE FROM schema_version WHERE version = 2;

