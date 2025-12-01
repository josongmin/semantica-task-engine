# Semantica Task Engine - Project Overview

## 프로젝트 목적
로컬 개발 환경에서 AI Native 개발 작업을 orchestrate하는 고성능 데몬
- 코드 인텔리전스 (인덱싱, 그래프, 임베딩)
- 테스트/빌드
- 문서/스펙 인덱싱
- LLM 보조 작업
을 "지금/나중/조건부"로 실행

## 핵심 특징
1. **로컬 최적화**: 사용자 노트북/워크스테이션 환경
2. **AI Native**: 조건부 실행 (idle일 때, 충전 중, 이벤트 발생 시)
3. **Crash-Resilient**: WAL 모드 SQLite + 자동 복구
4. **타입 안전**: Rust + Hexagonal Architecture
5. **Contract-First**: JSON-RPC + Schema 공유

## 기술 스택 (ADR-001)
- Language: Rust 2021 Edition
- Runtime: Tokio (async)
- Database: SQLite (WAL mode) + SQLx
- IPC: JSON-RPC 2.0 over UDS/Named Pipe
- Logging: tracing + structured JSON
- Config: config crate + directories

## 아키텍처 패턴 (ADR-001)
**Hexagonal Architecture (Ports & Adapters)**

```
         Inbound Adapters
              ↓
    ┌─────────────────────┐
    │   Application       │
    │   (Use Cases)       │
    ├─────────────────────┤
    │   Domain + Ports    │  ← Core (No external deps)
    │   (Pure Logic)      │
    └─────────────────────┘
              ↓
       Outbound Adapters
```

의존성 규칙:
- Core → 아무것도 의존하지 않음
- Infrastructure → Core
- API → Core
- Daemon → 모두 조립 (Composition Root)

## 현재 개발 Phase (ADR-050)

### Phase 1: Core Foundation (MVP) - 현재
**기간**: ~2주
**목표**: 연결, 영속성, 기본 실행

구현 범위:
- JSON-RPC over UDS/Named Pipe
- SQLite WAL persistence
- IN_PROCESS execution만
- Atomic pop
- Basic supersede (generation-based)

### Phase 2: Execution Engine (계획)
- SUBPROCESS execution + PID tracking
- Crash recovery + Zombie process kill
- Retry + Exponential backoff
- deadline, ttl

### Phase 3: AI-Native Scheduling (계획)
- wait_for_idle, require_charging
- Event-driven planning
- Advanced supersede
- Backpressure

### Phase 4: Production (계획)
- OpenTelemetry metrics
- Tag-based UX
- VACUUM automation
- Zero-downtime upgrade

## 문서 구조 (ADR v2)

### 권위 문서 (Single Source of Truth)
- **ADR-000**: 문서 우선순위 규칙
- **ADR-001**: 시스템 아키텍처 (코드 구조)
- **ADR-002**: 운영 의미론 (Failure, Throttling)
- **ADR-010**: 데이터베이스 (스키마 SSOT)
- **ADR-020**: API 계약 (JSON-RPC)
- **ADR-030**: 테스트 전략
- **ADR-040**: 보안 정책
- **ADR-050**: 개발 로드맵
- **ADR-060**: 배포 라이프사이클

### 충돌 해결 규칙
- DB 스키마 → ADR-010이 우선
- 코드 구조 → ADR-001이 우선
- 동작/운영 → ADR-002가 우선
- API → ADR-020이 우선
- 일정/범위 → ADR-050이 우선

## 핵심 설계 결정

### 1. SQLite WAL Mode (ADR-010)
- 1 Writer + N Readers 동시성
- synchronous=NORMAL (성능 vs 안전성 균형)
- busy_timeout=200ms (빠른 실패)

### 2. Supersede Logic (ADR-002)
- subject_key: `<client>::<repo>::<path>`
- generation 단조 증가
- Insert-time + Pop-time lazy skip

### 3. Failure Classification (ADR-002)
- Transient: 재시도
- Permanent: 즉시 실패
- Infra: 제한 재시도 + Circuit Breaker

### 4. Consistency Model (ADR-002)
- JobQueue (meta.db): Strong Consistency
- Planner: Eventual-but-Bounded
- Worker Output: Eventual Consistency

### 5. Testing Policy (ADR-030)
**"No Test, No Touch"**
- 테스트 없는 코드 수정 금지
- Unit → Contract → Integration → Golden 계층
- MockClock + SeededRNG로 결정론적 테스트

## 보안 모델 (ADR-040)

### IPC 인증 (이중 레이어)
1. OS-level: SO_PEERCRED (Unix) / DACL (Windows)
2. Application-level: Bearer Token (256-bit random)

### Subprocess 샌드박싱
- 환경변수 Allowlist
- 작업 디렉토리 제한
- (향후) Network 제한

### 시크릿 관리
- Redacted<T> 타입으로 로그 마스킹
- secrecy crate로 메모리 제로화

## 주요 API (ADR-020)

### dev.enqueue.v1
```typescript
enqueue({
  job_type: "INDEX_FILE",
  queue: "code_intel",
  subject_key: "vscode::repo123::src/main.rs",
  payload: { ... },
  schedule?: { type: "IMMEDIATE" | "AT" | "AFTER" | "CONDITION" }
})
→ { job_id, queue, state }
```

### dev.cancel.v1
```typescript
cancel({ job_id | tag | chain_group_id })
→ { cancelled_count }
```

### logs.tail.v1
```typescript
tail({ job_id, offset, limit? })
→ { chunk, next_offset, eof }
```

### admin.stats.v1
```typescript
stats()
→ { queues: [...], system: { cpu, memory, idle, db_wal } }
```

## 개발 워크플로우

### 코드 작성 전
1. 해당 기능이 현재 Phase 범위인지 확인 (ADR-050)
2. 필요한 필드가 스키마에 존재하는지 확인 (ADR-010)
3. 관련 테스트 먼저 작성 (ADR-030)

### 코드 작성
1. Hexagonal 레이어 준수 (ADR-001)
2. Magic value 금지, const 사용
3. Deterministic (Clock/UUID/Random 주입)
4. Structured logging (tracing)
5. 함수 30줄 이하, 모듈 200줄 이하

### 커밋 전
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## 참고 자료
- 전체 ADR: `ADR_v2/` 디렉토리
- Serena 메모리: `.serena/memories/`
- Cursor rule: `.cursorrules`