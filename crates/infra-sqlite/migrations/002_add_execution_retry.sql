-- Phase 2: Execution Engine & Retry (ADR-050)
-- Adds subprocess execution support, retry logic, and timeouts

-- Add execution and resilience fields to jobs table
ALTER TABLE jobs ADD COLUMN execution_mode TEXT CHECK(execution_mode IN ('IN_PROCESS', 'SUBPROCESS'));
ALTER TABLE jobs ADD COLUMN pid INTEGER;
ALTER TABLE jobs ADD COLUMN env_vars TEXT;  -- JSON

-- Add retry fields
ALTER TABLE jobs ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE jobs ADD COLUMN max_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE jobs ADD COLUMN backoff_factor REAL NOT NULL DEFAULT 2.0;

-- Add timeout fields
ALTER TABLE jobs ADD COLUMN deadline INTEGER;      -- Epoch ms (hard timeout)
ALTER TABLE jobs ADD COLUMN ttl_ms INTEGER;        -- Time-to-live in queue

-- Add tracing field
ALTER TABLE jobs ADD COLUMN trace_id TEXT;

-- Update schema version
INSERT INTO schema_version (version, applied_at)
VALUES (2, strftime('%s', 'now') * 1000);

