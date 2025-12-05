# SemanticaTask SDK - AI Quick Start Guide

**AI/LLM이 5분 안에 SDK를 사용할 수 있도록 작성된 가이드**

---

## ⚡ 1분 요약

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest

async def main():
    async with SemanticaTaskClient("http://localhost:9527") as client:
        # Job 등록
        response = await client.enqueue(
            EnqueueRequest(
                job_type="INDEX_FILE",
                queue="default",
                subject_key="src/main.py",
                payload={"path": "src/main.py"}
            )
        )
        print(f"Job ID: {response.job_id}")

asyncio.run(main())
```

**필수 전제조건**: Daemon이 `http://localhost:9527`에서 실행 중이어야 함.

---

## 📋 체크리스트 (순서대로 실행)

### Step 1: Daemon 실행 확인

```bash
# 포트 확인
lsof -i :9527

# 또는 curl로 테스트
curl http://localhost:9527
```

**Daemon이 없으면**:
```bash
# Rust 프로젝트 루트에서
cargo run --package semantica-daemon

# 또는 Docker
docker run -p 9527:9527 semantica-task-engine
```

### Step 2: SDK 설치

```bash
pip install semantica-task-engine

# 또는 로컬 개발
cd python-sdk && pip install -e .
```

### Step 3: 코드 작성

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest

async def test():
    async with SemanticaTaskClient() as client:
        response = await client.enqueue(
            EnqueueRequest(
                job_type="TEST",
                queue="default",
                subject_key="test-1",
                payload={"message": "hello"}
            )
        )
        print(f"✅ Job ID: {response.job_id}")

asyncio.run(test())
```

### Step 4: 실행

```bash
python your_script.py
```

---

## 🎯 핵심 패턴 (복사해서 사용)

### 패턴 1: 단일 Job 등록

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest

async def enqueue_job():
    async with SemanticaTaskClient("http://localhost:9527") as client:
        response = await client.enqueue(
            EnqueueRequest(
                job_type="INDEX_FILE",       # Job 타입
                queue="default",             # 큐 이름
                subject_key="file::main.py", # 중복 방지 키
                payload={                    # 데이터 (dict)
                    "path": "src/main.py",
                    "repo_id": "my-repo"
                },
                priority=5                   # 우선순위 (선택)
            )
        )
        return response.job_id

job_id = asyncio.run(enqueue_job())
print(f"Job ID: {job_id}")
```

### 패턴 2: 여러 Job 병렬 등록

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest

async def enqueue_multiple(files: list[str]):
    async with SemanticaTaskClient() as client:
        tasks = [
            client.enqueue(
                EnqueueRequest(
                    job_type="INDEX_FILE",
                    queue="default",
                    subject_key=f"file::{file}",
                    payload={"path": file}
                )
            )
            for file in files
        ]
        responses = await asyncio.gather(*tasks)
        return [r.job_id for r in responses]

job_ids = asyncio.run(enqueue_multiple([
    "src/main.py",
    "src/utils.py",
    "tests/test_main.py"
]))
print(f"등록된 Job 수: {len(job_ids)}")
```

### 패턴 3: 로그 조회

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient

async def get_logs(job_id: str):
    async with SemanticaTaskClient() as client:
        response = await client.tail_logs(job_id, lines=100)
        return response.lines

logs = asyncio.run(get_logs("your-job-id-here"))
for line in logs:
    print(line)
```

### 패턴 4: Job 취소

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient

async def cancel_job(job_id: str):
    async with SemanticaTaskClient() as client:
        response = await client.cancel(job_id)
        return response.cancelled

cancelled = asyncio.run(cancel_job("your-job-id-here"))
print(f"취소됨: {cancelled}")
```

### 패턴 5: 에러 처리

```python
import asyncio
from semantica_task_engine import (
    SemanticaTaskClient,
    EnqueueRequest,
    ConnectionError,
    RpcError
)

async def safe_enqueue():
    try:
        async with SemanticaTaskClient() as client:
            response = await client.enqueue(
                EnqueueRequest(
                    job_type="TEST",
                    queue="default",
                    subject_key="test",
                    payload={}
                )
            )
            return response.job_id
    except ConnectionError as e:
        print(f"❌ Daemon 연결 실패: {e.message}")
        return None
    except RpcError as e:
        print(f"❌ RPC 에러 {e.code}: {e.message}")
        return None

job_id = asyncio.run(safe_enqueue())
```

---

## 🔑 핵심 파라미터 설명

### `EnqueueRequest` 필드

```python
EnqueueRequest(
    job_type="INDEX_FILE",          # 필수: Job 타입 (문자열, 자유 형식)
    queue="default",                # 필수: 큐 이름 (문자열)
    subject_key="repo::file.py",    # 필수: 중복 방지 키 (문자열)
    payload={"key": "value"},       # 필수: JSON 직렬화 가능한 데이터
    priority=0                      # 선택: 정수 (높을수록 우선, 기본값 0)
)
```

**subject_key 규칙**:
- 동일한 `subject_key`로 다시 등록하면 기존 QUEUED Job을 SUPERSEDED로 변경
- 형식 예시: `"user_id::action"`, `"repo_id::file_path"`
- 파일별, 사용자별로 최신 Job만 실행하고 싶을 때 사용

---

## 🚨 흔한 에러 & 해결법

### 에러 1: `ConnectionError: Connection refused`

**원인**: Daemon이 실행 중이 아님

**해결**:
```bash
# 1. Daemon 실행
cargo run --package semantica-daemon

# 2. 포트 확인
lsof -i :9527
```

### 에러 2: `Client not initialized. Use 'async with' context manager.`

**원인**: `async with` 없이 사용

**해결**:
```python
# ❌ 잘못된 사용
client = SemanticaTaskClient()
await client.enqueue(...)  # 에러!

# ✅ 올바른 사용
async with SemanticaTaskClient() as client:
    await client.enqueue(...)
```

### 에러 3: `RpcError 4001: Invalid parameters`

**원인**: 필수 파라미터 누락 또는 잘못된 타입

**해결**:
```python
# job_type, queue, subject_key, payload는 필수!
EnqueueRequest(
    job_type="TEST",       # ✅ 문자열
    queue="default",       # ✅ 문자열
    subject_key="key-1",   # ✅ 문자열
    payload={"data": 1}    # ✅ dict (JSON 가능)
)
```

---

## 🎨 실전 시나리오

### 시나리오 1: 파일 인덱싱 자동화

```python
import asyncio
from pathlib import Path
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest

async def index_project(project_path: str):
    files = list(Path(project_path).rglob("*.py"))
    
    async with SemanticaTaskClient() as client:
        for file in files:
            response = await client.enqueue(
                EnqueueRequest(
                    job_type="INDEX_FILE",
                    queue="code_intel",
                    subject_key=f"project::{file}",
                    payload={
                        "path": str(file),
                        "language": "python"
                    },
                    priority=10 if "test" not in str(file) else 0
                )
            )
            print(f"✅ {file.name} -> {response.job_id}")

asyncio.run(index_project("./src"))
```

### 시나리오 2: 사용자별 알림 Job

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest

async def send_notification(user_id: str, message: str):
    async with SemanticaTaskClient() as client:
        response = await client.enqueue(
            EnqueueRequest(
                job_type="SEND_NOTIFICATION",
                queue="notifications",
                subject_key=f"user::{user_id}::notification",
                payload={
                    "user_id": user_id,
                    "message": message,
                    "timestamp": "2025-12-05T10:00:00Z"
                },
                priority=100  # 높은 우선순위
            )
        )
        return response.job_id

job_id = asyncio.run(send_notification("user-123", "Hello!"))
```

### 시나리오 3: Job 상태 모니터링

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient

async def wait_for_completion(job_id: str, max_wait: int = 60):
    """Job이 완료될 때까지 대기 (최대 max_wait초)"""
    async with SemanticaTaskClient() as client:
        for _ in range(max_wait // 2):
            logs = await client.tail_logs(job_id, lines=5)
            
            # 완료 체크
            if any("DONE" in line for line in logs.lines):
                print(f"✅ Job {job_id} 완료!")
                return True
            
            if any("FAILED" in line for line in logs.lines):
                print(f"❌ Job {job_id} 실패!")
                return False
            
            await asyncio.sleep(2)
        
        print(f"⏱️ Job {job_id} 타임아웃")
        return False

completed = asyncio.run(wait_for_completion("your-job-id"))
```

---

## 🧩 환경별 설정

### 개발 환경 (localhost)

```python
async with SemanticaTaskClient("http://localhost:9527") as client:
    ...
```

### 프로덕션 환경 (Docker)

```python
import os

url = os.getenv("SEMANTICA_RPC_URL", "http://semantica-daemon:9527")
async with SemanticaTaskClient(url) as client:
    ...
```

```bash
# 환경변수 설정
export SEMANTICA_RPC_URL=http://semantica-daemon:9527

# Docker Compose
services:
  your-app:
    environment:
      - SEMANTICA_RPC_URL=http://semantica:9527
```

### 커스텀 포트 (7701)

```python
async with SemanticaTaskClient("http://localhost:7701") as client:
    ...
```

---

## 📚 더 알아보기

- [전체 API 문서](./README.md)
- [에러 코드 목록](./README.md#%EF%B8%8F-에러-처리)
- [타입 정의](./README.md#-타입-정보-aillm용)

---

**빠른 도움말**:
- Daemon 연결 안 됨 → `lsof -i :9527` 확인
- `async with` 필수 → 모든 메서드는 `await`
- `subject_key` → 중복 방지용 고유 키
- `payload` → JSON 직렬화 가능한 dict/list/str/int

