-- Rollback Phase 3: AI-Native Scheduling (ADR-050)
-- Removes scheduling and condition fields from jobs table

-- Drop indexes first
DROP INDEX IF EXISTS idx_jobs_conditions;
DROP INDEX IF EXISTS idx_jobs_schedule_at;

-- Remove Phase 3 columns from jobs table
ALTER TABLE jobs DROP COLUMN wait_for_event;
ALTER TABLE jobs DROP COLUMN require_charging;
ALTER TABLE jobs DROP COLUMN wait_for_idle;
ALTER TABLE jobs DROP COLUMN schedule_at;

