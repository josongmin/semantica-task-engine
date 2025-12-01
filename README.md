# Semantica Task Engine

AI Native Dev Task Orchestrator - 로컬 환경에서 실행되는 고성능 태스크 오케스트레이터

## 프로젝트 상태

**현재 Phase**: Phase 4 (Production Ready - 99% 완료)

✅ Phase 1: Core Foundation - **완료**
✅ Phase 2: Execution Engine - **완료**  
✅ Phase 3: AI-Native Scheduling - **완료**  
🚀 Phase 4: Production Readiness - **진행 중** (DoD: 3/3)

## 아키텍처

Hexagonal Architecture 기반:
- **Domain**: 순수 비즈니스 로직 (Job, JobState, 상태 전이)
- **Port**: 외부 의존성 인터페이스 (JobRepository, TaskExecutor, SystemProbe)
- **Application**: Use-case 레이어 (Worker, Scheduler, Recovery, Maintenance)
- **Infrastructure**: SQLite, Subprocess, SystemProbe, Maintenance 구현
- **API**: JSON-RPC over TCP (포트: 9527)

## 빠른 시작

### 빌드 및 실행

```bash
# 개발 빌드
cargo build

# Release 빌드 (최적화)
cargo build --release

# OpenTelemetry 포함 빌드
cargo build --release --features telemetry

# Daemon 실행
./target/release/semantica

# 또는 환경변수 설정
SEMANTICA_DB_PATH=~/.semantica/meta.db \
SEMANTICA_RPC_PORT=9527 \
SEMANTICA_LOG_FORMAT=json \
    ./target/release/semantica
```

### CLI 사용

```bash
# Job 등록
./target/release/semantica-cli enqueue \
  --job-type INDEX_FILE \
  --queue default \
  --subject "src/main.rs" \
  --priority 0 \
  --payload '{"path": "src/main.rs"}'

# Job 취소
./target/release/semantica-cli cancel <job-id>

# 로그 조회
./target/release/semantica-cli logs <job-id>

# 시스템 상태 (Admin API 호출)
./target/release/semantica-cli status

# DB 유지보수 (Admin API 호출)
./target/release/semantica-cli maintenance
./target/release/semantica-cli maintenance --force-vacuum
```

### SDK 사용

#### Rust SDK

Rust 프로젝트에서 사용:

```rust
// Cargo.toml에 추가
[dependencies]
semantica-sdk = { path = "path/to/semantica-sdk" }

// 코드에서 사용
use semantica_sdk::{SematicaClient, EnqueueRequest};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = SematicaClient::connect("http://127.0.0.1:9527").await?;
    
    let response = client.enqueue(EnqueueRequest {
        job_type: "INDEX_FILE".to_string(),
        queue: "default".to_string(),
        subject_key: "src/main.rs".to_string(),
        priority: 0,
        payload: json!({"path": "src/main.rs"}),
    }).await?;
    
    println!("Job ID: {}", response.job_id);
    Ok(())
}
```

**SDK 문서:**
- Rust SDK: [crates/sdk/README.md](./crates/sdk/README.md)
- Python SDK: [python-sdk/README.md](./python-sdk/README.md)

#### Python SDK

Python 프로젝트에서 사용:

```python
# 설치
pip install semantica-sdk

# 사용
from semantica import SematicaClient, EnqueueRequest

async def main():
    async with SematicaClient("http://127.0.0.1:9527") as client:
        response = await client.enqueue(
            EnqueueRequest(
                job_type="INDEX_FILE",
                queue="default",
                subject_key="src/main.rs",
                payload={"path": "src/main.rs"}
            )
        )
        print(f"Job ID: {response.job_id}")
```

### Docker 사용

```bash
# 빌드 및 실행
./scripts/deploy.sh build
./scripts/deploy.sh start

# 로그 확인
./scripts/deploy.sh logs

# 상태 확인
./scripts/deploy.sh status
```

### 테스트

```bash
# 모든 테스트 실행
cargo test --all

# Phase별 DoD 테스트
cargo test --package semantica-integration-tests

# 워크로드 테스트
./.temp/workload-test.sh
```

## 디렉토리 구조

```
semantica-task-engine/
├── crates/
│   ├── core/               # Domain + Ports + Application
│   ├── infra-sqlite/       # SQLite 구현 (JobRepository, Maintenance)
│   ├── infra-system/       # System 구현 (TaskExecutor, SystemProbe)
│   ├── api-rpc/            # JSON-RPC 서버
│   ├── daemon/             # Main entry point (DI 조립)
│   ├── cli/                # CLI 도구
│   └── integration-tests/  # Phase DoD 통합 테스트
├── scripts/                # 배포 및 검증 스크립트
├── Dockerfile              # 프로덕션 이미지
├── docker-compose.yml      # 로컬/프로덕션 배포
└── Cargo.toml              # Workspace root
```

## 핵심 기능

### Phase 1: Core Foundation ✅
- [x] Job 생성 및 상태 관리 (QUEUED → RUNNING → DONE/FAILED)
- [x] Supersede 로직 (generation 기반, 동일 subject_key 덮어쓰기)
- [x] Hexagonal Architecture 구조
- [x] SQLite 영속성 (WAL 모드, 원자적 트랜잭션)
- [x] Worker loop (비동기 처리, panic isolation)
- [x] Priority 기반 FIFO 스케줄링
- [x] JSON-RPC API (dev.enqueue, dev.cancel, logs.tail)
- [x] Crash Recovery (orphaned job 복구)

### Phase 2: Execution Engine ✅
- [x] Subprocess 실행 (격리된 프로세스)
- [x] Retry 로직 (exponential backoff, max_attempts, deadline, TTL)
- [x] 환경변수 Allowlisting (PATH, HOME, USER, TERM)
- [x] Graceful Killing (SIGTERM → SIGKILL)
- [x] PID 추적 및 좀비 프로세스 방지

### Phase 3: AI-Native Scheduling ✅
- [x] 조건부 실행 (wait_for_idle, require_charging, wait_for_event)
- [x] CPU/Memory Idle 감지 (이동 평균 기반)
- [x] Battery Check (macOS/Linux: AC Power 또는 ≥80% 배터리)
- [x] Event Coalescing (초당 최대 N개)
- [x] 스케줄링 (scheduled_at, 미래 시각 대기)
- [x] Advanced Supersede (작업 종속성 관리)

### Phase 4: Production Readiness 🚀
- [x] JSON 구조화 로깅 (OpenTelemetry 준비)
- [x] DB Maintenance (Auto VACUUM, GC, Health Check)
- [x] Admin API (admin.stats.v1, admin.maintenance.v1)
- [x] CLI 도구 (enqueue, cancel, logs, status, maintenance)
- [x] SDK (Rust Client, Python 계획)
- [x] Docker/Compose 배포
- [x] 운영 가이드 문서
- [x] Determinism (테스트 재현 가능, IdProvider/TimeProvider 주입)
- [ ] 2주 연속 운영 테스트 (진행 예정)

## 개발 명령어

```bash
# 포맷팅
cargo fmt

# 린팅 (경고를 에러로)
cargo clippy --all-targets -- -D warnings

# 모든 테스트
cargo test --all

# 통합 테스트 (Phase DoD)
cargo test --package semantica-integration-tests

# Release 빌드
cargo build --release

# Just 사용 (권장)
just dev      # fmt + clippy + test
just check    # 전체 체크
just verify   # 검증 스크립트 실행
```

## 기술 스택

- **언어**: Rust 2021 Edition
- **비동기**: tokio, futures, async-trait
- **DB**: SQLx (SQLite, WAL 모드, 연결 풀)
- **직렬화**: serde, serde_json
- **에러**: thiserror (lib), anyhow (bin)
- **로깅**: tracing, tracing-subscriber (JSON 지원)
- **RPC**: jsonrpsee (JSON-RPC 2.0 over TCP)
- **시스템**: sysinfo (CPU/Memory 모니터링)
- **테스트**: mockall, tokio-test
- **Observability**: OpenTelemetry (optional)

## 성능 특성

- **처리량**: 초당 50+ job enqueue/pop
- **메모리**: ~10MB (idle), ~50MB (100 jobs 처리)
- **DB 크기**: ~1MB (1000 jobs), VACUUM으로 압축
- **Startup**: <100ms (migration 포함)
- **Shutdown**: <5s (graceful, 진행 중 작업 완료 대기)

## 안정성

- **Crash Recovery**: Daemon 재시작 시 orphaned job 자동 복구
- **Panic Isolation**: Worker panic이 Daemon 중단 안 함
- **Atomic Operations**: SQLite 트랜잭션 + WAL 모드
- **Graceful Shutdown**: SIGTERM 처리, 진행 중 작업 완료
- **Resource Cleanup**: RAII 패턴, Drop 구현

## 라이선스

MIT
