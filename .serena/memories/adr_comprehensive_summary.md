# ADR v2 종합 요약

## 문서 구조 (ADR_v2/)

### 핵심 원칙
- **ADR-000: Master Integration** - 문서 우선순위 및 충돌 해결 규칙
- **ADR-001: System Architecture** - 헥사고날 아키텍처, 워크스페이스 구조, 기술 스택
- **ADR-002: Operational Semantics** - Failure, Throttling, Isolation, Consistency 모델

### 구현 상세
- **ADR-010: Database Persistence** - 스키마 SSOT, 인덱스 전략, 마이그레이션
- **ADR-020: API Contract** - JSON-RPC 명세, 에러 코드, SDK 인터페이스
- **ADR-030: Testing Strategy** - 테스트 계층, Golden Test, Phase별 DoD
- **ADR-040: Security Policy** - IPC 인증, 샌드박싱, 시크릿 관리
- **ADR-050: Development Roadmap** - Phase 1-4 정의, 스코프, DoD
- **ADR-060: Distribution Lifecycle** - 코드 서명, 자동 업데이트, 배포

## 충돌 해결 규칙 (ADR-000)

| 도메인 | 권위 문서 | 근거 |
|--------|-----------|------|
| DB 스키마 | ADR-010 | 스키마 SSOT |
| 코드 구조 | ADR-001 | 헥사고날 규칙 |
| 동작/운영 | ADR-002 | Failure/Throttling 정의 |
| API 계약 | ADR-020 | JSON-RPC 바인딩 |
| 일정/범위 | ADR-050 | Phase 정의 |

## Phase별 진화 원칙

**절대 규칙**: Phase N+1 기능을 Phase N에 구현 금지

### Phase 매핑
- Phase 1 (MVP): 기본 통신, 영속성, IN_PROCESS 실행만
- Phase 2 (안정화): SUBPROCESS, 재시도, 크래시 복구
- Phase 3 (조건부 실행): wait_for_idle, 이벤트 기반 스케줄링
- Phase 4 (운영 완성): 메트릭, 태그, VACUUM, 업그레이드

## 헥사고날 아키텍처 (ADR-001)

```
crates/
  core/           # Domain + Ports + Application (외부 의존성 없음)
  infra-sqlite/   # JobRepository 구현
  infra-system/   # SystemProbe, TaskExecutor 구현
  infra-metrics/  # Logger, Metrics 구현
  api-rpc/        # JSON-RPC 서버
  api-cli/        # CLI
  daemon/         # Composition Root (DI wiring)
```

**의존성 규칙**:
- ✅ daemon → core + infra-* + api-*
- ✅ infra-* → core
- ✅ api-* → core
- ⛔ core → infra-* (절대 금지)

## 주요 설계 결정

### Failure Semantics (ADR-002)
- Transient: 재시도 (exponential backoff)
- Permanent: 즉시 실패
- Infra: 제한된 재시도 + Circuit Breaker

### Throttling (ADR-002)
- Queue Depth 기반
- CPU/Memory/Battery 상태 기반
- GlobalLimiter (cpu/gpu/io tokens)
- Weighted Round Robin (repo별 공정성)

### 일관성 모델 (ADR-002)
- JobQueue (meta.db): Strong Consistency (Single Writer)
- Planner: Eventual-but-Bounded
- Worker Output: Eventual Consistency

### 보안 (ADR-040)
- IPC: SO_PEERCRED (Unix) + Bearer Token
- Subprocess: 환경변수 sanitization, 작업 디렉토리 제한
- Logging: Redacted<T> 타입으로 시크릿 마스킹

## 테스트 정책 (ADR-030)

**No Test, No Touch** - 테스트 없는 코드 수정 금지

테스트 계층:
1. Unit: 순수 함수, 모든 I/O 모킹
2. Contract: API 스키마, 에러 코드
3. Integration: DB, Worker Loop, 실제 IPC
4. Golden: Scheduler/Planner 스냅샷 테스트

## 기술 스택 (ADR-001)

- Runtime: tokio
- DB: sqlx (컴파일 타임 쿼리 검증)
- Serialization: serde/serde_json
- Error: thiserror (lib), anyhow (bin)
- Logging: tracing/tracing-subscriber
- RPC: jsonrpsee
- System: sysinfo