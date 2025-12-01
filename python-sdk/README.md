# SemanticaTask Python SDK

Python client library for [SemanticaTask Engine](../README.md).

## Installation

```bash
# From Git
pip install git+https://github.com/<username>/semantica-task-engine#subdirectory=python-sdk

# Local development
cd python-sdk
pip install -e .
```

## Usage

### Basic Example

```python
import asyncio
from semantica import SemanticaTaskClient, EnqueueRequest

async def main():
    # Connect to daemon
    async with SemanticaTaskClient("http://127.0.0.1:9527") as client:
        # Enqueue a job
        response = await client.enqueue(
            EnqueueRequest(
                job_type="INDEX_FILE",
                queue="default",
                subject_key="src/main.py",
                payload={"path": "src/main.py"},
                priority=5,
            )
        )
        
        print(f"Job ID: {response.job_id}")
        print(f"State: {response.state}")
        
        # Tail logs
        logs = await client.tail_logs(response.job_id, lines=100)
        for line in logs.lines:
            print(line)
        
        # Cancel job
        cancel_resp = await client.cancel(response.job_id)
        if cancel_resp.cancelled:
            print("Job cancelled")

if __name__ == "__main__":
    asyncio.run(main())
```

## API

### `SemanticaTaskClient`

```python
async with SemanticaTaskClient("http://127.0.0.1:9527") as client:
    # Enqueue
    response = await client.enqueue(EnqueueRequest(...))
    
    # Cancel
    response = await client.cancel("job-123")
    
    # Tail logs
    response = await client.tail_logs("job-123", lines=100)
```

## Error Handling

```python
from semantica import SemanticaTaskClient, ConnectionError, RpcError

async with SemanticaTaskClient() as client:
    try:
        response = await client.enqueue(request)
    except ConnectionError as e:
        print(f"Connection error: {e}")
    except RpcError as e:
        print(f"RPC error {e.code}: {e.message}")
```

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Format code
black semantica tests

# Lint
ruff check semantica tests
```

## License

MIT

