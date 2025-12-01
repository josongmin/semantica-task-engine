# 스키마 진화 (Phase별)

## SSOT
스키마 관련 모든 결정은 ADR-010 (Job Schema & Migration)을 단일 진실 공급원으로 사용

## Phase별 스키마 서브셋

### Phase 1 - Minimal Schema
필수 필드:
- id, queue, job_type, subject_key, generation
- priority, state, created_at, started_at, finished_at
- payload, log_path

특징: execution_mode, retry, TTL, schedule, condition 없음

### Phase 2 - Execution & Retry
Phase 1 + 추가:
- execution_mode, pid
- deadline, ttl_ms
- attempts, max_attempts, backoff_factor

### Phase 3 - Conditional Scheduling
Phase 2 + 추가:
- schedule_type, scheduled_at, schedule_delay_ms
- wait_for_idle, require_charging, wait_for_event
- job_conditions 테이블 생성

### Phase 4 - Result/Chain/DX
Phase 3 + 추가:
- parent_job_id, chain_group_id, user_tag
- result_summary, artifact_path

## Migration 전략

### 디렉토리 구조
```
migrations/
  001_initial_schema.sql          -- Phase 1
  002_add_execution_retry.sql     -- Phase 2
  003_add_conditions.sql          -- Phase 3
  004_add_chain_result.sql        -- Phase 4
migrations_down/
  002_down.sql
  003_down.sql
  004_down.sql
```

### Migration 정책
- 앱 기동 시 schema_version 확인 후 순차 적용
- 실패 시 down 스크립트로 롤백
- 데이터 손상 상태로 서비스 기동 불허
- "기동 실패 + 롤백 가능" 상태 보장