# Testing Strategy (SSOT: ADR-030)

## 절대 원칙: "No Test, No Touch"

**코드 수정 전 테스트 작성 필수**
- 테스트 없는 코드 수정 금지
- 커버리지 누락 시: 수정 전 테스트 먼저 작성 (Refactoring-Safe)
- CI 강제: 테스트 없는 PR 자동 거부

## 테스트 피라미드

| 레이어 | 대상 | 역할 | 제약 |
|--------|------|------|------|
| Unit | 순수 함수, Domain 로직 | 비즈니스 규칙 및 엣지 케이스 고정 | 모든 I/O 모킹 (DB/Network/Time), < 1ms |
| Contract | SDK, RPC, DTOs | API 스키마 및 에러 코드 고정 | 하위 호환성 보장, Breaking change → 버전 업 |
| Integration | DB, Worker Loop, IPC | 트랜잭션 및 파이프라인 일관성 검증 | In-Memory SQLite + 실제 IPC 소켓 |
| Golden | Scheduler, Planner, RAG | 복잡한 시간 로직 및 품질 검증 | 스냅샷 테스트 + Deterministic Mock |

## Golden Test Framework

### 비결정성 제거
**문제**: Scheduler/Planner는 시간, UUID, 랜덤성에 의존
**해결**:
- MockClock: `SystemTime::now()` 대신 `ctx.now()` 사용, tick으로만 시간 진행
- Seeded RNG/UUID: 순차 생성 (job-001, job-002) 또는 고정 시드

### 디렉토리 구조
```
tests/golden/
├── planner/
│   ├── fs_event_burst.json
│   └── git_rebase_storm.json
└── scheduler/
    ├── supersede_debounce.json
    ├── retry_backoff.json
    └── recovery_zombie.json
```

### Golden 파일 형식
```json
{
  "scenario": "Transient failure triggers exponential backoff",
  "initial_db": [
    { "id": "job-1", "state": "RUNNING", "attempts": 0 }
  ],
  "actions": [
    { "tick": 100, "action": "worker_fail", "job_id": "job-1", "error": "transient" }
  ],
  "expected_transitions": [
    { 
      "job_id": "job-1", 
      "from": "RUNNING", 
      "to": "QUEUED", 
      "scheduled_delay": 2000,
      "reason": "backoff"
    }
  ]
}
```

## Phase별 Definition of Done (DoD)

### Phase 1: Core Foundation
**목표**: 데이터 손실 없음, FIFO 순서 보장

테스트 시나리오:
- **(F1) Order**: 100개 job enqueue → FIFO 실행 검증
- **(F2) Persistence**: kill -9 후 재시작 → QUEUED job 유지 확인
- **(F3) Log Tail**: logs.tail이 데이터 정확하게 스트리밍

### Phase 2: Execution Engine
**목표**: Subprocess 안전성 및 크래시 복구

테스트 시나리오:
- **(E1) Isolation**: Subprocess panic이 Daemon 크래시하지 않음
- **(E2) Zombie Kill**: 재시작 시 orphaned pid 탐지 → SIGKILL → FAILED
- **(E3) TTL/Deadline**: 시간 초과 job skip 검증

### Phase 3: AI-Native Scheduling
**목표**: 컨텍스트 인식

테스트 시나리오:
- **(S1) Idle**: wait_for_idle=true인 job은 SystemProbe idle 보고 시에만 시작
- **(S2) Supersede**: 50개 연속 이벤트 → 1개 job만 실행 (Debounce/Supersede)
- **(S3) Backpressure**: CPU > 80% → 낮은 우선순위 큐 일시 정지

### Phase 4: Reliability & Ops
**목표**: SRE급 안정성

테스트 시나리오:
- **(R1) Long-Run**: 24시간 스트레스 테스트 + 랜덤 fault, 메모리 안정
- **(R2) Migration**: 바이너리 업그레이드 + 스키마 마이그레이션 성공
- **(R3) Rollback**: 잘못된 스키마 → Daemon 시작 거부 (Fail-Safe)

## CI 강제 규칙

### Pre-commit
- `cargo fmt --check`
- `cargo clippy -- -D warnings`

### PR 필수 조건
- 모든 테스트 통과
- 변경된 코드에 대한 테스트 추가/수정
- Contract Test 통과 (API 변경 시)
- Golden Test 스냅샷 업데이트 (의도된 동작 변경 시)

### CI 파이프라인
```
Test → Audit (cargo audit) → Build → Sign → Publish
```

## AI Agent 시스템 프롬프트 (강제 규칙)

```
[CRITICAL DEVELOPMENT CONSTRAINTS - TEST FIRST POLICY]

1. **Test-Driven Modification:**
   - 테스트 없는 코드 수정 금지
   - 커버리지 누락 시 테스트 먼저 작성

2. **Contract Inviolability:**
   - Public API (Schema, Error Code) 절대 파괴 금지
   - DTO 변경 시 Contract Test 필수 업데이트

3. **Golden Set Quality:**
   - Planner/Scheduler 로직은 Golden Test 필수 실행
   - 스냅샷 불일치 시: Regression vs Intended Change 분석

4. **Deterministic Testing:**
   - MockClock + SeededRNG 사용
   - 테스트에서 sleep() 절대 금지

5. **Failure Explanation:**
   - 테스트 실패 시 assertion 메시지가 절대 진실
```

## 테스트 작성 가이드

### Unit Test 예시
```rust
#[test]
fn test_supersede_increments_generation() {
    let mut repo = InMemorySubjectRepository::new();
    let subject_key = "client::repo::path";
    
    // First job
    let gen1 = repo.next_generation(subject_key);
    assert_eq!(gen1, 1);
    
    // Second job (supersede)
    let gen2 = repo.next_generation(subject_key);
    assert_eq!(gen2, 2);
}
```

### Golden Test 예시
```rust
#[test]
fn test_retry_backoff_golden() {
    let scenario = load_golden("scheduler/retry_backoff.json");
    let mut scheduler = Scheduler::with_mock_clock(scenario.clock);
    
    scheduler.load_db(scenario.initial_db);
    scheduler.apply_actions(scenario.actions);
    
    let actual = scheduler.get_transitions();
    assert_eq!(actual, scenario.expected_transitions);
}
```