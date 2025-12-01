네, 요청하신 ADR-010: Database & Persistence (The Schema SSOT) 문서를 작성해 드립니다.

이 문서는 기존의 분산되어 있던 스키마 정의(ADR-007, 008, 001 등)를 모두 통합하여, 데이터베이스의 유일한 진실 공급원(Single Source of Truth) 역할을 합니다. 특히 Phase별 스키마 진화 계획과 성능을 위한 인덱싱/제약조건이 완벽하게 결합되었습니다.

ADR-010: Database & Persistence (The Schema SSOT)
0. Status
Accepted

1. Context
The Semantica Orchestrator relies on a local SQLite database (meta.db) to manage job state, scheduling, and history. As the system evolves through Phases 1–4, the schema must handle:

High Throughput: Frequent polling by Workers (pop) and Planners (supersede).

Data Integrity: Strict enforcement of Enums and Foreign Keys to prevent "zombie" states.

Phase-based Evolution: Controlled addition of columns without breaking existing versions.

This document is the Single Source of Truth (SSOT) for the DB Schema. Any schema definition found in other ADRs is purely illustrative.

2. Decision: Database Configuration
We utilize SQLite in WAL (Write-Ahead Logging) mode to enable high concurrency.

Journal Mode: WAL (Allows 1 Writer + N Readers simultaneously).

Synchronous: NORMAL (Safe enough for OS crashes, faster than FULL).

Foreign Keys: ON (Strict referential integrity).

Busy Timeout: 200ms (Fail fast on lock contention to trigger infra-retry).

3. Decision: Schema Definition (SSOT)
3.1. jobs Table
Stores the lifecycle and metadata of all tasks.

SQL

CREATE TABLE jobs (
  -- Identity
  id TEXT PRIMARY KEY,            -- UUID v4
  trace_id TEXT,                  -- OpenTelemetry Trace ID (Added in Phase 2)
  
  -- Classification
  queue TEXT NOT NULL,            -- e.g., 'code_intel', 'build'
  job_type TEXT NOT NULL,         -- e.g., 'INDEX_FILE', 'RUN_TEST'
  subject_key TEXT NOT NULL,      -- Format: <client>::<repo>::<path> (For Supersede)
  generation INTEGER NOT NULL,    -- Monotonic counter for Supersede logic
  
  -- State & Timing
  priority INTEGER DEFAULT 0,
  state TEXT NOT NULL CHECK(state IN (
    'QUEUED', 'SCHEDULED', 'RUNNING', 'DONE', 
    'FAILED', 'CANCELLED', 'SUPERSEDED', 
    'SKIPPED_TTL', 'SKIPPED_DEADLINE'
  )),
  created_at INTEGER NOT NULL,    -- Epoch ms
  started_at INTEGER,
  finished_at INTEGER,

  -- Execution Details (Phase 2+)
  execution_mode TEXT CHECK(execution_mode IN ('IN_PROCESS', 'SUBPROCESS')),
  pid INTEGER,                    -- OS PID for Subprocess
  env_vars TEXT,                  -- JSON: Context-specific env vars
  
  -- Resilience (Phase 2+)
  attempts INTEGER DEFAULT 0,
  max_attempts INTEGER DEFAULT 0,
  backoff_factor REAL DEFAULT 2.0,
  deadline INTEGER,               -- Epoch ms (Hard timeout)
  ttl_ms INTEGER,                 -- Time-to-Live in Queue

  -- Scheduling & Conditions (Phase 3+)
  schedule_type TEXT CHECK(schedule_type IN ('IMMEDIATE', 'AT', 'AFTER', 'CONDITION')),
  scheduled_at INTEGER,
  schedule_delay_ms INTEGER,
  wait_for_idle INTEGER DEFAULT 0,      -- Boolean (0/1)
  require_charging INTEGER DEFAULT 0,   -- Boolean (0/1)
  wait_for_event TEXT,                  -- Event Key (e.g., 'git:commit')

  -- Grouping & UX (Phase 4)
  parent_job_id TEXT,             -- For atomic chains
  chain_group_id TEXT,            -- Logical grouping (e.g., CI Session)
  user_tag TEXT,                  -- User-facing tag (e.g., 'nightly-check')

  -- Outputs
  payload TEXT NOT NULL,          -- Input Arguments (JSON)
  result_summary TEXT,            -- Execution Result (JSON)
  log_path TEXT,                  -- Path to stdout/stderr log file
  artifact_path TEXT              -- Path to output artifacts directory
);
3.2. job_conditions Table (Phase 3+)
Normalized table for complex, multi-variable conditional scheduling.

SQL

CREATE TABLE job_conditions (
  job_id TEXT NOT NULL,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY (job_id, key),
  FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
);
4. Decision: Indexing Strategy
Indexes are designed to cover the most frequent access patterns (Pop, Supersede, GC).

Index Name	Columns	Purpose
idx_jobs_pop	(queue, priority DESC, created_at ASC, id)	Critical. Allows O(1) popping of the next job. id added for deterministic sorting.
idx_jobs_subject_generation	(subject_key, generation DESC)	Critical. Used by Planner to find and supersede obsolete jobs.
idx_jobs_state_queue	(state, queue)	Used for Admin Stats and Health Checks.
idx_jobs_gc	(finished_at)	Used by Garbage Collector to prune old history (WHERE finished_at < ?).
idx_jobs_user_tag	(user_tag)	Fast lookup for cancel(tag=...) operations.
idx_job_conditions_lookup	(key, value, job_id)	Reverse lookup for Event-driven triggering.

Sheets로 내보내기

5. Strategy: Phase-based Schema Evolution
The schema evolves additively across phases. Code must only access fields available in the current phase.

Field Category	Phase 1 (MVP)	Phase 2 (Execution)	Phase 3 (Scheduling)	Phase 4 (Ops/DX)
Identity	id, queue, job_type	trace_id	-	-
Supersede	subject_key, generation	-	-	-
State	state, created_at	-	-	-
Execution	payload, log_path	execution_mode, pid, env_vars	-	result_summary, artifact_path
Retry/Life	-	attempts, max, deadline, ttl	-	-
Scheduling	-	-	schedule_*, wait_for_*, job_conditions	-
Grouping	-	-	-	parent_id, chain_group, user_tag

Sheets로 내보내기

6. Strategy: Migration & Rollback
6.1. Migration Rules
Startup Check: The Daemon checks schema_version at startup.

Sequential Apply: Applies SQL files from migrations/ in order (e.g., 001_init.sql, 002_add_retry.sql).

Fail-Stop: If a migration fails, the Daemon aborts immediately. It does NOT attempt to auto-recover corrupt schema states.

Immutable History: Once released, migration files (001...sql) must never be modified.

6.2. Directory Structure
Plaintext

migrations/
  001_initial_schema.sql         # Phase 1
  002_add_execution_retry.sql    # Phase 2
  003_add_conditions.sql         # Phase 3
  004_add_dx_fields.sql          # Phase 4