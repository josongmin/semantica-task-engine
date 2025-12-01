-- Phase 3: AI-Native Scheduling (ADR-050)
-- Adds scheduling and condition fields to jobs table

-- Add scheduling fields to jobs table
ALTER TABLE jobs ADD COLUMN schedule_at INTEGER;  -- Unix timestamp (ms) when job should run
ALTER TABLE jobs ADD COLUMN wait_for_idle BOOLEAN DEFAULT 0;  -- Wait for system idle
ALTER TABLE jobs ADD COLUMN require_charging BOOLEAN DEFAULT 0;  -- Require device charging
ALTER TABLE jobs ADD COLUMN wait_for_event TEXT;  -- Event name to wait for (e.g., "pr_merged")

-- Create index for scheduled jobs
CREATE INDEX IF NOT EXISTS idx_jobs_schedule_at ON jobs(schedule_at) WHERE schedule_at IS NOT NULL;

-- Create index for conditional jobs
CREATE INDEX IF NOT EXISTS idx_jobs_conditions ON jobs(wait_for_idle, require_charging, wait_for_event) 
WHERE wait_for_idle = 1 OR require_charging = 1 OR wait_for_event IS NOT NULL;

