ADR-050: Development Roadmap & Phase Definitions
0. Status
Accepted

1. Context
The Semantica Orchestrator evolves from a functional MVP to a SOTA AI-Native engine through four distinct phases. This document defines the Scope, Definition of Done (DoD), and Schema Evolution for each phase.

Constraint: Features defined in a specific Phase MUST NOT be implemented in earlier phases.

Dependency: Schema changes strictly follow the Evolution Matrix defined in Section 6.

2. Phase 1: Core Foundation (MVP)
Goal: Connectivity, Persistence, and Basic Execution.

Duration: ~2 Weeks.

2.1. Scope
Transport: JSON-RPC 2.0 over UDS (Unix) & Named Pipes (Windows).

Persistence: SQLite WAL Mode, Single-Writer/Multi-Reader.

Execution: IN_PROCESS mode only. No retry, no timeout.

Queue: FIFO ordering, Atomic Pop transaction.

Logging: Basic logs.tail API support.

2.2. Definition of Done (DoD)
[ ] Indexing: Can successfully index 100+ files in the Semantica Repo.

[ ] Persistence: Daemon restart restores QUEUED jobs without data loss.

[ ] Concurrency: meta.db operates under load without SQLITE_BUSY errors.

[ ] API: dev.enqueue, dev.cancel, logs.tail are fully functional.

3. Phase 2: Execution Engine Hardening
Goal: Safety, Isolation, and Crash Recovery.

Duration: ~3 Weeks.

3.1. Scope
Subprocess: Support execution_mode = SUBPROCESS with PID tracking.

Resilience: Panic handling (catch_unwind), Zombie process killing on startup.

Retry Logic: attempts, max_attempts, backoff_factor.

Timeouts: deadline (Run limit) and ttl_ms (Queue limit).

System Probe: Basic CPU/Memory monitoring.

3.2. Definition of Done (DoD)
[ ] Isolation: A worker panic or subprocess crash does NOT kill the Daemon.

[ ] Recovery: Restarting the Daemon cleans up orphaned PIDs and recovers jobs.

[ ] Throttling: System pauses low-priority queues when CPU > 90%.

[ ] Retry: Transient failures trigger exponential backoff correctly.

4. Phase 3: AI-Native Scheduling
Goal: Context Awareness and Conditional Execution.

Duration: ~2 Weeks.

4.1. Scope
Conditions: wait_for_idle, require_charging, wait_for_event.

Schema: Introduce job_conditions table.

Planner: Event coalescing (e.g., merging 50 FS events into 1 job).

Supersede: Advanced "Insert-time" and "Pop-time" supersede logic.

Backpressure: Dynamic throttling based on Battery/IO.

4.2. Definition of Done (DoD)
[ ] Idle Trigger: Heavy indexing tasks only start when the user stops typing.

[ ] Event Trigger: "Rebuild on PR Merge" workflow functions reliably.

[ ] Efficiency: Supersede logic reduces redundant job executions by >80% during typing bursts.

5. Phase 4: Reliability & Ops (Production)
Goal: SRE-Grade Stability and Observability.

Duration: ~3 Weeks.

5.1. Scope
Observability: Full Structured Logging (JSON), OpenTelemetry Metrics.

UX: Tag-based management (user_tag, chain_group_id).

Maintenance: Automated VACUUM, Artifact Garbage Collection.

Lifecycle: Zero-downtime-like upgrades (Migration rollback capability).

5.2. Definition of Done (DoD)
[ ] Stability: 2 weeks of continuous operation with zero leaks or degradation.

[ ] Debuggability: Root cause of failures can be identified solely from logs.

[ ] Upgrade: Schema migration and rollback tested in CI.

6. Schema Evolution Matrix
The jobs and job_conditions tables evolve additively. Code in Phase N must not rely on fields from Phase N+1.

Field / Feature	Phase 1 (MVP)	Phase 2 (Exec)	Phase 3 (Sched)	Phase 4 (Ops)
Core Identity (id, queue, type)	✅	✅	✅	✅
Basic State (state, created_at)	✅	✅	✅	✅
Payload/Logs (payload, log_path)	✅	✅	✅	✅
Execution Mode (mode, pid, env)	❌	✅	✅	✅
Retry/Limits (attempts, deadline)	❌	✅	✅	✅
Scheduling (schedule_*)	❌	❌	✅	✅
Conditions (wait_for_*, job_conditions)	❌	❌	✅	✅
Grouping/DX (user_tag, chain_group)	❌	❌	❌	✅
Results (result_summary, artifacts)	❌	❌	❌	✅

Sheets로 내보내기

6.1. Migration Policy
Phase 1 → 2: Run 002_add_execution_retry.sql.

Phase 2 → 3: Run 003_add_conditions.sql.

Phase 3 → 4: Run 004_add_dx_fields.sql.ㅂ대