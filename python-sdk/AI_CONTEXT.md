# AI Context - SemanticaTask SDK 사용 가이드

**이 문서는 AI/LLM이 SemanticaTask SDK를 정확하게 이해하고 사용할 수 있도록 작성되었습니다.**

---

## 1. SDK 개요

**SemanticaTask SDK**는 비동기 Job Queue 시스템인 **SemanticaTask Engine**과 통신하는 Python 클라이언트입니다.

### 핵심 개념

1. **Daemon** (백그라운드 서비스):
   - Rust로 작성된 Job Queue 엔진
   - 기본 포트: `9527` (환경변수로 변경 가능)
   - JSON-RPC 2.0 프로토콜 사용

2. **Job** (작업 단위):
   - `job_type`: 작업 유형 식별자 (예: "INDEX_FILE")
   - `queue`: 작업이 속한 큐
   - `subject_key`: 중복 방지용 고유 키
   - `payload`: 작업 데이터 (JSON)
   - `priority`: 우선순위 (정수, 높을수록 먼저 실행)

3. **State** (Job 상태):
   - `QUEUED`: 대기 중
   - `RUNNING`: 실행 중
   - `DONE`: 완료
   - `FAILED`: 실패
   - `SUPERSEDED`: 새 Job으로 대체됨

---

## 2. 기본 사용 패턴

### 필수 Import

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest
```

### 기본 구조 (항상 이 패턴 사용)

```python
async def main():
    async with SemanticaTaskClient("http://localhost:9527") as client:
        # client 사용
        response = await client.enqueue(...)
        print(response.job_id)

asyncio.run(main())
```

**중요**: 
- `async with` 필수 (리소스 자동 정리)
- 모든 메서드는 `await` 필요
- URL은 Daemon 주소 (`http://host:port`)

---

## 3. API 메서드 (3개)

### 3.1 `enqueue()` - Job 등록

```python
response = await client.enqueue(
    EnqueueRequest(
        job_type="INDEX_FILE",          # 필수: str
        queue="default",                # 필수: str
        subject_key="repo::file.py",    # 필수: str (중복 방지 키)
        payload={"path": "file.py"},    # 필수: Any (JSON 가능)
        priority=5                      # 선택: int (기본값 0)
    )
)

# response.job_id     -> str (UUID)
# response.state      -> str ("QUEUED")
# response.queue      -> str ("default")
```

**subject_key 규칙**:
- 동일한 `subject_key`로 재등록 시 기존 QUEUED Job을 SUPERSEDED로 변경
- 최신 Job만 실행하고 싶을 때 사용
- 예: `"user_123::send_email"`, `"repo_456::index_file.py"`

### 3.2 `cancel()` - Job 취소

```python
response = await client.cancel("job-uuid-here")

# response.job_id     -> str
# response.cancelled  -> bool (True: 취소됨, False: 이미 완료)
```

**주의**: QUEUED 또는 RUNNING 상태만 취소 가능

### 3.3 `tail_logs()` - 로그 조회

```python
response = await client.tail_logs("job-uuid-here", lines=100)

# response.job_id     -> str
# response.log_path   -> Optional[str]
# response.lines      -> list[str] (로그 라인 배열)
```

---

## 4. 타입 정의

```python
# 요청 타입
@dataclass
class EnqueueRequest:
    job_type: str           # 필수
    queue: str              # 필수
    subject_key: str        # 필수
    payload: Any            # 필수 (dict, list, str, int 등)
    priority: int = 0       # 선택 (기본값: 0)

# 응답 타입
@dataclass
class EnqueueResponse:
    job_id: str             # Job UUID
    state: str              # "QUEUED" | "RUNNING" | "DONE" | "FAILED"
    queue: str              # 큐 이름

@dataclass
class CancelResponse:
    job_id: str
    cancelled: bool

@dataclass
class TailLogsResponse:
    job_id: str
    log_path: Optional[str]
    lines: list[str]
```

---

## 5. 에러 처리

### 에러 타입 2가지

```python
from semantica_task_engine import ConnectionError, RpcError

try:
    async with SemanticaTaskClient() as client:
        response = await client.enqueue(...)

except ConnectionError as e:
    # Daemon 연결 실패
    # e.message: str
    print(f"연결 실패: {e.message}")

except RpcError as e:
    # 서버에서 반환한 에러
    # e.code: int (4xxx: 클라이언트, 5xxx: 서버)
    # e.message: str
    # e.data: Optional[Any]
    print(f"RPC 에러 {e.code}: {e.message}")
```

### 에러 코드

| 코드 범위 | 의미 | 처리 방법 |
|----------|------|----------|
| 4000-4999 | 클라이언트 에러 (잘못된 파라미터) | 파라미터 수정 |
| 5000-5999 | 서버 에러 (DB, 내부 오류) | 재시도 또는 관리자 문의 |

---

## 6. 실전 코드 템플릿

### 템플릿 1: 단일 Job 등록

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest

async def create_job():
    async with SemanticaTaskClient("http://localhost:9527") as client:
        response = await client.enqueue(
            EnqueueRequest(
                job_type="YOUR_JOB_TYPE",
                queue="default",
                subject_key="unique-key-here",
                payload={"key": "value"}
            )
        )
        return response.job_id

job_id = asyncio.run(create_job())
print(f"Job ID: {job_id}")
```

### 템플릿 2: 여러 Job 병렬 등록

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest

async def create_multiple_jobs(items: list):
    async with SemanticaTaskClient() as client:
        tasks = [
            client.enqueue(
                EnqueueRequest(
                    job_type="PROCESS_ITEM",
                    queue="default",
                    subject_key=f"item::{item['id']}",
                    payload=item
                )
            )
            for item in items
        ]
        responses = await asyncio.gather(*tasks)
        return [r.job_id for r in responses]

items = [{"id": 1, "data": "a"}, {"id": 2, "data": "b"}]
job_ids = asyncio.run(create_multiple_jobs(items))
```

### 템플릿 3: 로그 조회 및 모니터링

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient

async def monitor_job(job_id: str):
    async with SemanticaTaskClient() as client:
        while True:
            logs = await client.tail_logs(job_id, lines=10)
            
            for line in logs.lines:
                print(f"[LOG] {line}")
            
            # 완료 체크
            if any("DONE" in line or "FAILED" in line for line in logs.lines):
                break
            
            await asyncio.sleep(2)

asyncio.run(monitor_job("your-job-id"))
```

### 템플릿 4: 에러 처리 포함

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest, ConnectionError, RpcError

async def safe_job():
    try:
        async with SemanticaTaskClient() as client:
            response = await client.enqueue(
                EnqueueRequest(
                    job_type="TEST",
                    queue="default",
                    subject_key="test-1",
                    payload={}
                )
            )
            print(f"✅ Job: {response.job_id}")
    
    except ConnectionError:
        print("❌ Daemon 연결 실패. Daemon이 실행 중인지 확인하세요.")
    
    except RpcError as e:
        if 4000 <= e.code < 5000:
            print(f"❌ 잘못된 요청: {e.message}")
        else:
            print(f"❌ 서버 에러: {e.message}")

asyncio.run(safe_job())
```

---

## 7. 자주 하는 실수 (AI 주의사항)

### ❌ 실수 1: `async with` 없이 사용

```python
# ❌ 잘못됨
client = SemanticaTaskClient()
response = await client.enqueue(...)  # 에러!

# ✅ 올바름
async with SemanticaTaskClient() as client:
    response = await client.enqueue(...)
```

### ❌ 실수 2: `await` 빠뜨림

```python
# ❌ 잘못됨
response = client.enqueue(...)  # SyntaxError

# ✅ 올바름
response = await client.enqueue(...)
```

### ❌ 실수 3: 필수 파라미터 누락

```python
# ❌ 잘못됨 (subject_key 누락)
EnqueueRequest(
    job_type="TEST",
    queue="default",
    payload={}
)

# ✅ 올바름
EnqueueRequest(
    job_type="TEST",
    queue="default",
    subject_key="test-1",  # 필수!
    payload={}
)
```

### ❌ 실수 4: 동기 함수에서 비동기 호출

```python
# ❌ 잘못됨
def my_function():
    async with SemanticaTaskClient() as client:  # 에러!
        ...

# ✅ 올바름
async def my_function():
    async with SemanticaTaskClient() as client:
        ...

asyncio.run(my_function())
```

---

## 8. 환경별 설정

### 로컬 개발

```python
async with SemanticaTaskClient("http://localhost:9527") as client:
    ...
```

### Docker Compose

```python
import os

url = os.getenv("SEMANTICA_RPC_URL", "http://semantica-daemon:9527")
async with SemanticaTaskClient(url) as client:
    ...
```

```yaml
# docker-compose.yml
services:
  your-app:
    environment:
      - SEMANTICA_RPC_URL=http://semantica:9527
```

### 커스텀 포트

```python
async with SemanticaTaskClient("http://localhost:7701") as client:
    ...
```

---

## 9. JSON-RPC 프로토콜 (내부 동작)

SDK 내부적으로 다음과 같은 JSON-RPC 요청을 보냄:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "dev.enqueue.v1",
  "params": {
    "job_type": "INDEX_FILE",
    "queue": "default",
    "subject_key": "test",
    "payload": {"key": "value"},
    "priority": 0
  }
}
```

응답:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "job_id": "c4b2bb3a-72f0-4f1e-8f6b-3aa95b2e18c3",
    "state": "QUEUED",
    "queue": "default"
  }
}
```

**AI는 이 프로토콜을 알 필요 없음. SDK가 자동 처리함.**

---

## 10. 디버깅 팁

### 연결 테스트

```bash
# Daemon 실행 확인
lsof -i :9527

# 또는 curl
curl -X POST http://localhost:9527 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"admin.stats.v1","params":{}}'
```

### 로깅 활성화

```python
import logging
logging.basicConfig(level=logging.DEBUG)
```

---

## 11. 요약 (AI 체크리스트)

코드 작성 시 다음을 확인:

- [ ] `async with SemanticaTaskClient(...) as client:` 사용
- [ ] 모든 메서드에 `await` 붙임
- [ ] `EnqueueRequest` 필수 파라미터 4개 제공 (job_type, queue, subject_key, payload)
- [ ] `asyncio.run()` 또는 async 함수 내에서 실행
- [ ] 에러 처리 (`ConnectionError`, `RpcError`)
- [ ] Daemon이 실행 중인지 확인 (http://localhost:9527)

---

**문서 버전**: 1.0  
**SDK 버전**: 0.1.0  
**마지막 업데이트**: 2025-12-05

