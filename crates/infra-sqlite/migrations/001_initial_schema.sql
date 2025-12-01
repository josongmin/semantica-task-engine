-- Phase 1: Initial Schema (ADR-010)
-- Minimal job queue with supersede support

-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

-- Jobs table (Phase 1 minimal fields)
CREATE TABLE IF NOT EXISTS jobs (
  id TEXT PRIMARY KEY,
  queue TEXT NOT NULL,
  job_type TEXT NOT NULL,
  subject_key TEXT NOT NULL,
  generation INTEGER NOT NULL,
  
  priority INTEGER NOT NULL DEFAULT 0,
  state TEXT NOT NULL CHECK(state IN (
    'QUEUED', 'SCHEDULED', 'RUNNING', 'DONE', 
    'FAILED', 'CANCELLED', 'SUPERSEDED', 
    'SKIPPED_TTL', 'SKIPPED_DEADLINE'
  )),
  created_at INTEGER NOT NULL,
  started_at INTEGER,
  finished_at INTEGER,
  
  payload TEXT NOT NULL,
  log_path TEXT
);

-- Indexes (ADR-010)
CREATE INDEX IF NOT EXISTS idx_jobs_pop
  ON jobs (queue, priority DESC, created_at ASC, id);

CREATE INDEX IF NOT EXISTS idx_jobs_state_queue
  ON jobs (state, queue);

CREATE INDEX IF NOT EXISTS idx_jobs_subject_generation
  ON jobs (subject_key, generation DESC);

CREATE INDEX IF NOT EXISTS idx_jobs_gc
  ON jobs (finished_at);

-- Subjects table (for tracking latest generation)
CREATE TABLE IF NOT EXISTS subjects (
  subject_key TEXT PRIMARY KEY,
  latest_generation INTEGER NOT NULL DEFAULT 0
);

-- Mark schema version
INSERT INTO schema_version (version, applied_at)
VALUES (1, strftime('%s', 'now') * 1000);

