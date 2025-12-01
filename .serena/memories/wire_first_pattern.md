# Wire-First, Implement-Second Pattern

## Problem: Orphaned Code
Common failure mode where traits/implementations are defined but never wired into the system.

## Solution: 5-Step Protocol (STRICT ORDER)

### Step 1: Define Port
Define trait in `crates/core/src/port/`

### Step 2: Break the Consumer
- Add trait as field to consuming struct
- Update constructor signature
- **CHECKPOINT**: `cargo check` MUST fail

### Step 3: Wire the Daemon
- Go to `crates/daemon/src/main.rs`
- Inject implementation (even if placeholder)
- **CHECKPOINT**: `cargo check` now passes

### Step 4: Implement Logic
- Use the field in actual logic
- **CHECKPOINT**: No "field is never read" warning

### Step 5: Implement Infra
- Write concrete implementation in `infra-*`

## Enforcement Rules

### Rule 1: Compiler as Gatekeeper
- NEVER use `#[allow(dead_code)]`
- Treat these as CRITICAL ERRORS:
  - `warning: field is never read`
  - `warning: unused import`
  - `warning: associated function is never used`

### Rule 2: Grep Proof Verification
```bash
grep -r "YourNewTrait" crates/core/src/application/
grep -r "YourNewImpl" crates/daemon/src/
cargo check 2>&1 | grep "never read\|never used"  # Must be empty
```

### Rule 3: Integration Test Required
Every new Port/Infra pair needs integration test proving they work together.

### Rule 4: PR Checklist
- [ ] Port defined
- [ ] Consumer updated (caused error)
- [ ] Daemon wired (error fixed)
- [ ] Logic implemented (field used)
- [ ] Infra implemented
- [ ] No dead code warnings
- [ ] Grep verification passed
- [ ] Integration test added

## Benefits
- Zero orphaned code
- Compiler-enforced integration
- Clear progress checkpoints
- AI-friendly mechanical process
