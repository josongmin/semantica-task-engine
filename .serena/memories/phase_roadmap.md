# Development Roadmap (SSOT: ADR-050)

## 절대 원칙
**Phase N+1 기능을 Phase N에 구현 금지**
스키마 진화는 ADR-010의 Evolution Matrix 준수

## Phase 1: Core Foundation (MVP)
**목표**: 연결, 영속성, 기본 실행
**기간**: ~2주

### Scope
- Transport: JSON-RPC 2.0 over UDS/Named Pipe
- Persistence: SQLite WAL, Single-Writer/Multi-Reader
- Execution: IN_PROCESS만 (재시도 없음, 타임아웃 없음)
- Queue: FIFO 순서, Atomic Pop
- Logging: logs.tail API

### 구현 필드 (jobs 테이블)
- ✅ id, queue, job_type
- ✅ subject_key, generation (Supersede용)
- ✅ state, priority, created_at
- ✅ payload, log_path
- ❌ execution_mode (Phase 2)
- ❌ attempts, deadline, ttl (Phase 2)
- ❌ schedule_*, wait_for_* (Phase 3)
- ❌ user_tag, chain_group_id (Phase 4)

### Definition of Done
- [ ] Indexing: Semantica Repo에서 100+ 파일 인덱싱 성공
- [ ] Persistence: 데몬 재시작 후 QUEUED job 복구 (데이터 손실 없음)
- [ ] Concurrency: 부하 상황에서 SQLITE_BUSY 에러 없음
- [ ] API: dev.enqueue, dev.cancel, logs.tail 완전 작동

## Phase 2: Execution Engine Hardening
**목표**: 안전성, 격리, 크래시 복구
**기간**: ~3주

### Scope
- Subprocess: execution_mode=SUBPROCESS + PID 추적
- Resilience: panic 처리 (catch_unwind), Zombie process kill
- Retry: attempts, max_attempts, backoff_factor
- Timeouts: deadline (실행 제한), ttl_ms (큐 제한)
- System Probe: CPU/Memory 모니터링

### 추가 필드
- ✅ execution_mode, pid, env_vars
- ✅ attempts, max_attempts, backoff_factor
- ✅ deadline, ttl_ms
- ✅ trace_id
- ❌ schedule_*, wait_for_* (Phase 3)

### Definition of Done
- [ ] Isolation: Worker panic/subprocess crash가 Daemon 죽이지 않음
- [ ] Recovery: 재시작 시 orphaned PID 정리 및 job 복구
- [ ] Throttling: CPU > 90% 시 낮은 우선순위 큐 일시 중지
- [ ] Retry: Transient failure → exponential backoff 정상 작동

## Phase 3: AI-Native Scheduling
**목표**: 컨텍스트 인식 및 조건부 실행
**기간**: ~2주

### Scope
- Conditions: wait_for_idle, require_charging, wait_for_event
- Schema: job_conditions 테이블 추가
- Planner: 이벤트 병합 (50개 FS 이벤트 → 1개 job)
- Supersede: Insert-time + Pop-time 고급 로직
- Backpressure: Battery/IO 기반 동적 조절

### 추가 필드
- ✅ schedule_type, scheduled_at, schedule_delay_ms
- ✅ wait_for_idle, require_charging, wait_for_event
- ✅ job_conditions 테이블
- ❌ user_tag, chain_group_id (Phase 4)

### Definition of Done
- [ ] Idle Trigger: 사용자 타이핑 중지 시에만 heavy 인덱싱 시작
- [ ] Event Trigger: "PR merge 시 rebuild" 워크플로우 안정 동작
- [ ] Efficiency: 타이핑 폭주 시 Supersede로 중복 job 80% 이상 감소

## Phase 4: Reliability & Ops (Production)
**목표**: SRE급 안정성 및 관측성
**기간**: ~3주

### Scope
- Observability: Structured Logging (JSON), OpenTelemetry Metrics
- UX: Tag 기반 관리 (user_tag, chain_group_id)
- Maintenance: 자동 VACUUM, Artifact GC
- Lifecycle: 무중단 업그레이드, 마이그레이션 롤백

### 추가 필드
- ✅ user_tag, chain_group_id, parent_job_id
- ✅ result_summary, artifact_path

### Definition of Done
- [ ] Stability: 2주 연속 운영 (메모리 누수/성능 저하 없음)
- [ ] Debuggability: 로그만으로 장애 원인 파악 가능
- [ ] Upgrade: 스키마 마이그레이션 및 롤백 CI 테스트 완료

## 타임라인 요약

| Phase | 기간 | 핵심 목표 |
|-------|------|-----------|
| Phase 1 | 1-2주 | MVP: 증분 인덱싱/그래프/임베딩 실행 가능 |
| Phase 2 | 2-3주 | Stable: subprocess + 복구 + 리소스 인식 |
| Phase 3 | 2주 | AI Native: 조건부 스케줄링 완성 |
| Phase 4 | 2-4주 | Production: 장기 실행 안정성 + 관측성 |

## 구현 체크리스트 (Phase 진입 전)

### Phase 1 → 2 전환 조건
- [ ] Phase 1 DoD 100% 완료
- [ ] 마이그레이션 스크립트 준비: `002_add_execution_retry.sql`
- [ ] GlobalLimiter 설계 완료
- [ ] Subprocess executor port trait 정의

### Phase 2 → 3 전환 조건
- [ ] Phase 2 DoD 100% 완료
- [ ] 마이그레이션 스크립트: `003_add_conditions.sql`
- [ ] SystemProbe idle 감지 로직 검증
- [ ] Event-driven Planner 설계 완료

### Phase 3 → 4 전환 조건
- [ ] Phase 3 DoD 100% 완료
- [ ] 마이그레이션 스크립트: `004_add_dx_fields.sql`
- [ ] OpenTelemetry exporter 통합 완료
- [ ] VACUUM job 구현 완료