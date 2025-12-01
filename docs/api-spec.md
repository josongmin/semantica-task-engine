# Semantica Orchestrator JSON-RPC API Spec

본 문서는 Semantica Orchestrator의 **JSON-RPC 2.0 API**와  
Python/TS SDK가 공유하는 **공식 계약(Contract)** 을 정의한다.

- Transport: UDS / Named Pipe / TCP (개발용)
- Protocol: JSON-RPC 2.0
- Method 네임스페이스:
  - `dev.*.v1`: 개발 워크플로우 관련 기능
  - `logs.*.v1`: 로그 조회
  - `admin.*.v1`: 상태/관리

---

## 1. 공통 규칙

### 1.1. JSON-RPC Envelope

요청:
```json
{
  "jsonrpc": "2.0",
  "id": "<string|number>",
  "method": "dev.enqueue.v1",
  "params": { ... }
}
```

응답(성공):
```json
{
  "jsonrpc": "2.0",
  "id": "<same-as-request>",
  "result": { ... }
}
```

응답(실패):
```json
{
  "jsonrpc": "2.0",
  "id": "<same-as-request>",
  "error": {
    "code": <int>,
    "message": "<short-description>",
    "data": {
      "kind": "validation|not_found|internal|...",
      "details": "..."
    }
  }
}
```

### 1.2. 공통 타입

```typescript
type JobId = string;  // UUID v4
type UserTag = string;
type ChainGroupId = string;
type ScheduleType = "IMMEDIATE" | "AT" | "AFTER" | "CONDITION";
```

---

## 2. dev.enqueue.v1

### 2.1. 개요
Dev 작업(Job)을 큐에 등록한다.

### 2.2. Request
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "dev.enqueue.v1",
  "params": {
    "job_type": "INDEX_FILE_DELTA",
    "queue": "code_intel",
    "subject_key": "vscode::repo123::src/app/main.py",
    "payload": {
      "repo_id": "repo123",
      "path": "src/app/main.py"
    },
    "priority": 0
  }
}
```

### 2.3. Response
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "job_id": "c4b2bb3a-72f0-4f1e-8f6b-3aa95b2e18c3",
    "queue": "code_intel",
    "state": "QUEUED"
  }
}
```

---

## 3. admin.stats.v1

### 3.1. 개요
큐/워커/시스템 상태 통계 조회

### 3.2. Response 예시
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "queues": [
      {
        "name": "code_intel",
        "queued": 12,
        "running": 2,
        "failed": 1,
        "superseded": 5
      }
    ]
  }
}
```

---

더 많은 API는 Phase 2+ 에서 추가됩니다.

