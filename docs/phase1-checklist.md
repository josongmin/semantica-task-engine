# Phase 1 구현 체크리스트

## 목표
실제 Semantica 개발 워크플로우에서 증분 인덱싱/그래프/임베딩 작업 안정 실행

## 완료된 작업

### 프로젝트 구조
- [x] Cargo.toml 업데이트 (의존성, 빌드 프로파일)
- [x] 디렉토리 구조 생성 (src/domain, src/port, src/application)
- [x] migrations/ 디렉토리
- [x] docs/ 디렉토리
- [x] tests/golden/ 디렉토리
- [x] rustfmt.toml, justfile, .gitignore

### Domain Layer
- [x] Job entity (id, queue, job_type, subject_key, generation, state, etc.)
- [x] JobState enum (QUEUED, RUNNING, DONE, FAILED, SUPERSEDED)
- [x] State transition methods (start, complete, fail, supersede)
- [x] DomainError types
- [x] Unit tests for Job

### Port Layer
- [x] JobRepository trait
- [x] TimeProvider trait

### Application Layer
- [x] DevTaskService
- [x] Enqueue use case

## 다음 단계 (구현 필요)

### Infrastructure Layer
- [ ] SQLite JobRepository 구현체
  - [ ] insert, find_by_id, update
  - [ ] pop_next (FIFO + priority)
  - [ ] get_latest_generation
  - [ ] mark_superseded
  - [ ] count_by_state
- [ ] Connection pool 설정 (WAL mode)
- [ ] Migration runner

### Worker
- [ ] Worker loop
  - [ ] pop_next from queue
  - [ ] Execute job (Phase 1: in-process mock)
  - [ ] Update state (RUNNING → DONE/FAILED)
  - [ ] Supersede check

### JSON-RPC API
- [ ] dev.enqueue.v1 구현
- [ ] admin.stats.v1 구현
- [ ] UDS/TCP transport

### Main
- [ ] Config 로딩
- [ ] DI/Composition root
- [ ] Graceful shutdown

### Testing
- [ ] Integration test
  - [ ] F1: 기본 enqueue/pop FIFO
  - [ ] F2: Supersede (generation)
  - [ ] F3: Crash 후 재시작
- [ ] 부하 테스트 (초당 50 enqueue)

## Phase 1 DoD
- [ ] 모든 기능 테스트 통과
- [ ] Semantica 코드베이스에서 실제 워크로드 구동 가능
- [ ] p95 enqueue latency ≤ 10ms

