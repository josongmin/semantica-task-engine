This is the final ADR-001. It defines the static structure and technology standards for the project.

ADR-001: System Architecture & Tech Stack
Status: Accepted

Date: 2024-XX-XX

Scope: Global (Project Structure, Library Selection, Build Configuration)

Tags: #architecture, #rust, #hexagonal, #workspace, #stack

1. Context
The Semantica Orchestrator is a long-running, local, AI-native task daemon. It manages diverse workloads including SQLite persistence, OS-level subprocess execution, and IPC communication.

To ensure long-term maintainability, testability, and type safety, the system cannot be a monolithic script. It requires:

Strict Boundary Enforcement: Domain logic must be isolated from technical details (DB, OS).

Modular Compilation: Changes in one adapter should not force a recompile of the core logic.

Standardized Tooling: A "Gold Standard" tech stack to reduce cognitive load and ensure stability.

This ADR defines the Hexagonal Architecture implementation using a Rust Workspace strategy and locks the Technology Stack.

2. Architectural Pattern: Hexagonal (Ports & Adapters)
We adopt Hexagonal Architecture as the governing principle.

2.1. The Dependency Rule
The most critical rule of this architecture is the direction of dependencies:

The Core (Domain) depends on NOTHING.

Infrastructure (Adapters) depends on The Core.

This allows us to swap the Database (SQLite → Postgres) or the Transport (UDS → TCP) without touching a single line of business logic.

2.2. Layer Definitions
Domain (Inner Hexagon): Pure entities, rules, and logic.

Ports (The Boundary): Traits (Interfaces) defined in the Core that dictate how the outside world interacts with the Core.

Adapters (Outer Hexagon): Concrete implementations of Ports (e.g., SqliteJobRepository, RpcServer).

Composition Root: The entry point where Adapters are instantiated and injected into the Core.

3. Workspace Structure (The Physical Layout)
The project utilizes a Cargo Workspace to enforce physical separation of concerns.

Plaintext

semantica-orchestrator/
  Cargo.toml              # Workspace definitions
  rust-toolchain.toml
  README.md

  crates/
    # --- The Hexagon (Core Logic) ---
    core/                 # [LIB] Domain Entities + Use Cases + Port Traits
                          # Dependencies: Pure Rust only (serde, chrono, uuid)

    # --- Outbound Adapters (Driven) ---
    infra-sqlite/         # [LIB] Persistence Adapter
                          # Implements: JobRepository, SubjectRepository
    infra-system/         # [LIB] System Adapter (OS Probe, Subprocess)
                          # Implements: SystemProbe, TaskExecutor
    infra-metrics/        # [LIB] Observability Adapter
                          # Implements: Logger, MetricsExporter

    # --- Inbound Adapters (Driving) ---
    api-rpc/              # [LIB] Transport Layer
                          # Exposes: JSON-RPC over UDS/Named Pipe
    api-cli/              # [BIN] Command Line Interface
                          # Consumes: JSON-RPC Client

    # --- Composition Root ---
    daemon/               # [BIN] The Main Executable
                          # Responsibility: DI Wiring, Config Loading, Shutdown Hook
4. Technology Stack (The Standard Kit)
We rely on the "De Facto Standard" ecosystem to ensure stability and developer familiarity.

Category	Crate	Justification
Async Runtime	tokio	Industry standard. Handles networking, timers, signals, and FS threads. Features: full.
Database	sqlx	Async SQLite with Compile-time Query Verification. Ensures type safety for SQL queries.
Serialization	serde	Universal standard for serialization. Used for Config, RPC DTOs, and DB JSON columns.
Error Handling	thiserror	For Library/Core errors. Defines explicit, typed enums for domain logic.
Error Handling	anyhow	For App/Daemon errors. Easy propagation of errors in the composition root.
Observability	tracing	Async-aware structured logging. Essential for debugging concurrent tasks.
RPC	jsonrpsee	Type-safe JSON-RPC 2.0 implementation. Abstracts transport details.
Config	config	Layered configuration (Default -> File -> Env Vars).
System Info	sysinfo	Cross-platform monitoring for CPU, Memory, and Process states.
Utils	uuid, chrono	Standard ID generation (v4) and Time handling.
Dirs	directories	Adheres to XDG/OS standards for data/config paths.

Sheets로 내보내기

5. Build & Development Configuration
Rust compile times can be significant. We optimize Cargo.toml profiles to balance iteration speed (Dev) and runtime performance (Release).

5.1. Dev Profile (Optimized for Speed)
We optimize external dependencies even in debug mode to ensure the runtime isn't sluggish, while keeping our code compile-fast.

Ini, TOML

[profile.dev.package."*"]
opt-level = 3  # Optimize dependencies
5.2. Release Profile (Optimized for Performance)
Ini, TOML

[profile.release]
lto = true          # Link Time Optimization
codegen-units = 1   # Maximize optimization (slower build, faster binary)
strip = true        # Reduce binary size
panic = "abort"     # Reduce binary size, simpler stack unwinding
5.3. Linting & Formatting
Formatter: rustfmt with rustfmt.toml.

Linter: clippy.

CI Policy: cargo clippy -- -D warnings (Warnings are treated as Errors).

6. Implementation Guidelines
6.1. Dependency Flow Check
Developers must verify the dependency graph:

✅ daemon depends on core, infra-*, api-*.

✅ infra-* depends on core.

✅ api-* depends on core.

⛔ core depends on infra-* (FORBIDDEN).

⛔ core depends on sqlx (FORBIDDEN - except for types, but prefer pure domain types).

6.2. Module Responsibilities
crates/core/src/domain: Pure structs. No async logic usually.

crates/core/src/port: Traits only. #[async_trait] is allowed here.

crates/core/src/application: The "Service" layer. Orchestrates Ports to fulfill a Use Case.

crates/daemon/src/bootstrap.rs: The only place where Arc<SqliteJobRepository> is cast to Arc<dyn JobRepository>.

7. Consequences
Positive
Testability: Core logic can be unit tested with Mocks (mockall) without spinning up a real DB or Spawning subprocesses.

Parallel Development: One engineer can work on infra-sqlite while another works on api-rpc with minimal merge conflicts.

Stability: The Domain model is protected from external library changes.

Negative
Boilerplate: Requires defining Traits (Ports) and implementing them (Adapters), which is more verbose than direct calls.

Compile Time: Heavy dependencies (sqlx, tokio) increase initial build times (mitigated by build profiles).