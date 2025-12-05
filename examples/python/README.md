# Semantica Task Engine - Python SDK

Semantica Task Engine용 Python 클라이언트 라이브러리.

## 빠른 시작 (Docker)

### 1. Daemon + Python 테스트 실행

프로젝트 루트에서:

```bash
# Daemon만 실행
docker-compose -f docker-compose.dev.yml up daemon

# Daemon + Python 통합 테스트
docker-compose -f docker-compose.dev.yml --profile test up
```

### 2. 로컬에서 테스트

```bash
# 터미널 1: Daemon 실행
cd ../..
cargo run --package semantica-daemon

# 터미널 2: Python 테스트
cd examples/python
pip install -r requirements.txt
python test_integration.py
```

## 사용법

### 기본 예제

```python
from semantica_client import SemanticaTaskClient

# 클라이언트 생성
client = SemanticaTaskClient("http://127.0.0.1:9527")

# Job 등록
response = client.enqueue(
    job_type="INDEX_FILE",
    queue="default",
    subject_key="src/main.py",
    payload={"path": "src/main.py", "repo_id": "my-project"},
    priority=0
)
print(f"Job ID: {response['job_id']}")

# 로그 확인
logs = client.tail_logs(response['job_id'], limit=50)
for line in logs['lines']:
    print(line)

# 통계 확인
stats = client.stats()
print(stats)

# Job 취소
client.cancel(response['job_id'])
```

### 에러 처리

```python
from semantica_client import (
    SemanticaTaskClient,
    SemanticaConnectionError,
    SemanticaRpcError
)

try:
    client = SemanticaTaskClient()
    response = client.enqueue(...)
except SemanticaConnectionError as e:
    print(f"연결 실패: {e}")
except SemanticaRpcError as e:
    print(f"RPC 에러 (코드 {e.code}): {e.message}")
```

## 환경변수

- `SEMANTICA_RPC_URL`: RPC 엔드포인트 (기본값: `http://127.0.0.1:9527`)

## Python 프로젝트 통합

### 1. SDK 복사

```bash
# semantica_client.py를 프로젝트에 복사
cp semantica_client.py /path/to/your/project/
```

### 2. 의존성 추가

**requirements.txt**:
```
requests>=2.31.0
```

**pyproject.toml** (Poetry):
```toml
[tool.poetry.dependencies]
requests = "^2.31.0"
```

### 3. 코드에서 사용

```python
from semantica_client import SemanticaTaskClient

client = SemanticaTaskClient()
# ... use client
```

## API Reference

### `SemanticaTaskClient`

#### `__init__(url: str = None)`
- `url`: RPC 엔드포인트 (기본값: env `SEMANTICA_RPC_URL` 또는 `http://127.0.0.1:9527`)

#### `enqueue(job_type, queue, subject_key, payload, priority=0)`
- Job 등록
- Returns: `{"job_id": "...", "queue": "...", "state": "QUEUED"}`

#### `cancel(job_id: str)`
- Job 취소
- Returns: `{"job_id": "...", "state": "CANCELLED"}`

#### `tail_logs(job_id: str, limit: int = 50)`
- 로그 조회
- Returns: `{"lines": [...]}`

#### `stats()`
- 시스템 통계
- Returns: `{"queues": [...]}`

## 라이선스

MIT


