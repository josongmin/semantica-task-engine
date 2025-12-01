기존의 09. Test.md (Test Policy), 08. DB...md (Golden Test Framework), 그리고 07. ADR patch...md (Phase-based Test Plan) 내용을 모두 통합하여 ADR-030: Testing Strategy를 작성해 드립니다.

이 문서는 단순한 테스트 가이드가 아니라, AI 에이전트가 코드를 수정할 때 반드시 따라야 하는 품질 보증 헌법(Constitution) 역할을 합니다.

ADR-030: Testing Strategy & Quality Assurance
0. Status
Accepted

1. Context
The Semantica Orchestrator manages complex, asynchronous, and stateful workflows. As we introduce Autonomous AI Agents to the codebase, the risk of subtle regressions (e.g., breaking retry logic, degrading RAG quality) increases significantly.

We elevate Testing from a "best practice" to a Mandatory Policy. Our goal is a state where "The AI develops at SOTA speed, but is mathematically incapable of breaking the system."

2. Decision: The "No Test, No Touch" Policy
We enforce the following rules via CI/CD pipelines and Code Review norms:

Strict Hierarchy: Code changes must be covered by the appropriate test layer (Unit, Contract, Integration, or Golden).

No Test, No Touch: It is forbidden to modify logic without existing test coverage. If coverage is missing, the AI Agent must write the test first (Refactoring-Safe state) before making changes.

CI Enforcement: Pull Requests without relevant test updates or passing results are automatically rejected.

3. Decision: Test Pyramid & Layers
Layer	Target	Role	Constraint
Unit	Pure functions, Domain Logic	Pin Business Rules & Edge Cases.	Must mock all I/O (DB, Network, Time). Fast (< 1ms).
Contract	SDK, RPC, DTOs	Pin API Schema & Error Codes.	Ensures backward compatibility. Breaking changes require version bumps.
Integration	DB, Worker Loop, IPC	Verify Transaction & Pipeline consistency.	Uses In-Memory SQLite and real IPC sockets.
Golden	Scheduler, Planner, RAG	Verify Complex Temporal Logic & Quality.	Uses Snapshot Testing with Deterministic Mocks.

Sheets로 내보내기

4. Decision: Golden Test Framework (For Non-Determinism)
The Planner and Scheduler involve complex temporal logic (Debounce, Backoff, Supersede) that is hard to verify with standard unit tests. We use Golden Tests (Snapshot Testing) to lock in behavior.

4.1. Directory Structure
Plaintext

tests/golden/
├── planner/                 # Event -> Job Creation Logic
│   ├── fs_event_burst.json
│   └── git_rebase_storm.json
└── scheduler/               # Queue -> State Transition Logic
    ├── supersede_debounce.json
    ├── retry_backoff.json
    └── recovery_zombie.json
4.2. Handling Non-Determinism
To ensure snapshots are stable, the Test Runner MUST inject:

MockClock: Instead of SystemTime::now(), logic uses ctx.now(). Time advances only by explicit ticks in the test scenario.

Seeded RNG / UUIDs: UUIDs are generated sequentially (job-001, job-002) or via a fixed seed, ensuring the output JSON is identical across runs.

4.3. Golden File Format Example
Scenario (scheduler/retry_backoff.json):

JSON

{
  "scenario": "Transient failure should trigger exponential backoff",
  "initial_db": [
    { "id": "job-1", "state": "RUNNING", "attempts": 0 }
  ],
  "actions": [
    { "tick": 100, "action": "worker_fail", "job_id": "job-1", "error": "transient" }
  ],
  "expected_transitions": [
    { 
      "job_id": "job-1", 
      "from": "RUNNING", 
      "to": "QUEUED", 
      "scheduled_delay": 2000,
      "reason": "backoff"
    }
  ]
}
5. Decision: Phase-based Test Plan & DoD
Each development phase has specific functional scenarios that serve as the Definition of Done (DoD).

5.1. Phase 1: Core Foundation (MVP)
Goal: No Data Loss. FIFO Ordering.

Scenarios:

(F1) Order: Enqueue 100 jobs -> Verify FIFO execution.

(F2) Persistence: Kill Daemon (kill -9) -> Restart -> Verify QUEUED jobs persist.

(F3) Log Tail: Verify logs.tail streams data correctly.

5.2. Phase 2: Execution Engine (Hardening)
Goal: Subprocess Safety & Crash Recovery.

Scenarios:

(E1) Isolation: Subprocess panic does not crash Daemon.

(E2) Zombie Kill: Restart Daemon -> Find orphaned pid -> SIGKILL & Mark FAILED.

(E3) TTL/Deadline: Verify jobs expire if not picked up or finished in time.

5.3. Phase 3: AI-Native Scheduling
Goal: Context Awareness.

Scenarios:

(S1) Idle: wait_for_idle=true jobs start only when SystemProbe reports idle.

(S2) Supersede: 50 rapid events result in 1 actual job (Debounce/Supersede verification).

(S3) Backpressure: CPU > 80% -> Low priority queues pause.

5.4. Phase 4: Reliability & Ops
Goal: SRE-Grade Stability.

Scenarios:

(R1) Long-Run: 24h stress test with random faults. Memory usage stable.

(R2) Migration: Upgrade binary with new schema -> Migration succeeds.

(R3) Rollback: Bad schema -> Daemon refuses to start (Fail-Safe).

6. Appendix: AI Agent System Prompt
Copy this block into the System Prompt of any AI Agent working on this repository to enforce test discipline.

Markdown

[CRITICAL DEVELOPMENT CONSTRAINTS - TEST FIRST POLICY]

You are an Autonomous Developer Agent. You must adhere to the following Constitution.

1. **Test-Driven Modification:** - You cannot modify code that does not have existing tests.
   - If coverage is missing, WRITE THE TEST FIRST to capture current behavior.

2. **Contract Inviolability:**
   - Never break the Public API (Schema, Error Codes).
   - If you change a DTO, you MUST update the Contract Test.

3. **Golden Set Quality:**
   - For Planner/Scheduler logic, you must run the Golden Test suite.
   - If a snapshot mismatch occurs, analyze if it's a Regression or an Intended Change.

4. **Deterministic Testing:**
   - Use `MockClock` and `SeededRNG`. NEVER use `sleep()` in tests.

5. **Failure Explanation:**
   - When a test fails, analyze the assertion message. It is the absolute truth.