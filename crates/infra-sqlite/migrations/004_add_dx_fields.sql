-- Phase 4: Reliability & Ops (ADR-050)
-- Adds user experience and operational fields

-- Add UX/Grouping fields
ALTER TABLE jobs ADD COLUMN user_tag TEXT;  -- User-defined tag for filtering
ALTER TABLE jobs ADD COLUMN parent_job_id TEXT;  -- Parent job ID for chains
ALTER TABLE jobs ADD COLUMN chain_group_id TEXT;  -- Chain/batch group identifier

-- Add operational fields
ALTER TABLE jobs ADD COLUMN result_summary TEXT;  -- JSON result summary
ALTER TABLE jobs ADD COLUMN artifacts TEXT;  -- Comma-separated artifact paths

-- Create indexes for user experience
CREATE INDEX IF NOT EXISTS idx_jobs_user_tag ON jobs(user_tag) WHERE user_tag IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_jobs_chain_group ON jobs(chain_group_id) WHERE chain_group_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_jobs_parent ON jobs(parent_job_id) WHERE parent_job_id IS NOT NULL;

