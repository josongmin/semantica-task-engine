# API Contract (SSOT: ADR-020)

## 절대 원칙
**ADR-020이 JSON-RPC API의 유일한 진실 공급원**
Breaking change는 반드시 버전 업 필요

## Transport Layer

### macOS/Linux
- Unix Domain Socket (UDS)
- Path: `~/.semantica/semantica.sock`

### Windows
- Named Pipe
- Path: `\\.\pipe\semantica-orchestrator`

### Fallback (Dev/Debug)
- TCP Loopback: `127.0.0.1:42069`

## JSON-RPC 2.0 Envelope

### Request
```json
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "method": "dev.enqueue.v1",
  "params": { ... }
}
```

### Response (Success)
```json
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "result": { ... }
}
```

### Response (Error)
```json
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "error": {
    "code": 4000,
    "message": "Validation Error",
    "data": {
      "kind": "missing_field",
      "details": "field 'subject_key' is required",
      "trace_id": "trace-abc-123"
    }
  }
}
```

## Error Model

### Error Code Ranges
- **4000-4999**: Client Errors (호출자가 수정 가능)
- **5000-5999**: Server Errors (재시도 또는 치명적)

### Standard Error Codes

| Code | Mnemonic | Description | Action |
|------|----------|-------------|--------|
| 4000 | VALIDATION_ERROR | 스키마 위반, 필수 필드 누락 | Payload 수정 |
| 4001 | NOT_FOUND | Job ID, Tag, Group 없음 | ID 확인 |
| 4002 | CONFLICT | 잘못된 상태 전환, 중복 키 | 상태 해결 |
| 4003 | THROTTLED | Rate limit, Backpressure 활성 | Backoff & Retry |
| 5000 | INTERNAL_ERROR | 예상치 못한 panic/버그 | 버그 리포트 |
| 5001 | DB_ERROR | SQLite IO, Corruption, Lock timeout | Disk/DB 확인 |
| 5002 | SYSTEM_ERROR | OS 리소스 고갈 (파일, RAM) | 리소스 확보 |

## API Methods

### dev.enqueue.v1
Job을 큐에 추가

**Params:**
```typescript
interface EnqueueParams {
  job_type: string;          // "INDEX_FILE", "RUN_TEST"
  queue: string;             // "code_intel", "build"
  subject_key: string;       // "<client>::<repo>::<path>"
  
  payload: any;              // JSON 직렬화 가능 객체
  priority?: number;         // Default 0
  
  // Scheduling
  schedule?: {
    type: "IMMEDIATE" | "AT" | "AFTER" | "CONDITION";
    scheduled_at?: number;   // Epoch ms
    delay_ms?: number;
  };
  
  // Conditions (Phase 3+)
  conditions?: {
    wait_for_idle?: boolean;
    require_charging?: boolean;
    wait_for_event?: string; // "git_commit"
  };
  
  // Traceability & UX
  tag?: string;              // "nightly"
  chain_group_id?: string;   // Workflow session ID
}
```

**Result:**
```typescript
interface EnqueueResult {
  job_id: string;            // UUID v4
  queue: string;
  state: "QUEUED" | "SCHEDULED";
}
```

### dev.cancel.v1
ID, Tag, 또는 Group으로 Job 취소

**Params:**
```typescript
interface CancelParams {
  job_id?: string;
  tag?: string;
  chain_group_id?: string;
}
// 최소 하나 필드 필수
```

**Result:**
```typescript
interface CancelResult {
  cancelled_count: number;
}
```

### dev.query_jobs.v1
유연한 검색 + 커서 기반 페이지네이션

**Params:**
```typescript
interface QueryParams {
  filter: {
    state?: JobState[];
    queue?: string[];
    tag?: string;
    chain_group_id?: string;
    subject_key_prefix?: string;
    created_after?: number;
  };
  sort?: "ASC" | "DESC";     // Default DESC
  limit?: number;            // Default 50, Max 200
  cursor?: string;           // Base64 불투명 커서
}
```

**Result:**
```typescript
interface QueryResult {
  items: JobView[];
  next_cursor: string | null;
}
```

### logs.tail.v1
Job의 로그 출력 스트리밍

**Params:**
```typescript
interface TailParams {
  job_id: string;
  offset: number;            // Byte offset
  limit?: number;            // Max bytes to read
}
```

**Result:**
```typescript
interface TailResult {
  chunk: string;             // UTF-8 string
  next_offset: number;
  eof: boolean;              // Job DONE + 파일 끝
}
```

### admin.stats.v1
시스템 상태 및 큐 메트릭 조회

**Params:** `{}`

**Result:**
```typescript
interface StatsResult {
  queues: Array<{
    name: string;
    queued: number;
    running: number;
    failed: number;
    avg_wait_ms: number;
  }>;
  system: {
    cpu_usage: number;
    memory_usage: number;
    is_idle: boolean;
    db_wal_size: number;
  };
}
```

## SDK Interface Guidelines

### 일반 원칙
1. **Type Safety**: Rust DTO에서 타입 생성 (가능한 경우)
2. **Traceability**: 요청마다 trace_id 자동 생성
3. **Connection Management**: 자동 재연결 로직

### Python SDK
```python
class SemanticaClient:
    def enqueue(self, job_type: str, queue: str, subject_key: str, 
                payload: dict, **kwargs) -> str:
        """Returns job_id"""
        ...
    
    def cancel(self, tag: str = None, job_id: str = None) -> int:
        ...

    def tail_logs(self, job_id: str, follow: bool = True) -> Generator[str, None, None]:
        ...
```

### TypeScript SDK
```typescript
class SemanticaClient {
  enqueue(params: EnqueueParams): Promise<string>; // Returns job_id
  cancel(params: CancelParams): Promise<number>;
  query(params: QueryParams): Promise<QueryResult>;
}
```

## Contract Test 필수 사항

1. **Schema Validation**: 모든 DTO는 JSON Schema로 검증
2. **Error Code Coverage**: 모든 에러 코드 테스트 케이스 존재
3. **Backward Compatibility**: API 변경 시 이전 버전과의 호환성 검증
4. **SDK Parity**: Python/TS SDK가 동일한 동작 보장