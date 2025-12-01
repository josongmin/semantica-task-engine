ADR-000: Master Integration & Architecture Principles
Status: Accepted

Date: 2024-XX-XX

Scope: Root Document (Applies to the entire Semantica Orchestrator project)

Tags: #root #architecture #principles #ssot

1. Context
The Semantica Orchestrator is a high-performance, local daemon designed to manage AI-native development tasks (Indexing, Graph building, Testing, etc.). It operates in a resource-constrained environment (User's Laptop) and must guarantee data integrity, crash resilience, and strict type safety.

As the project grows, technical decisions are split across multiple ADRs. Without a central "Constitution," conflicts between documents (e.g., Schema definitions vs. Roadmap requirements) can lead to implementation ambiguity.

This document serves as the Master Integration ADR. It defines the Documentation Hierarchy, the Single Source of Truth (SSOT) rules, and the high-level Engineering Principles that all other documents must follow.

2. Documentation Priority & SSOT Rules
To prevent ambiguity, the following rules apply to all design documents and code implementations.

2.1. The "Rule of Law" (Hierarchy)
If two documents conflict, the specific Authority ADR prevails over general descriptions.

Conflict Domain	Authority ADR (The Winner)	Rationale
Database Schema	ADR-010: Database Persistence	ADR-010 is the SSOT for jobs, job_conditions DDL, and migrations. Any schema shown in other ADRs is illustrative only.
Code Structure	ADR-001: System Architecture	Rules for Hexagonal Architecture, Workspace layout, and Crate dependencies are absolute here.
Behavior / Ops	ADR-002: Operational Semantics	Logic for Failure, Throttling, Isolation, and Concurrency (Global Limiter) is defined here.
API Contract	ADR-020: API Contract	JSON-RPC method signatures and Error Codes defined here are the binding contract for SDKs.
Scope / Schedule	ADR-050: Development Roadmap	Phase definitions (P1~P4), Priorities, and Definitions of Done (DoD) are authoritative here.

Sheets로 내보내기

2.2. Phase-based Evolution Rule
The system evolves in strict Phases (1→4). Features marked for a later phase must not act as blockers for earlier phases.

Rule: Implementation must strictly follow the Schema Evolution Table defined in ADR-050 (Roadmap) and ADR-010 (Schema).

Constraint: Do not implement "Phase 3 Columns" (e.g., wait_for_idle) in Phase 1. Keep the schema minimal to the current phase's requirements.

3. The ADR Map (File Structure)
The documentation is reorganized into 9 specific files to separate concerns.

Core Architecture
ADR-000-master-integration.md: (This file) The root rules and principles.

ADR-001-system-architecture.md: Workspace layout, Crate details, Hexagonal dependency rules, Tech stack (tokio, sqlx).

ADR-002-operational-semantics.md: Failure handling, Retry policies, Global Resource Limiter, Isolation strategy.

Implementation Details
ADR-010-database-persistence.md: [Critical] Final DB Schema, Indexing strategy, Constraints, Migration/Rollback policy.

ADR-020-api-contract.md: JSON-RPC specification (dev.*, admin.*), Error models, SDK guidelines.

ADR-030-testing-strategy.md: Test layers (Unit/Integration/Golden), CI gates, QA policies.

Security & Lifecycle
ADR-040-security-policy.md: IPC Authentication, Peer Credentials, Subprocess Sandbox rules.

ADR-050-development-roadmap.md: Detailed Phase 1~4 scope, Schema mapping per phase, DoD.

ADR-060-distribution-lifecycle.md: Code signing, Auto-update mechanism, Installation scope.

4. Global Engineering Principles
4.1. AI-Native & Contract-First
Principle: We assume an AI Agent will write 50%+ of the code.

Rule: Ambiguity is the enemy. All interfaces (RPC, DB, Traits) must be strictly typed and defined before implementation logic (impl) is written.

Mechanism: Shared DTOs and Schema validation (ADR-010, ADR-020) act as the immutable contract between the Human Architect and the AI Coder.

4.2. Hexagonal Purity
Principle: The Core logic must be testable without sqlite or subprocess.

Rule: crates/core must never depend on infra-*. All external interactions must go through Port Traits defined in Core.

4.3. Observability by Default
Principle: If it's not logged, it didn't happen.

Rule: Every public operation must emit a Structured Log (JSON) and a Metric.

Traceability: A trace_id must propagate from the Client (SDK) → API → Core → Worker → Subprocess.

4.4. Crash-Resilience
Principle: The daemon will crash. The user will kill the process.

Rule: The system must recover to a consistent state upon restart.

Data: DB uses WAL mode. Transactions are atomic.

Process: Zombie processes are detected and killed on startup.

State: Interrupted jobs are re-queued or marked failed based on the Retry Policy (ADR-002).