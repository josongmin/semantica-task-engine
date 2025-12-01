# ADR-070: Integration Enforcement Protocol

## Status
**Accepted** - Mandatory enforcement pattern for all new capabilities

## Context

### The "Orphaned Code" Problem
A critical failure mode in Hexagonal Architecture is **orphaned implementation**:
- Trait defined in `port/` but never used
- Implementation in `infra-*/` but never wired to daemon
- Field added to struct but never initialized
- Consumer updated but daemon not wired

This results in:
1. **Silent Feature Incompleteness**: Code exists but doesn't run
2. **Dead Code Accumulation**: Warnings suppressed with `#[allow(dead_code)]`
3. **Integration Gaps**: Components implemented but not connected
4. **False Completion**: Task marked "done" but feature non-functional

### Real-World Example (Anti-Pattern)

```rust
// Step 1: Developer adds TaskExecutor trait
// port/task_executor.rs
pub trait TaskExecutor {
    fn execute(&self, job: &Job) -> Result<()>;
}

// Step 2: Developer implements it
// infra-system/executor.rs
pub struct SubprocessExecutor;
impl TaskExecutor for SubprocessExecutor { ... }

// Step 3: Developer "forgets" to wire it
// application/worker/mod.rs
pub struct Worker {
    job_repo: Arc<dyn JobRepository>,
    // Missing: executor field
}

// Result: Feature "implemented" but NEVER runs
// No compiler error, just dead code warnings
```

## Decision

### Wire-First, Implement-Second Pattern

All new capabilities MUST follow this **STRICT 5-step sequence**:

```
Define Port → Break Consumer → Wire Daemon → Implement Logic → Implement Infra
```

### Step-by-Step Protocol

#### Step 1: Define Port (Trait)
Define the interface in `crates/core/src/port/`:

```rust
// crates/core/src/port/task_executor.rs
use crate::domain::Job;
use crate::error::Result;
use async_trait::async_trait;

#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a job with the given execution mode
    async fn execute(&self, job: &Job) -> Result<()>;
}
```

**Checkpoint**: Port defined, but NOT used yet.

---

#### Step 2: Break the Consumer (MUST FAIL)
Immediately add the trait as a dependency in the consuming component:

```rust
// crates/core/src/application/worker/mod.rs
use crate::port::TaskExecutor;
use std::sync::Arc;

pub struct Worker {
    job_repo: Arc<dyn JobRepository>,
    executor: Arc<dyn TaskExecutor>,  // NEW FIELD
}

impl Worker {
    pub fn new(
        job_repo: Arc<dyn JobRepository>,
        executor: Arc<dyn TaskExecutor>,  // NEW PARAMETER
    ) -> Self {
        Self { job_repo, executor }
    }
}
```

**Checkpoint**: Run `cargo check`. It MUST fail:

```
error[E0061]: this function takes 2 arguments but 1 argument was supplied
  --> crates/daemon/src/main.rs:42:20
   |
42 |     let worker = Worker::new(repo);
   |                  ^^^^^^^^^^^ ---- supplied 1 argument
   |
note: function defined here
  --> crates/core/src/application/worker/mod.rs:15:12
   |
15 |     pub fn new(job_repo: Arc<dyn JobRepository>, executor: Arc<dyn TaskExecutor>) -> Self {
   |            ^^^
```

**CRITICAL**: If `cargo check` does NOT fail, you skipped a step. STOP and fix.

---

#### Step 3: Wire the Daemon (Fix the Compilation Error)
Go to `crates/daemon/src/main.rs` and provide the implementation:

```rust
// crates/daemon/src/main.rs
use crate::infra_system::SubprocessExecutor;  // Or placeholder

#[tokio::main]
async fn main() -> Result<()> {
    let repo = Arc::new(SqliteJobRepository::new(pool));
    
    // Wire the executor (even if placeholder for now)
    let executor = Arc::new(SubprocessExecutor::new());
    
    let worker = Worker::new(repo, executor);  // NOW compiles
    
    // ...
}
```

**Checkpoint**: `cargo check` now passes. Integration skeleton complete.

---

#### Step 4: Implement Logic (Use the Field)
Now implement the actual logic in the consumer:

```rust
// crates/core/src/application/worker/mod.rs
impl Worker {
    pub async fn process_next_job(&self) -> Result<bool> {
        let mut job = match self.job_repo.pop_next(&self.queue).await? {
            Some(j) => j,
            None => return Ok(false),
        };

        // NOW actually using the executor field
        if let Err(e) = self.executor.execute(&job).await {
            job.fail();
            self.job_repo.update(&job).await?;
            return Err(e);
        }

        job.complete();
        self.job_repo.update(&job).await?;
        Ok(true)
    }
}
```

**Checkpoint**: Field is now used. No `warning: field is never read`.

---

#### Step 5: Implement Infra (Concrete Implementation)
Finally, write the real implementation:

```rust
// crates/infra-system/src/task_executor.rs
use crate::port::TaskExecutor;
use async_trait::async_trait;

pub struct SubprocessExecutor {
    timeout: Duration,
}

impl SubprocessExecutor {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(300),
        }
    }
}

#[async_trait]
impl TaskExecutor for SubprocessExecutor {
    async fn execute(&self, job: &Job) -> Result<()> {
        // Real implementation
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(job.payload.as_value()["command"].as_str().unwrap())
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        Ok(())
    }
}
```

**Checkpoint**: Implementation complete and fully wired.

---

## Enforcement Rules

### Rule 1: Compiler as Gatekeeper
- **NEVER** use `#[allow(dead_code)]` to silence warnings
- **NEVER** use `#[allow(unused_imports)]`
- **ALWAYS** treat these warnings as **CRITICAL ERRORS**:
  ```
  warning: field is never read
  warning: unused import
  warning: associated function is never used
  ```

### Rule 2: Grep Proof Verification
Before marking any task as "Done", run verification:

```bash
# Verify trait usage in application
grep -r "YourNewTrait" crates/core/src/application/
# Expected: Should find actual usage, not just imports

# Verify wiring in daemon
grep -r "YourNewImpl" crates/daemon/src/
# Expected: Should find construction and injection

# Verify no dead code warnings
cargo check 2>&1 | grep "never read\|never used"
# Expected: Empty output
```

### Rule 3: Integration Test Required
Every new Port/Infra pair MUST have an integration test:

```rust
// tests/integration_executor.rs
#[tokio::test]
async fn test_executor_integration() {
    // Setup
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();
    let repo = Arc::new(SqliteJobRepository::new(pool));
    let executor = Arc::new(SubprocessExecutor::new());
    
    // Create worker with wired executor
    let worker = Worker::new("test_queue", repo.clone(), executor);
    
    // Enqueue a job
    let job_id = repo.insert(&Job::new(...)).await.unwrap();
    
    // Process job (this MUST use the executor)
    let processed = worker.process_next_job().await.unwrap();
    assert!(processed);
    
    // Verify executor ran (check side effects)
    let job = repo.find_by_id(&job_id).await.unwrap().unwrap();
    assert_eq!(job.state, JobState::Done);
}
```

### Rule 4: PR Checklist Addition
Every PR that adds a new Port MUST include:

```markdown
## Integration Checklist
- [ ] Step 1: Port trait defined in `crates/core/src/port/`
- [ ] Step 2: Consumer updated (caused compilation error)
- [ ] Step 3: Daemon wired (compilation error fixed)
- [ ] Step 4: Logic implemented (field actually used)
- [ ] Step 5: Infra implemented (concrete implementation)
- [ ] No `dead_code` warnings: `cargo check 2>&1 | grep "never"`
- [ ] Grep verification passed
- [ ] Integration test added
```

## Anti-Patterns (FORBIDDEN)

### ❌ Anti-Pattern 1: Trait Without Usage
```rust
// port/metrics.rs
pub trait MetricsCollector { ... }

// NO usage in application layer
// NO wiring in daemon
// Result: Dead trait
```

### ❌ Anti-Pattern 2: Field Without Initialization
```rust
pub struct Worker {
    executor: Arc<dyn TaskExecutor>,
}

// No constructor parameter
// No initialization in daemon
// Compiler warning suppressed with #[allow(dead_code)]
```

### ❌ Anti-Pattern 3: Silencing Warnings
```rust
#[allow(dead_code)]  // ❌ FORBIDDEN
pub struct Worker {
    executor: Arc<dyn TaskExecutor>,
}
```

### ❌ Anti-Pattern 4: Skipping Integration Test
```rust
// Only unit tests for TaskExecutor
// No test proving Worker + TaskExecutor integration
// Result: Integration gap not caught
```

## Correct Patterns (REQUIRED)

### ✅ Correct Pattern 1: Full Wire-First
```rust
// 1. Port
trait Executor { ... }

// 2. Consumer (breaks compilation)
struct Worker { executor: Arc<dyn Executor> }

// 3. Daemon (fixes compilation)
let executor = Arc::new(RealExecutor::new());
let worker = Worker::new(repo, executor);

// 4. Logic (uses field)
impl Worker { fn run(&self) { self.executor.execute(...) } }

// 5. Infra (implementation)
struct RealExecutor;
impl Executor for RealExecutor { ... }
```

### ✅ Correct Pattern 2: Placeholder-First
If you can't implement the infra immediately:

```rust
// Step 3: Use a placeholder
struct TodoExecutor;
impl TaskExecutor for TodoExecutor {
    async fn execute(&self, _job: &Job) -> Result<()> {
        // TODO(@username): Implement subprocess execution by 2024-12-20
        tracing::warn!("Using placeholder executor");
        Ok(())
    }
}

// Wire the placeholder
let executor = Arc::new(TodoExecutor);
let worker = Worker::new(repo, executor);
```

**Key**: Even placeholder is WIRED. No orphaned code.

## Benefits

1. **Zero Orphaned Code**: Impossible to write code that never runs
2. **Compiler Enforced**: Integration verified at compile time
3. **Clear Progress**: Each step has a verifiable checkpoint
4. **Team Alignment**: Everyone follows same sequence
5. **AI-Friendly**: LLM can follow the 5-step protocol mechanically

## Consequences

### Positive
- **100% Integration Coverage**: Every Port has a Consumer and Daemon wiring
- **No Dead Code**: Warnings become errors, forcing cleanup
- **Fast Feedback**: Compilation errors surface immediately
- **Testable by Default**: Integration tests become mandatory

### Negative
- **Initial Slowdown**: Must wire before implementing
- **Breaking Changes**: Step 2 intentionally breaks the build
- **Discipline Required**: Easy to skip steps if not enforced

## Mitigation
- **CI Enforcement**: Add `cargo check` warnings-as-errors
- **PR Template**: Mandatory integration checklist
- **Code Review**: Reviewers verify 5-step sequence
- **Grep CI Check**: Automated verification in CI pipeline

## Examples

### Example 1: Adding Metrics

```bash
# Step 1: Define Port
# crates/core/src/port/metrics.rs
trait MetricsCollector { fn record(&self, name: &str, value: f64); }

# Step 2: Break Consumer
# crates/core/src/application/worker/mod.rs
pub struct Worker {
    metrics: Arc<dyn MetricsCollector>,  // Breaks daemon
}

# Step 3: Wire Daemon
# crates/daemon/src/main.rs
let metrics = Arc::new(PrometheusCollector::new());
let worker = Worker::new(repo, executor, metrics);  // Fixed

# Step 4: Implement Logic
impl Worker {
    pub async fn process(&self) {
        let start = Instant::now();
        // ... process job ...
        self.metrics.record("job_duration", start.elapsed().as_secs_f64());
    }
}

# Step 5: Implement Infra
# crates/infra-metrics/src/prometheus.rs
pub struct PrometheusCollector;
impl MetricsCollector for PrometheusCollector { ... }
```

### Example 2: Adding SystemProbe

```bash
# Step 1: Define Port
trait SystemProbe {
    fn cpu_usage(&self) -> f32;
    fn is_idle(&self) -> bool;
}

# Step 2: Break Consumer (Planner needs it)
pub struct Planner {
    system: Arc<dyn SystemProbe>,  // Breaks daemon
}

# Step 3: Wire Daemon
let system = Arc::new(SysinfoProbe::new());
let planner = Planner::new(repo, system);  // Fixed

# Step 4: Implement Logic
impl Planner {
    pub fn should_schedule(&self, job: &Job) -> bool {
        if job.wait_for_idle && !self.system.is_idle() {
            return false;
        }
        true
    }
}

# Step 5: Implement Infra
pub struct SysinfoProbe { system: System }
impl SystemProbe for SysinfoProbe { ... }
```

## Auto-Verification Protocol (CRITICAL)

### Immediate Verification Requirement
After writing ANY code, the developer/AI MUST:

1. **Run verification commands immediately**:
   ```bash
   # Compile check
   cargo check
   
   # Lint check
   cargo clippy -- -D warnings
   
   # Grep verification (if adding new trait/struct)
   grep -r "YourNewTrait" src/application/
   grep -r "YourNewStruct" src/daemon/
   
   # Test execution
   cargo test
   ```

2. **Read output and fix issues**:
   - Dead code warnings → Wire the code
   - Compilation errors → Fix immediately
   - Test failures → Debug and resolve
   - Clippy warnings → Address all issues

3. **BEFORE responding "Finished"**:
   - ALL checks MUST pass
   - NO warnings allowed
   - NO test failures
   - Grep verification successful

### Constraint
- **DO NOT** ask user to run verification
- **RUN IT YOURSELF** using terminal tools
- **FIX ALL ISSUES** before declaring completion
- **USER SEES** only the final, verified, working code

### Goal
The user should receive:
- ✅ Code that compiles cleanly
- ✅ Code that passes all tests
- ✅ Code with zero warnings
- ✅ Code that is fully integrated

### Anti-Pattern (FORBIDDEN)
```bash
# ❌ BAD: Telling user to verify
"Please run `cargo check` to verify the implementation"

# ❌ BAD: Leaving warnings
"There are some warnings but the code works"

# ❌ BAD: Skipping verification
"The implementation is complete" (without running checks)
```

### Correct Pattern (REQUIRED)
```bash
# ✅ GOOD: Run verification immediately
$ cargo check
   Compiling semantica-task-engine v0.1.0
   Finished dev [unoptimized + debuginfo] target(s) in 2.3s

$ cargo clippy -- -D warnings
   Checking semantica-task-engine v0.1.0
   Finished dev [unoptimized + debuginfo] target(s) in 1.8s

$ grep -r "TaskExecutor" src/application/
src/application/worker/mod.rs:    executor: Arc<dyn TaskExecutor>,
src/application/worker/mod.rs:        self.executor.execute(job)?;

$ cargo test
   Running unittests (target/debug/deps/...)
test result: ok. 25 passed; 0 failed

# NOW respond to user: "Implementation complete and verified"
```

## Summary

**Wire-First Pattern Guarantees**:
1. No trait without usage
2. No implementation without wiring
3. No field without initialization
4. No feature without integration test

**Enforcement**:
- Compiler errors (Step 2)
- Warnings as errors (Rule 1)
- Grep verification (Rule 2)
- Integration tests (Rule 3)
- PR checklist (Rule 4)
- **Auto-verification (immediate)**

**Result**: Zero orphaned code, 100% integration coverage, verified working code.

