# Database Policy (SSOT: ADR-010)

## 절대 규칙
**ADR-010이 스키마의 유일한 진실 공급원(Single Source of Truth)**
다른 ADR의 스키마 정의는 예시일 뿐, ADR-010과 충돌 시 ADR-010이 우선

## SQLite 설정

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 200;
```

- WAL: 1 Writer + N Readers 동시성
- synchronous=NORMAL: OS 크래시 안전, FULL보다 빠름
- busy_timeout=200ms: Lock 경합 시 빠른 실패 (infra-retry 트리거)

## 핵심 테이블

### jobs
```sql
CREATE TABLE jobs (
  -- Identity
  id TEXT PRIMARY KEY,
  trace_id TEXT,  -- Phase 2+
  
  -- Classification
  queue TEXT NOT NULL,
  job_type TEXT NOT NULL,
  subject_key TEXT NOT NULL,  -- <client>::<repo>::<path>
  generation INTEGER NOT NULL,
  
  -- State & Timing
  priority INTEGER DEFAULT 0,
  state TEXT NOT NULL CHECK(state IN (
    'QUEUED', 'SCHEDULED', 'RUNNING', 'DONE', 
    'FAILED', 'CANCELLED', 'SUPERSEDED', 
    'SKIPPED_TTL', 'SKIPPED_DEADLINE'
  )),
  created_at INTEGER NOT NULL,
  started_at INTEGER,
  finished_at INTEGER,
  
  -- Execution (Phase 2+)
  execution_mode TEXT CHECK(execution_mode IN ('IN_PROCESS', 'SUBPROCESS')),
  pid INTEGER,
  env_vars TEXT,  -- JSON
  
  -- Resilience (Phase 2+)
  attempts INTEGER DEFAULT 0,
  max_attempts INTEGER DEFAULT 0,
  backoff_factor REAL DEFAULT 2.0,
  deadline INTEGER,
  ttl_ms INTEGER,
  
  -- Scheduling (Phase 3+)
  schedule_type TEXT CHECK(schedule_type IN ('IMMEDIATE', 'AT', 'AFTER', 'CONDITION')),
  scheduled_at INTEGER,
  schedule_delay_ms INTEGER,
  wait_for_idle INTEGER DEFAULT 0,
  require_charging INTEGER DEFAULT 0,
  wait_for_event TEXT,
  
  -- Grouping (Phase 4)
  parent_job_id TEXT,
  chain_group_id TEXT,
  user_tag TEXT,
  
  -- Outputs
  payload TEXT NOT NULL,
  result_summary TEXT,
  log_path TEXT,
  artifact_path TEXT
);
```

### job_conditions (Phase 3+)
```sql
CREATE TABLE job_conditions (
  job_id TEXT NOT NULL,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY (job_id, key),
  FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
);
```

## 인덱스 전략 (성능 크리티컬)

| 인덱스 이름 | 컬럼 | 목적 |
|-------------|------|------|
| `idx_jobs_pop` | `(queue, priority DESC, created_at ASC, id)` | ⚡ Worker Pop (O(1)) |
| `idx_jobs_subject_generation` | `(subject_key, generation DESC)` | ⚡ Supersede 로직 |
| `idx_jobs_state_queue` | `(state, queue)` | Admin Stats |
| `idx_jobs_gc` | `(finished_at)` | GC (오래된 기록 제거) |
| `idx_jobs_user_tag` | `(user_tag)` | cancel(tag=...) |
| `idx_job_conditions_lookup` | `(key, value, job_id)` | 이벤트 트리거 조회 |

## Phase별 스키마 진화

| 필드 범주 | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|-----------|---------|---------|---------|---------|
| Identity | id, queue, job_type | trace_id | - | - |
| Supersede | subject_key, generation | - | - | - |
| State | state, created_at | - | - | - |
| Execution | payload, log_path | execution_mode, pid, env_vars | - | result_summary, artifact_path |
| Retry/Life | - | attempts, max, deadline, ttl | - | - |
| Scheduling | - | - | schedule_*, wait_for_*, job_conditions | - |
| Grouping | - | - | - | parent_id, chain_group, user_tag |

## 마이그레이션 정책

### 디렉토리 구조
```
migrations/
  001_initial_schema.sql      # Phase 1
  002_add_execution_retry.sql # Phase 2
  003_add_conditions.sql      # Phase 3
  004_add_dx_fields.sql        # Phase 4
```

### 규칙
1. Startup Check: schema_version 확인
2. Sequential Apply: 순서대로 적용
3. Fail-Stop: 마이그레이션 실패 시 데몬 시작 중단
4. Immutable: 릴리스된 마이그레이션 파일 절대 수정 금지

## 제약조건

### CHECK Constraints
- state: 정해진 enum 값만 허용 (zombie state 방지)
- execution_mode: IN_PROCESS | SUBPROCESS
- schedule_type: IMMEDIATE | AT | AFTER | CONDITION

### Foreign Keys
- job_conditions.job_id → jobs.id (CASCADE DELETE)
- parent_job_id → jobs.id (향후)

## 성능 고려사항

### Connection Pool
- Writer: 1 (Single Writer 보장)
- Readers: N (WAL 모드)
- busy_timeout: 200ms (빠른 실패)

### 쿼리 패턴
- Pop: `WHERE queue=? ORDER BY priority DESC, created_at ASC LIMIT 1`
- Supersede: `WHERE subject_key=? ORDER BY generation DESC LIMIT 1`
- GC: `DELETE FROM jobs WHERE finished_at < ?`