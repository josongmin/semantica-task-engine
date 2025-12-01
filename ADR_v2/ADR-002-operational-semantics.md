Based on the provided context files and your request for a SOTA (State of the Art) Operational Model, here is the finalized ADR-002: Operational Semantics.

I have integrated the core logic from 04. Failure·Throttling·Isolation·Ops.md with the architectural constraints from 01, 03, and 05. Additionally, I have reinforced specific areas—particularly Circuit Breaking for Infra Failures, Fairness Scheduling, and OpenTelemetry compatibility—to meet SOTA operational standards.

ADR-002: Operational Model – Failure, Throttling, Isolation & Reliability
Status: Accepted Date: 2024-XX-XX Pre-requisites: ADR-000 (Core Architecture) Scope: Semantica Orchestrator (Long-running Daemon Operations)

1. Context
The Semantica Orchestrator is not merely a transient job queue; it is a long-running background daemon operating within a resource-constrained local environment (User’s Laptop/Workstation).

Unlike server-side clusters, this orchestrator must:

Survive Crashes: Auto-recover state without data corruption.

Respect the User: Dynamically throttle based on CPU, Memory, Battery, and GPU usage.

Isolate Contexts: Prevent one heavy repository or client from starving others.

Guarantee Consistency: Clearly define strong consistency (Job DB) vs. eventual consistency (Search Indices).

This ADR defines the Operational Semantics required to make the Core Architecture (defined in ADR-000) robust, observable, and maintainable.

2. Decision Summary
Domain	Strategy
Failure Semantics	Strict classification (Transient / Permanent / Infra). Exponential backoff. Zombie process cleanup.
Throttling	Multi-vector Backpressure (Queue Length + System Resources + Global Tokens).
Isolation	Logical: Per-repo subject keys with weighted priority. Process: Daemon/Client separation. RPC: Short/Long-request segregation.
Consistency	Meta DB: Strong Consistency (Single Writer). Planner/Worker: Eventual Consistency with defined bounds.
SLAs	Defined p95 latency targets for Enqueue, Pop, and Event-to-Running.
Observability	Structured JSON Logs, OTel-compatible Metrics, Tracing, and Health Checks.

Sheets로 내보내기

3. Failure Semantics & Recovery
3-1. Failure Classification
Workers must classify errors into three distinct categories to determine the recovery strategy.

Transient Failure (retryable)

Examples: Network timeout, File lock contention, GPU OOM (spikes), External tool temporary failure.

Action: Retry with exponential backoff.

Permanent Failure (fatal)

Examples: Invalid payload (Schema violation), Compilation syntax error, Missing mandatory file, Repo not found.

Action: Fail immediately. No retry. Move to Dead Letter Queue (DLQ) logic (marked as FAILED).

Infrastructure Failure (critical)

Examples: SQLite busy_timeout persistence, Disk full, Permission denied on DB, Corrupt WAL.

Action: Retry limited times + Trigger Circuit Breaker + Alert.

Result Payload Schema:

JSON

{
  "status": "failed",
  "error_kind": "transient", // or "permanent", "infra"
  "code": "ERR_GPU_OOM",
  "message": "Allocation failed",
  "duration_ms": 1250,
  "attempt": 2
}
3-2. Retry & Backoff Policy
Standard Logic: delay = initial_backoff_ms * (backoff_factor ^ (attempt - 1))

Jitter: Apply ±10% random jitter to prevent "Thundering Herd" problems.

State: During backoff, job state is QUEUED (with scheduled_at updated) or RETRY_WAIT.

Queue-Specific Overrides:

GC/Cleanup: Max 1 retry (Fail fast).

Indexing/Embedding: Max 5 retries, aggressive backoff.

Meta Migration: Unlimited retry (blocking start) or crash process.

3-3. Crash Recovery & Zombie Management
Upon Daemon Startup, the system performs a Recovery Scan:

Detect RUNNING Jobs: Jobs marked as RUNNING but with started_at < (now - recovery_window).

Resolution by Mode:

IN_PROCESS: Requeue immediately (state → PENDING). Increment attempts. If attempts > max, mark FAILED (Prevents Infinite Crash Loops).

SUBPROCESS:

Check OS Process Table for job.pid.

Found: Send SIGTERM, wait, then SIGKILL. Mark job FAILED.

Not Found: Mark job FAILED (Process died silently).

Deadline Enforcement:

If now > job.deadline, mark SKIPPED_DEADLINE.

Background CleanupJob deletes expired TTL records.

3-4. Partial Execution (Idempotency)
Worker Contract: Workers must be designed to be idempotent.

Partial Success: If a worker processes 50/100 files and fails:

Ideally: Commit the 50 results to the index, mark job failed.

On Retry: Worker detects existing state and resumes or overwrites.

Orchestrator Role: The Orchestrator does not manage partial rollbacks. It only manages the Job State.

4. Throttling & Backpressure Model
4-1. Observation Vectors
The Planner and Scheduler make decisions based on:

Queue Pressure: Depth, Wait Time, Supersede Rate.

System Pressure: CPU Usage, Memory Pressure, Battery State, Thermal Throttling.

Storage Pressure: DB WAL Size, Disk Availability.

Global Limiter: Token availability (CPU/GPU/IO).

4-2. Backpressure Rules
Condition	Action
Queue > Max Length	Reject new Enqueue (429) OR Drop lowest priority (Shed Load).
CPU High (> 80%)	Increase Worker Pull Interval. Pause "Heavy" queues.
Battery Low & Discharging	Pause all heavy jobs (Training, Bulk Indexing). Only allow light (Editing) jobs.
DB Locked / Large WAL	Pause all Enqueue. Prioritize VACUUM / CHECKPOINT tasks.

Sheets로 내보내기

4-3. Global Resource Limiter (Token Bucket)
To prevent resource contention between parallel workers, a global semaphore system is used.

Rust

struct GlobalLimiter {
    cpu_tokens: Semaphore, // Total Cores - Reserve
    gpu_tokens: Semaphore, // e.g., 1 (Exclusive access)
    io_tokens:  Semaphore, // Concurrent heavy IO ops
}
Acquisition: Worker must acquire tokens before moving to RUNNING.

Acquisition Timeout: If token not acquired within token_acquire_timeout (e.g., 30s), job is re-queued or backed off to prevent deadlock.

Starvation Prevention: If a job waits for tokens > timeout, it yields and increases its priority slightly for the next attempt (Aging).

4-4. Worker-Level Checkpoints
Long-running workers (e.g., "Index entire repo") must:

Periodically check for Cancellation Tokens.

Periodically check for Pause Signals (e.g., User unplugged power).

Save intermediate state to allow resuming.

5. Isolation Strategy
5-1. Repo-Level Isolation (Fairness)
Subject Key: <client_id>::<repo_id>::<scope>

Fair Scheduling: The Scheduler uses a Weighted Round Robin approach.

Prevents one active repo from blocking jobs for a different repo.

Priority Weighting:

Active Window (User currently typing): Weight 100.

Background Repo: Weight 10.

Hotness Metric: Dynamically adjust weight based on loc_24h or recent activity.

5-2. Process Isolation
Architecture: Daemon is a standalone binary/process.

Benefit: IDE crashing does not kill the Orchestrator. Orchestrator crashing does not kill the IDE.

Database Lock: Strict busy_timeout ensures the Orchestrator doesn't freeze due to a rogue reader holding a lock.

5-3. RPC Isolation
Separation:

Control Plane (Short): enqueue, cancel, health. (Time budget: < 50ms).

Data Plane (Long): pop, log_tail, watch. (Time budget: Seconds/Minutes).

Handling: Long requests run on a separate Tokio thread pool to ensure Control Plane responsiveness.

6. Consistency Model
6-1. JobQueue (meta.db) - Strong Consistency
Single Source of Truth: meta.db (SQLite).

Atomicity: Pop, State Transitions, and Supersede Logic occur within a Single Writer Transaction.

Guarantee: A job is never assigned to two workers simultaneously.

6-2. Planner State - Eventual-but-Bounded
Event Driven: FileSystem/Git events trigger the Planner.

Debounce: Rapid events are aggregated.

Bounded: The system guarantees a "Reconciliation Loop" (e.g., every 30s) to catch any missed events, ensuring the plan eventually matches the disk state.

6-3. Worker Output - Eventual Consistency
Search Indices/Embeddings: Updates are asynchronous.

Query Time: Queries may run against "stale" indices while indexing is RUNNING.

Versioning: Workers manage version switching (e.g., index_v1 -> index_v2) atomically upon completion.

7. Scheduling Contracts (SLA)
7-1. API Latency Targets (p95)
enqueue: ≤ 10ms (Fast ack).

stats / health: ≤ 20ms.

pop (Item available): ≤ 20ms.

pop (Empty): Returns immediately or blocks until timeout (Long Polling).

7-2. End-to-End Latency
Event-to-Plan: File Change → Job Created: Avg < 1s.

Queue-to-Run: Job Created → Worker Pickup (System Idle): Avg < 200ms.

Violation: Latency breaches trigger "Performance Degradation" logs/metrics but do not crash the system.

8. Observability Model
8-1. Structured Logging
All logs must be JSON.

JSON

{
  "level": "INFO",
  "ts": "2024-01-01T12:00:00Z",
  "trace_id": "req-123",
  "component": "scheduler",
  "event": "job_superseded",
  "job_id": "job-abc",
  "old_gen": 1,
  "new_gen": 2,
  "reason": "newer_file_version"
}
8-2. Metrics (OpenTelemetry Compatible)
Counters: jobs_enqueued, jobs_failed, jobs_superseded, tokens_acquired.

Gauges: queue_depth, active_workers, cpu_usage, wal_size_bytes.

Histograms: job_duration_ms, queue_wait_ms, db_txn_time_ms.

8-3. Health Checks
Liveness: Daemon process is running. RPC port is open.

Readiness: DB is connected. Migrations applied. GlobalLimiter initialized.

Self-Check: Periodically enqueue a synthetic "Canary Job" to verify full pipeline functionality.

9. Lifecycle & Upgrade Strategy
9-1. Graceful Shutdown
Stop Listener: Reject new RPC connections.

Cancel Idle: Cancel waiting jobs.

Drain/Kill Running:

Short Jobs: Wait for completion (max 5s).

Long Jobs: Send Cancellation Token -> Wait -> Force Kill.

Close Resources: Release DB locks, close File Handles.

9-2. Upgrade & Migration
Atomic Migration: DB Schema migrations run synchronously at startup. If migration fails, Daemon aborts (Prevention of data corruption).

Fallback: Recommendation to snapshot meta.db before applying major version upgrades.

9-3. Maintenance
VACUUM: Triggered by SystemProbe when idle + charging. Optimizes SQLite pages and WAL file.

Pruning: DELETE FROM jobs WHERE finished_at < NOW - 7 DAYS.

10. Testing Strategy
Chaos Testing:

Randomly kill -9 the Orchestrator during high load. Verify consistency on restart.

Simulate Locked database. Verify proper backoff/infra-fail handling.

Integration Testing:

Validate GlobalLimiter prevents over-subscription.

Verify Supersede logic correctly discards obsolete jobs under rapid fire.

SDK Verification:

Verify Python/TS SDKs correctly handle reconnection and trace_id propagation.