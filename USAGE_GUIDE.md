# Semantica Task Engine - AI Agent Usage Guide

**Target Audience**: AI Agents (Claude, GPT, etc.)  
**Goal**: Enable AI agents to queue and manage long-running tasks efficiently

---

## 📖 What is Semantica?

Semantica is a **local task orchestrator daemon** that manages background jobs for AI-powered developer tools.

**Core Capabilities**:
- ✅ Queue tasks (indexing, analysis, builds) without blocking the main workflow
- ✅ Prevent duplicate work with smart superseding (same subject_key = only latest runs)
- ✅ Schedule tasks (run now, run later, run when idle)
- ✅ Retrieve logs and results asynchronously
- ✅ Cancel jobs mid-execution

**Architecture**:
```
┌─────────────┐
│  AI Agent   │ (You)
│  (Python)   │
└──────┬──────┘
       │ HTTP JSON-RPC
       ↓
┌─────────────┐
│   Daemon    │ (Runs in background)
│  Port 9527  │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│  SQLite DB  │ (Persistent queue)
└─────────────┘
```

---

## 🚀 Quick Start (5 minutes)

### Step 1: Start the Daemon

```bash
# Option A: Native binary
cargo run --package semantica-daemon

# Option B: Docker
docker-compose up -d

# Verify it's running
curl http://127.0.0.1:9527/health
```

### Step 2: Install Python SDK

```bash
pip install git+https://github.com/<username>/semantica-task-engine#subdirectory=python-sdk
```

### Step 3: Enqueue Your First Job

```python
import asyncio
from semantica import SematicaClient, EnqueueRequest

async def main():
    async with SematicaClient("http://127.0.0.1:9527") as client:
        response = await client.enqueue(
            EnqueueRequest(
                job_type="INDEX_FILE",           # What to do
                queue="code_intel",              # Which worker queue
                subject_key="repo/src/main.py",  # Dedup key
                payload={"path": "src/main.py"}, # Job-specific data
                priority=5,                      # Higher = sooner
            )
        )
        print(f"✓ Job queued: {response.job_id}")

asyncio.run(main())
```

**Output**:
```
✓ Job queued: c4b2bb3a-72f0-4f1e-8f6b-3aa95b2e18c3
```

---

## 🎯 Common Use Cases

### Use Case 1: Index Repository (Smart Superseding)

**Scenario**: User edits `main.py` 10 times in 30 seconds. You don't want 10 index jobs.

**Solution**: Use `subject_key` to auto-cancel old jobs.

```python
async def index_file(file_path: str):
    async with SematicaClient() as client:
        # Same subject_key = only latest job runs
        response = await client.enqueue(
            EnqueueRequest(
                job_type="INDEX_FILE",
                queue="code_intel",
                subject_key=f"vscode::repo123::{file_path}",  # Unique per file
                payload={"repo_id": "repo123", "path": file_path},
            )
        )
        return response.job_id
```

**Key Point**: If you call `index_file("main.py")` 10 times, only the **last enqueue** runs. Previous jobs with the same `subject_key` are auto-superseded.

---

### Use Case 2: Schedule Heavy Task for Later (When Idle)

**Scenario**: User requests full repo reindex, but don't block their typing.

```python
async def schedule_full_reindex():
    async with SematicaClient() as client:
        response = await client.enqueue(
            EnqueueRequest(
                job_type="FULL_REINDEX",
                queue="heavy",
                subject_key="repo123::full_reindex",
                payload={"repo_id": "repo123"},
                priority=-10,  # Low priority
            )
        )
        print(f"Scheduled for idle time: {response.job_id}")
```

**Advanced** (Phase 3+): Add scheduling conditions:
```python
# Wait until system is idle
payload = {
    "repo_id": "repo123",
    "conditions": {
        "wait_for_idle": True,
        "require_charging": False,
    }
}
```

---

### Use Case 3: Retrieve Logs (Real-time)

**Scenario**: User asks "What's the status of my build?"

```python
async def show_job_logs(job_id: str):
    async with SematicaClient() as client:
        logs = await client.tail_logs(job_id, lines=50)
        
        if not logs.lines:
            print("No logs yet (job pending or just started)")
        else:
            print(f"Last {len(logs.lines)} lines:")
            for line in logs.lines:
                print(f"  {line}")
```

**Output**:
```
Last 3 lines:
  [INFO] Indexing started
  [INFO] Processed 1247 files
  [INFO] Indexing completed in 2.3s
```

---

### Use Case 4: Cancel Jobs

**Scenario**: User says "Stop that slow analysis job"

```python
async def cancel_job(job_id: str):
    async with SematicaClient() as client:
        response = await client.cancel(job_id)
        
        if response.cancelled:
            print(f"✓ Job {job_id} cancelled")
        else:
            print(f"Job {job_id} already finished")
```

---

## 🧠 AI Agent Decision Tree

```
User Request: "Index this file"
    ↓
Is it time-sensitive? (< 1s expected)
    ├─ YES → Run inline (don't use Semantica)
    └─ NO  → Enqueue to Semantica
        ↓
    Does it need deduplication?
        ├─ YES → Use subject_key = "file_path"
        └─ NO  → Use subject_key = "unique_id"
        ↓
    Does it need to wait? (e.g., git commit event)
        ├─ YES → Schedule with conditions (Phase 3+)
        └─ NO  → Enqueue with priority
        ↓
    Return job_id to user
    ↓
User asks for status?
    → tail_logs(job_id)
    ↓
User wants to cancel?
    → cancel(job_id)
```

---

## 📋 API Reference (Python SDK)

### `SematicaClient`

```python
async with SematicaClient(url: str = "http://127.0.0.1:9527") as client:
    ...
```

### `enqueue(request: EnqueueRequest) -> EnqueueResponse`

**Parameters**:
- `job_type` (str): Job identifier (e.g., `INDEX_FILE`, `RUN_TEST`, `BUILD_PROJECT`)
- `queue` (str): Worker queue name (e.g., `code_intel`, `build`, `heavy`)
- `subject_key` (str): Deduplication key (format: `<source>::<repo>::<identifier>`)
- `payload` (dict): Job-specific data (JSON serializable)
- `priority` (int): `-100` (lowest) to `100` (highest), default `0`

**Returns**:
- `job_id` (str): UUID of the enqueued job
- `state` (str): `QUEUED` or `SCHEDULED`
- `queue` (str): Assigned queue

**Example**:
```python
response = await client.enqueue(
    EnqueueRequest(
        job_type="INDEX_FILE",
        queue="default",
        subject_key="vscode::repo123::src/main.py",
        payload={"path": "src/main.py", "incremental": True},
        priority=5,
    )
)
# response.job_id = "c4b2bb3a-72f0-4f1e-8f6b-3aa95b2e18c3"
```

---

### `cancel(job_id: str) -> CancelResponse`

**Parameters**:
- `job_id` (str): Job UUID to cancel

**Returns**:
- `job_id` (str): Confirmed job ID
- `cancelled` (bool): `True` if cancelled, `False` if already done

**Example**:
```python
response = await client.cancel("c4b2bb3a-72f0-4f1e-8f6b-3aa95b2e18c3")
# response.cancelled = True
```

---

### `tail_logs(job_id: str, lines: int = 50) -> TailLogsResponse`

**Parameters**:
- `job_id` (str): Job UUID
- `lines` (int): Number of lines to retrieve (default 50, max 1000)

**Returns**:
- `job_id` (str): Confirmed job ID
- `log_path` (str | None): Full path to log file
- `lines` (list[str]): Log lines (newest last)

**Example**:
```python
response = await client.tail_logs("c4b2bb3a-...", lines=100)
for line in response.lines:
    print(line)
```

---

## ⚠️ Error Handling

### Connection Errors

```python
from semantica import SematicaClient, ConnectionError

try:
    async with SematicaClient("http://127.0.0.1:9527") as client:
        response = await client.enqueue(...)
except ConnectionError as e:
    print(f"Daemon not running? {e.message}")
    # Action: Ask user to start daemon or use docker-compose up
```

### RPC Errors

```python
from semantica import RpcError

try:
    response = await client.enqueue(...)
except RpcError as e:
    if e.code == 4000:
        print(f"Validation error: {e.message}")
        # Action: Fix request parameters
    elif e.code == 5000:
        print(f"Internal error: {e.message}")
        # Action: Report bug or retry
```

**Error Code Ranges**:
- `4000-4999`: Client errors (fix your request)
  - `4000`: Validation error (missing field, invalid type)
  - `4001`: Not found (job_id doesn't exist)
  - `4003`: Throttled (too many requests)
- `5000-5999`: Server errors (daemon issue)
  - `5000`: Internal error (bug in daemon)
  - `5001`: Database error (disk full, corruption)

---

## 🎨 Best Practices for AI Agents

### 1. **Use Descriptive `subject_key` Format**

```python
# ✅ GOOD: Hierarchical and specific
subject_key = "vscode::repo123::src/main.py"
subject_key = "cursor::project_foo::test_suite_integration"

# ❌ BAD: Too generic (causes unintended superseding)
subject_key = "file"
subject_key = "build"
```

**Format**: `<tool>::<context>::<identifier>`

---

### 2. **Choose the Right Queue**

| Queue Name   | Use For                        | Concurrency |
|--------------|--------------------------------|-------------|
| `code_intel` | Indexing, LSP, semantic search | High (4-8)  |
| `build`      | Compilation, bundling          | Medium (2-4)|
| `heavy`      | Full repo analysis, tests      | Low (1-2)   |
| `default`    | General tasks                  | Medium (2)  |

```python
# ✅ GOOD: Match task to queue
await client.enqueue(EnqueueRequest(
    job_type="INDEX_FILE",
    queue="code_intel",  # Fast, high concurrency
    ...
))

# ❌ BAD: Heavy task in high-concurrency queue
await client.enqueue(EnqueueRequest(
    job_type="FULL_REINDEX",
    queue="code_intel",  # Will slow down other indexing!
    ...
))
```

---

### 3. **Handle Logs Gracefully**

```python
# User asks: "Show me the build output"
logs = await client.tail_logs(job_id, lines=50)

if not logs.lines:
    return "Job is still starting, no logs yet."
elif "ERROR" in "\n".join(logs.lines):
    return f"Build failed. Last error:\n{logs.lines[-1]}"
else:
    return f"Build in progress. Last line:\n{logs.lines[-1]}"
```

---

### 4. **Set Appropriate Priority**

```python
# User typing (interactive) → High priority
priority = 10

# Background refresh → Normal priority
priority = 0

# Scheduled maintenance → Low priority
priority = -10
```

---

## 🔄 Complete Workflow Example

**Scenario**: User edits `main.py`, wants to see type errors.

```python
import asyncio
from semantica import SematicaClient, EnqueueRequest

async def handle_file_edit(file_path: str, content: str):
    """Called when user saves a file"""
    
    async with SematicaClient() as client:
        # 1. Enqueue type checking
        response = await client.enqueue(
            EnqueueRequest(
                job_type="TYPE_CHECK",
                queue="code_intel",
                subject_key=f"vscode::repo123::{file_path}",
                payload={
                    "file_path": file_path,
                    "content": content,
                },
                priority=5,  # Interactive
            )
        )
        
        job_id = response.job_id
        print(f"Type checking queued: {job_id}")
        
        # 2. Poll for completion (or use webhooks in Phase 4+)
        await asyncio.sleep(2)
        
        # 3. Fetch results
        logs = await client.tail_logs(job_id, lines=100)
        
        # 4. Parse and display
        errors = [line for line in logs.lines if "error:" in line.lower()]
        
        if errors:
            print(f"Found {len(errors)} type errors:")
            for error in errors[:5]:  # Show first 5
                print(f"  - {error}")
        else:
            print("✓ No type errors")

# Simulate user editing
asyncio.run(handle_file_edit("src/main.py", "def foo(): pass"))
```

**Output**:
```
Type checking queued: c4b2bb3a-72f0-4f1e-8f6b-3aa95b2e18c3
✓ No type errors
```

---

## 🐛 Troubleshooting

### Problem: `ConnectionError: Connection refused`

**Cause**: Daemon not running.

**Fix**:
```bash
# Check if daemon is running
ps aux | grep semantica

# Start daemon
cargo run --package semantica-daemon
# or
docker-compose up -d
```

---

### Problem: Jobs not executing

**Cause**: Worker might be stuck or crashed.

**Fix**:
```bash
# Check daemon logs
docker-compose logs -f semantica

# Or if running natively
tail -f ~/.semantica/logs/daemon.log
```

---

### Problem: Duplicate jobs running

**Cause**: `subject_key` not set or too generic.

**Fix**: Use specific `subject_key`:
```python
# ❌ WRONG
subject_key = "index"  # Too generic!

# ✅ RIGHT
subject_key = f"vscode::repo123::{file_path}"
```

---

## 📚 Advanced Topics

### Retry Logic (Phase 2+)

Jobs with transient failures auto-retry with exponential backoff:

```python
# Daemon automatically retries:
# - Attempt 1: immediate
# - Attempt 2: after 2s
# - Attempt 3: after 4s
# - Attempt 4: after 8s
# Max 3 retries, then marked FAILED
```

No SDK changes needed—it's automatic.

---

### Scheduling (Phase 3+)

```python
# Run at specific time
payload = {
    "schedule": {
        "type": "AT",
        "scheduled_at": 1704067200000,  # Epoch ms
    }
}

# Run after delay
payload = {
    "schedule": {
        "type": "AFTER",
        "delay_ms": 60000,  # 1 minute
    }
}

# Run when idle
payload = {
    "schedule": {
        "type": "CONDITION",
    },
    "conditions": {
        "wait_for_idle": True,
    }
}
```

---

## 🎓 Summary for AI Agents

**When to Use Semantica**:
- ✅ Task takes > 1 second
- ✅ Task can be deduplicated (e.g., indexing same file)
- ✅ Task should run in background (don't block user)
- ✅ Task might need cancellation

**When NOT to Use**:
- ❌ Task takes < 100ms (overhead not worth it)
- ❌ Task needs immediate return value (use direct execution)
- ❌ Task is stateless and never duplicated

**Core Pattern**:
```python
# 1. Enqueue
job_id = await client.enqueue(EnqueueRequest(...))

# 2. (Optional) Poll or wait
await asyncio.sleep(1)

# 3. Get logs/results
logs = await client.tail_logs(job_id)

# 4. (Optional) Cancel if needed
await client.cancel(job_id)
```

---

## 📞 Support

- **Documentation**: [ADR_v2/](./ADR_v2/)
- **API Spec**: [docs/api-spec.md](./docs/api-spec.md)
- **Examples**: [python-sdk/examples/](./python-sdk/examples/)
- **Issues**: GitHub Issues (once repo is public)

---

**Version**: 0.1.0 (Phase 3 Complete)  
**Last Updated**: 2024-12-01

