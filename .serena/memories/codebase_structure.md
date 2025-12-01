# Codebase Structure (SSOT: ADR-001)

## 헥사고날 아키텍처 원칙

**절대 규칙**: Domain(Core)는 Infrastructure에 절대 의존하지 않음

### 의존성 방향
```
✅ daemon → core + infra-* + api-*
✅ infra-* → core
✅ api-* → core
⛔ core → infra-* (절대 금지)
⛔ core → sqlx (타입 제외, 순수 domain 타입 선호)
```

## Workspace 구조

```
semantica-orchestrator/
  Cargo.toml              # Workspace root
  rust-toolchain.toml
  README.md

  crates/
    # --- The Hexagon (Core Logic) ---
    core/                 # Domain + Ports + Application
                          # 의존성: 순수 Rust (serde, chrono, uuid만)
      src/
        domain/           # 순수 엔티티, 규칙
          job.rs
          schedule.rs
          condition.rs
          subject.rs
          error.rs
        port/             # Trait 정의 (#[async_trait] 허용)
          job_repository.rs
          system_probe.rs
          time_provider.rs
          subprocess_executor.rs
          logger.rs
        application/      # Use Cases (Port 사용)
          dev_task/
            enqueue_task.rs
            cancel_task.rs
            query_tasks.rs
          scheduling/
            planner.rs
            scheduler_loop.rs
            condition_evaluator.rs
          worker/
            worker_loop.rs
            global_limiter.rs
        dto/              # API/SDK용 Projection
          job_view.rs
          queue_stats_view.rs

    # --- Outbound Adapters (Driven) ---
    infra-sqlite/         # Persistence
      src/
        connection.rs     # WAL 설정, migrations
        job_repository_sqlite.rs
        subject_repository_sqlite.rs
        mapping/
          job_row_mapper.rs

    infra-system/         # OS / Subprocess
      src/
        system_probe_impl.rs   # CPU/Idle/Battery
        time_provider_impl.rs
        subprocess/
          cargo_test_executor.rs
          generic_executor.rs
          process_killer.rs

    infra-metrics/        # Observability
      src/
        logger_impl.rs
        metrics_exporter.rs

    # --- Inbound Adapters (Driving) ---
    api-rpc/              # JSON-RPC Server
      src/
        server.rs
        routes/
          dev_task_routes.rs
          admin_routes.rs
        dto/
          request.rs
          response.rs

    api-cli/              # CLI (Optional)
      src/
        main.rs
        commands/
          enqueue.rs
          list.rs
          cancel.rs

    # --- Composition Root ---
    daemon/               # Main Executable
      src/
        main.rs
        config.rs         # Queue 정책, GlobalLimiter 설정
        bootstrap.rs      # DI Wiring (Adapter → Core 주입)
        runtime.rs        # Scheduler + Worker Pool 구동
        signal.rs         # SIGINT/SIGTERM, Graceful Shutdown

  tests/
    integration/          # E2E 통합 테스트
    golden/               # Golden Test 스냅샷
      planner/
      scheduler/

  migrations/             # DB 마이그레이션
    001_initial_schema.sql
    002_add_execution_retry.sql
    003_add_conditions.sql
    004_add_dx_fields.sql
```

## 모듈 책임

### core/domain
- 순수 struct, 비즈니스 규칙
- 일반적으로 async 로직 없음
- 외부 의존성 절대 금지

### core/port
- Trait만 정의
- `#[async_trait]` 허용
- 구현체는 infra-*에 존재

### core/application
- Use Case 계층
- Port trait만 사용하여 동작
- Infrastructure 구체 타입 절대 참조 금지

### daemon/bootstrap.rs
- **유일하게** `Arc<SqliteJobRepository>`를 `Arc<dyn JobRepository>`로 캐스팅하는 곳
- DI 조립 책임
- 모든 Adapter 인스턴스화

## Import 예시

### core/application에서
```rust
use crate::domain::{job::Job, schedule::Schedule};
use crate::port::{
    job_repository::JobRepository,
    system_probe::SystemProbe,
    time_provider::TimeProvider,
};
```

### infra-sqlite에서
```rust
use core::domain::job::Job;
use core::port::job_repository::JobRepository;
```

### daemon/bootstrap에서
```rust
use core::application::scheduling::scheduler_loop::SchedulerLoop;
use infra_sqlite::job_repository_sqlite::SqliteJobRepository;
use infra_system::system_probe_impl::OsSystemProbe;
use api_rpc::server::RpcServer;
```

## 의존성 검증 방법

```bash
# core가 infra에 의존하지 않는지 확인
cargo tree -p semantica-core | grep -i infra
# 결과 없어야 함

# daemon이 모든 crate를 사용하는지 확인
cargo tree -p semantica-daemon | grep semantica
# core, infra-*, api-* 모두 포함되어야 함
```

## 확장 시나리오

### 새 Storage 추가 (SQLite → Postgres)
1. `crates/infra-postgres/` 생성
2. `JobRepository` trait 구현
3. `daemon/bootstrap.rs`에서 교체
4. **core 코드 수정 불필요**

### 새 Transport 추가 (UDS → gRPC)
1. `crates/api-grpc/` 생성
2. `DevTaskService` trait 사용
3. `daemon/bootstrap.rs`에서 병렬 실행
4. **core 코드 수정 불필요**