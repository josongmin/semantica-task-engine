-- Rollback Phase 4: Reliability & Ops (ADR-050)
-- Removes user experience and operational fields

-- Drop indexes first
DROP INDEX IF EXISTS idx_jobs_parent;
DROP INDEX IF EXISTS idx_jobs_chain_group;
DROP INDEX IF EXISTS idx_jobs_user_tag;

-- Remove Phase 4 columns
ALTER TABLE jobs DROP COLUMN artifacts;
ALTER TABLE jobs DROP COLUMN result_summary;
ALTER TABLE jobs DROP COLUMN chain_group_id;
ALTER TABLE jobs DROP COLUMN parent_job_id;
ALTER TABLE jobs DROP COLUMN user_tag;

