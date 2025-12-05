# SemanticaTask Engine - AI 전체 구조 가이드

**AI/LLM이 프로젝트 전체 구조를 완벽히 이해하고 코드를 작성/수정할 수 있도록 작성된 종합 가이드**

> 이 문서는 SemanticaTask Engine의 모든 컴포넌트, 아키텍처, 데이터 플로우, 확장 방법을 설명합니다.

---

## 📋 목차

1. [프로젝트 개요](#1-프로젝트-개요)
2. [아키텍처 전체 구조](#2-아키텍처-전체-구조)
3. [디렉토리 구조](#3-디렉토리-구조)
4. [핵심 개념](#4-핵심-개념)
5. [데이터 플로우](#5-데이터-플로우)
6. [컴포넌트 상세](#6-컴포넌트-상세)
7. [통신 프로토콜](#7-통신-프로토콜)
8. [확장 포인트](#8-확장-포인트)
9. [개발 가이드](#9-개발-가이드)
10. [테스트 전략](#10-테스트-전략)

---

## 1. 프로젝트 개요

### 1.1 무엇인가?

**SemanticaTask Engine**은 로컬 환경에서 실행되는 **비동기 Job Queue 시스템**입니다.

**목적**:
- AI/개발자가 백그라운드 작업(파일 인덱싱, 코드 분석 등)을 비동기로 실행
- 우선순위 기반 스케줄링
- Crash Recovery (재시작 시 RUNNING Job 복구)
- Subject-based Superseding (동일 subject_key의 최신 Job만 실행)

**기술 스택**:
- **언어**: Rust (Backend), Python (SDK)
- **DB**: SQLite (WAL 모드)
- **통신**: JSON-RPC 2.0 over TCP
- **아키텍처**: Hexagonal Architecture (Ports & Adapters)

### 1.2 핵심 가치

1. **타입 안전성**: Rust 타입 시스템으로 컴파일 타임 검증
2. **Zero 다운타임**: 재시작 시 RUNNING Job 자동 복구
3. **확장 가능**: Port/Adapter 패턴으로 구현체 교체 가능
4. **AI 친화적**: 명확한 API, 문서화, 타입 정의

---

## 2. 아키텍처 전체 구조

### 2.1 Hexagonal Architecture (Clean Architecture)

```
┌─────────────────────────────────────────────────────────────┐
│                    Inbound Adapters                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   JSON-RPC   │  │     CLI      │  │   Rust SDK   │      │
│  │  (api-rpc)   │  │   (cli)      │  │    (sdk)     │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                 │                 │               │
└─────────┼─────────────────┼─────────────────┼───────────────┘
          │                 │                 │
          ▼                 ▼                 ▼
┌─────────────────────────────────────────────────────────────┐
│                   Application Layer                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │    Worker    │  │  Scheduler   │  │   Recovery   │      │
│  │  (Job 실행)   │  │ (우선순위)    │  │ (Crash 복구) │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                 │                 │               │
│         └─────────────────┴─────────────────┘               │
│                         │                                   │
│                         ▼                                   │
│  ┌────────────────────────────────────────────────────┐    │
│  │              Domain Layer (순수 비즈니스 로직)        │    │
│  │  ┌──────┐  ┌─────────┐  ┌─────────┐  ┌──────────┐ │    │
│  │  │ Job  │  │JobState │  │  Queue  │  │  Error   │ │    │
│  │  └──────┘  └─────────┘  └─────────┘  └──────────┘ │    │
│  └────────────────────────────────────────────────────┘    │
│                         │                                   │
│                         ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Port Interfaces (추상화)                 │   │
│  │  JobRepository | TaskExecutor | SystemProbe         │   │
│  │  TimeProvider  | IdProvider   | Maintenance         │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                   Outbound Adapters                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   SQLite     │  │  Subprocess  │  │ SystemProbe  │      │
│  │ (infra-sql)  │  │ (infra-sys)  │  │ (infra-sys)  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 계층별 책임

| 계층 | 책임 | 예시 | 의존성 |
|------|------|------|--------|
| **Domain** | 순수 비즈니스 로직 | Job, JobState, Queue | 없음 |
| **Port** | 외부 의존성 인터페이스 | `trait JobRepository` | Domain만 |
| **Application** | Use-case 구현 | Worker, Scheduler, Recovery | Domain + Port |
| **Infrastructure** | Port 구현체 | SQLiteJobRepository | Domain + Port |
| **API** | 외부 인터페이스 | JSON-RPC, CLI | All |
| **Daemon** | Composition Root (DI) | main.rs (의존성 주입) | All |

**핵심 규칙**:
- Domain은 **어떤 것도 의존하지 않음** (순수 Rust, no I/O)
- Application은 **Port를 통해서만** Infrastructure 사용
- Infrastructure는 **Port를 구현**
- Daemon은 **모든 의존성을 조립**

---

## 3. 디렉토리 구조

```
semantica-task-engine/
│
├── ADR_v2/                     # Architecture Decision Records
│   ├── ADR-000-master-integration.md    # 문서 우선순위 정의
│   ├── ADR-001-system-architecture.md   # Hexagonal 아키텍처
│   ├── ADR-002-operational-semantics.md # Failure/Throttling 로직
│   ├── ADR-010-database-persistence.md  # DB 스키마 SSOT
│   ├── ADR-020-api-contract.md          # JSON-RPC 명세
│   ├── ADR-030-testing-strategy.md      # 테스트 계층
│   ├── ADR-040-security-policy.md       # IPC 인증
│   ├── ADR-050-development-roadmap.md   # Phase 정의
│   └── ADR-060-distribution-lifecycle.md # 배포 전략
│
├── crates/                     # Rust Workspace (Cargo 멀티 패키지)
│   │
│   ├── core/                   # 🧠 Domain + Port + Application
│   │   ├── src/
│   │   │   ├── domain/         # 순수 비즈니스 로직
│   │   │   │   ├── job.rs      # Job 구조체, 상태 전이
│   │   │   │   ├── queue.rs    # Queue 개념
│   │   │   │   └── error.rs    # Domain 에러
│   │   │   │
│   │   │   ├── port/           # 외부 의존성 인터페이스 (trait)
│   │   │   │   ├── job_repository.rs     # DB CRUD trait
│   │   │   │   ├── task_executor.rs      # Job 실행 trait
│   │   │   │   ├── system_probe.rs       # CPU/Mem 조회 trait
│   │   │   │   ├── time_provider.rs      # 시간 제공 trait
│   │   │   │   ├── id_provider.rs        # UUID 생성 trait
│   │   │   │   └── maintenance.rs        # GC trait
│   │   │   │
│   │   │   └── application/    # Use-case 레이어
│   │   │       ├── worker/     # Job 실행 루프
│   │   │       ├── scheduler.rs # 우선순위 스케줄링
│   │   │       ├── recovery.rs  # Crash Recovery
│   │   │       ├── retry.rs     # Retry 로직
│   │   │       └── maintenance/ # GC, 클린업
│   │   │
│   │   └── Cargo.toml
│   │
│   ├── infra-sqlite/           # 🗄️ SQLite 구현체
│   │   ├── migrations/         # DB 스키마 마이그레이션
│   │   │   ├── 001_initial_schema.sql
│   │   │   ├── 002_add_execution_retry.sql
│   │   │   ├── 003_add_scheduling.sql
│   │   │   └── 004_add_dx_fields.sql
│   │   ├── src/
│   │   │   ├── job_repository.rs  # JobRepository 구현
│   │   │   ├── transaction.rs     # Transaction 구현
│   │   │   ├── connection.rs      # SQLite 연결 풀
│   │   │   ├── migration.rs       # 마이그레이션 실행
│   │   │   └── maintenance_impl.rs # GC 구현
│   │   └── Cargo.toml
│   │
│   ├── infra-system/           # 💻 System 구현체
│   │   ├── src/
│   │   │   ├── subprocess_executor.rs # TaskExecutor 구현
│   │   │   └── system_probe_impl.rs   # SystemProbe 구현
│   │   └── Cargo.toml
│   │
│   ├── api-rpc/                # 🌐 JSON-RPC API
│   │   ├── src/
│   │   │   ├── server.rs       # RPC 서버 (jsonrpsee)
│   │   │   ├── handler.rs      # RPC 메서드 핸들러
│   │   │   ├── types.rs        # Request/Response DTO
│   │   │   └── error.rs        # RPC 에러
│   │   └── Cargo.toml
│   │
│   ├── daemon/                 # 🚀 Main Binary (Composition Root)
│   │   ├── src/
│   │   │   ├── main.rs         # 의존성 주입 + 서버 시작
│   │   │   └── telemetry.rs    # OpenTelemetry 설정
│   │   └── Cargo.toml
│   │
│   ├── cli/                    # 🛠️ CLI Tool
│   │   ├── src/
│   │   │   └── main.rs         # CLI 명령어 (clap)
│   │   └── Cargo.toml
│   │
│   ├── sdk/                    # 📦 Rust SDK
│   │   ├── src/
│   │   │   ├── client.rs       # SDK 클라이언트
│   │   │   ├── types.rs        # SDK 타입
│   │   │   └── error.rs        # SDK 에러
│   │   └── Cargo.toml
│   │
│   └── integration-tests/      # 🧪 통합 테스트
│       ├── tests/
│       │   ├── phase1_dod.rs   # Phase 1 DoD 테스트
│       │   ├── phase2_dod.rs   # Phase 2 DoD 테스트
│       │   └── phase3_dod.rs   # Phase 3 DoD 테스트
│       └── Cargo.toml
│
├── python-sdk/                 # 🐍 Python SDK
│   ├── semantica/
│   │   ├── __init__.py
│   │   ├── client.py           # Python 클라이언트 (httpx)
│   │   ├── types.py            # Python 타입 (dataclass)
│   │   └── errors.py           # Python 에러
│   ├── README.md               # Python SDK 문서
│   ├── QUICKSTART.md           # 5분 시작 가이드
│   ├── AI_CONTEXT.md           # AI 전용 가이드
│   └── pyproject.toml          # Python 패키지 설정
│
├── docs/                       # 📚 문서
│   ├── api-spec.md             # JSON-RPC API 명세
│   ├── operations.md           # 운영 가이드
│   └── PHASE*_COMPLETION.md    # Phase 완료 보고서
│
├── examples/                   # 📝 예제
│   ├── python/                 # Python 통합 예제
│   └── integration/            # Docker 통합 예제
│
├── Dockerfile                  # 🐳 프로덕션 이미지
├── Dockerfile.dev              # 🐳 개발 이미지
├── docker-compose.yml          # 🐳 프로덕션 Compose
├── docker-compose.dev.yml      # 🐳 개발 Compose
│
├── Cargo.toml                  # 📦 Workspace 루트
├── Cargo.lock                  # 📦 의존성 락 파일
└── README.md                   # 📖 프로젝트 README
```

---

## 4. 핵심 개념

### 4.1 Job (작업 단위)

```rust
pub struct Job {
    pub id: JobId,                  // UUID
    pub queue: String,              // 큐 이름 ("default", "code_intel")
    pub job_type: String,           // Job 타입 ("INDEX_FILE", "ANALYZE")
    pub subject_key: String,        // 중복 방지 키 ("repo::file.py")
    pub generation: i32,            // Subject별 세대 번호
    pub state: JobState,            // 현재 상태
    pub priority: i32,              // 우선순위 (높을수록 먼저)
    pub payload: Value,             // Job 데이터 (JSON)
    pub log_path: Option<String>,   // 로그 파일 경로
    pub created_at: Timestamp,      // 생성 시간
    pub started_at: Option<Timestamp>,  // 시작 시간
    pub finished_at: Option<Timestamp>, // 완료 시간
    
    // Phase 2 (Execution & Retry)
    pub execution_mode: ExecutionMode,  // IN_PROCESS | SUBPROCESS
    pub attempts: i32,              // 시도 횟수
    pub max_attempts: i32,          // 최대 시도 (0=무한)
    pub deadline: Option<Timestamp>,// 마감 시간
    
    // Phase 3 (Scheduling)
    pub schedule_type: ScheduleType,    // IMMEDIATE | AT | AFTER | CONDITION
    pub schedule_at: Option<Timestamp>, // 예약 시간
    pub wait_for_job_id: Option<JobId>,// 대기할 Job ID
    
    // Phase 4 (UX)
    pub user_tag: Option<String>,   // 사용자 태그
    pub parent_job_id: Option<JobId>,   // 부모 Job
    pub chain_group_id: Option<String>, // Chain 그룹 ID
}
```

### 4.2 JobState (상태 머신)

```rust
pub enum JobState {
    QUEUED,      // 대기 중 (초기 상태)
    RUNNING,     // 실행 중
    DONE,        // 완료 (성공)
    FAILED,      // 실패
    SUPERSEDED,  // 새 Job으로 대체됨
    CANCELLED,   // 사용자가 취소
    REQUEUED,    // 재시도 대기
    SCHEDULED,   // 예약됨 (Phase 3)
    WAITING,     // 다른 Job 대기 (Phase 3)
}
```

**상태 전이 규칙**:
```
QUEUED → RUNNING → DONE
       → RUNNING → FAILED → REQUEUED → RUNNING
       → SUPERSEDED (새 Job 등록 시)
       → CANCELLED (사용자 취소)

SCHEDULED → QUEUED (예약 시간 도달)
WAITING → QUEUED (대기 조건 만족)
```

### 4.3 Subject-based Superseding

**개념**: 동일한 `subject_key`를 가진 Job이 QUEUED 상태로 있을 때, 새 Job을 등록하면:
1. 기존 Job의 `state`를 `SUPERSEDED`로 변경
2. 새 Job의 `generation`을 `기존 generation + 1`로 설정
3. 새 Job을 `QUEUED`로 등록

**목적**: 파일별, 사용자별로 최신 Job만 실행 (오래된 Job 무시)

**예시**:
```rust
// 1번째 등록: subject_key="repo1::file.py"
Job { id: "job-1", subject_key: "repo1::file.py", generation: 1, state: QUEUED }

// 2번째 등록: 동일 subject_key
// -> job-1 state=SUPERSEDED
// -> job-2 generation=2, state=QUEUED
Job { id: "job-2", subject_key: "repo1::file.py", generation: 2, state: QUEUED }
```

### 4.4 Queue (큐)

**개념**: Job을 그룹화하는 논리적 단위

**특징**:
- 큐별로 독립적인 Worker 프로세스
- 큐별 동시 실행 제한 가능 (향후)
- 기본 큐: `"default"`

**예시**:
- `"default"`: 일반 작업
- `"code_intel"`: 코드 인덱싱
- `"high_priority"`: 긴급 작업

---

## 5. 데이터 플로우

### 5.1 Job 등록 플로우

```
┌─────────────┐
│   Client    │  (Python/Rust SDK, CLI)
│  enqueue()  │
└──────┬──────┘
       │ HTTP POST (JSON-RPC)
       ▼
┌─────────────────────────────────────────────┐
│         api-rpc (JSON-RPC Server)           │
│  ┌──────────────────────────────────────┐   │
│  │ handler.rs::enqueue()                │   │
│  │  - DTO 파싱 (EnqueueRequest)          │   │
│  │  - 검증 (job_type, queue, subject_key)│   │
│  └─────────────┬────────────────────────┘   │
└────────────────┼────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│      core::application::dev_task           │
│  ┌──────────────────────────────────────┐   │
│  │ enqueue.rs::execute()                │   │
│  │  1. subject_key로 기존 Job 조회       │   │
│  │  2. 있으면 SUPERSEDED 처리            │   │
│  │  3. generation 계산 (max + 1)        │   │
│  │  4. Job 생성 (Domain::Job::new)      │   │
│  │  5. JobRepository::create() 호출     │   │
│  └─────────────┬────────────────────────┘   │
└────────────────┼────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│    infra-sqlite::JobRepository             │
│  ┌──────────────────────────────────────┐   │
│  │ job_repository.rs::create()          │   │
│  │  - BEGIN TRANSACTION                 │   │
│  │  - UPDATE old jobs (SUPERSEDED)      │   │
│  │  - INSERT new job                    │   │
│  │  - COMMIT                            │   │
│  └─────────────┬────────────────────────┘   │
└────────────────┼────────────────────────────┘
                 │
                 ▼
         ┌──────────────┐
         │   SQLite DB  │
         │ ~/.semantica │
         │   /meta.db   │
         └──────────────┘
```

### 5.2 Job 실행 플로우

```
┌─────────────────────────────────────────────┐
│      core::application::worker             │
│  ┌──────────────────────────────────────┐   │
│  │ worker/mod.rs::run()                 │   │
│  │  Loop:                               │   │
│  │    1. System throttling check        │   │
│  │    2. Pop next job (atomic)          │   │
│  │    3. Execute job                    │   │
│  │    4. Update state (DONE/FAILED)     │   │
│  └─────────────┬────────────────────────┘   │
└────────────────┼────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────┐
│  infra-sqlite::JobRepository::pop_next()   │
│  ┌──────────────────────────────────────┐   │
│  │ UPDATE jobs                          │   │
│  │ SET state='RUNNING', started_at=NOW  │   │
│  │ WHERE state='QUEUED'                 │   │
│  │   AND queue='default'                │   │
│  │ ORDER BY priority DESC, created_at   │   │
│  │ LIMIT 1                              │   │
│  │ RETURNING *                          │   │
│  └─────────────┬────────────────────────┘   │
└────────────────┼────────────────────────────┘
                 │
                 ▼
         ┌──────────────┐
         │ Job (RUNNING)│
         └──────┬───────┘
                │
                ▼
┌─────────────────────────────────────────────┐
│    core::port::TaskExecutor                │
│  ┌──────────────────────────────────────┐   │
│  │ IN_PROCESS:                          │   │
│  │   - 동기 함수 호출                     │   │
│  │   - stdout/stderr 캡처                │   │
│  │ SUBPROCESS:                          │   │
│  │   - spawn 프로세스                     │   │
│  │   - log_path에 출력 저장               │   │
│  └─────────────┬────────────────────────┘   │
└────────────────┼────────────────────────────┘
                 │
                 ▼
         ┌──────────────┐
         │ Job 완료      │
         │ state=DONE   │
         │ finished_at  │
         └──────────────┘
```

### 5.3 Crash Recovery 플로우

```
         ┌──────────────┐
         │ Daemon 시작   │
         └──────┬───────┘
                │
                ▼
┌─────────────────────────────────────────────┐
│   core::application::recovery              │
│  ┌──────────────────────────────────────┐   │
│  │ recovery.rs::recover_orphaned_jobs() │   │
│  │  1. RUNNING job 조회                 │   │
│  │  2. started_at < (now - 5분) 필터     │   │
│  │  3. execution_mode 확인              │   │
│  │     - IN_PROCESS: REQUEUED           │   │
│  │     - SUBPROCESS: PID 확인 후 KILL   │   │
│  └─────────────┬────────────────────────┘   │
└────────────────┼────────────────────────────┘
                 │
                 ▼
         ┌──────────────┐
         │ Job 복구 완료 │
         │ state=REQUEUED│
         └──────────────┘
```

---

## 6. 컴포넌트 상세

### 6.1 Core (핵심 비즈니스 로직)

**위치**: `crates/core/`

#### 6.1.1 Domain Layer

**파일**: `src/domain/`

**`job.rs`**:
```rust
pub struct Job {
    // 필드 정의 (섹션 4.1 참고)
}

impl Job {
    pub fn new(/* ... */) -> Self { /* ... */ }
    
    pub fn transition_to(&mut self, new_state: JobState) -> Result<()> {
        // 상태 전이 검증
        match (self.state, new_state) {
            (JobState::QUEUED, JobState::RUNNING) => Ok(()),
            (JobState::RUNNING, JobState::DONE) => Ok(()),
            // ...
            _ => Err(DomainError::InvalidStateTransition),
        }
    }
}
```

**책임**: Job 데이터 구조, 상태 전이 검증

**`queue.rs`**:
```rust
pub struct Queue {
    pub name: String,
    pub max_concurrent: Option<usize>,
}
```

**책임**: Queue 개념 정의

#### 6.1.2 Port Layer

**파일**: `src/port/`

**`job_repository.rs`**:
```rust
#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn create(&self, job: Job) -> Result<Job>;
    async fn find_by_id(&self, id: &JobId) -> Result<Option<Job>>;
    async fn pop_next(&self, queue: &str) -> Result<Option<Job>>;
    async fn update(&self, job: &Job) -> Result<()>;
    // ...
}

#[async_trait]
pub trait TransactionalJobRepository: Send + Sync {
    async fn in_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn JobRepository) -> BoxFuture<'_, Result<T>> + Send,
        T: Send;
}
```

**책임**: DB CRUD 인터페이스 정의

**`task_executor.rs`**:
```rust
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute(&self, job: &Job) -> Result<ExecutionResult>;
}

pub struct ExecutionResult {
    pub status: ExecutionStatus,  // Success | Failure
    pub duration_ms: u64,
    pub error_message: Option<String>,
}
```

**책임**: Job 실행 인터페이스

**`system_probe.rs`**:
```rust
#[async_trait]
pub trait SystemProbe: Send + Sync {
    async fn cpu_usage(&self) -> Result<f32>;   // 0.0 ~ 100.0
    async fn memory_usage(&self) -> Result<f32>; // 0.0 ~ 100.0
}
```

**책임**: 시스템 리소스 조회

#### 6.1.3 Application Layer

**파일**: `src/application/`

**`worker/mod.rs`**:
```rust
pub struct Worker {
    queue: String,
    job_repo: Arc<dyn JobRepository>,
    executor: Arc<dyn TaskExecutor>,
    system_probe: Arc<dyn SystemProbe>,
}

impl Worker {
    pub async fn run(&self, shutdown: Receiver<()>) -> Result<()> {
        loop {
            // 1. System throttling
            let cpu = self.system_probe.cpu_usage().await?;
            if cpu > 90.0 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            
            // 2. Pop next job
            let Some(job) = self.job_repo.pop_next(&self.queue).await? else {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            
            // 3. Execute
            let result = self.executor.execute(&job).await?;
            
            // 4. Update state
            let new_state = match result.status {
                ExecutionStatus::Success => JobState::DONE,
                ExecutionStatus::Failure => JobState::FAILED,
            };
            job.transition_to(new_state)?;
            self.job_repo.update(&job).await?;
        }
    }
}
```

**책임**: Job 실행 루프, System throttling

**`scheduler.rs`**:
```rust
pub struct Scheduler {
    job_repo: Arc<dyn JobRepository>,
    time_provider: Arc<dyn TimeProvider>,
}

impl Scheduler {
    pub async fn process_scheduled_jobs(&self) -> Result<()> {
        let now = self.time_provider.now();
        
        // SCHEDULED -> QUEUED (시간 도달)
        let jobs = self.job_repo.find_scheduled_jobs(now).await?;
        for mut job in jobs {
            job.transition_to(JobState::QUEUED)?;
            self.job_repo.update(&job).await?;
        }
        
        Ok(())
    }
}
```

**책임**: 예약된 Job 처리

**`recovery.rs`**:
```rust
pub struct Recovery {
    job_repo: Arc<dyn JobRepository>,
    time_provider: Arc<dyn TimeProvider>,
}

impl Recovery {
    pub async fn recover_orphaned_jobs(&self) -> Result<usize> {
        let cutoff = self.time_provider.now() - Duration::from_secs(300);
        
        let orphaned = self.job_repo.find_running_before(cutoff).await?;
        
        for mut job in orphaned {
            match job.execution_mode {
                ExecutionMode::IN_PROCESS => {
                    job.transition_to(JobState::REQUEUED)?;
                }
                ExecutionMode::SUBPROCESS => {
                    // Kill process if exists
                    if let Some(pid) = job.pid {
                        kill_process(pid);
                    }
                    job.transition_to(JobState::FAILED)?;
                }
            }
            self.job_repo.update(&job).await?;
        }
        
        Ok(orphaned.len())
    }
}
```

**책임**: Crash 복구

### 6.2 Infrastructure (구현체)

#### 6.2.1 infra-sqlite

**위치**: `crates/infra-sqlite/`

**`job_repository.rs`**:
```rust
pub struct SqliteJobRepository {
    pool: SqlitePool,
}

#[async_trait]
impl JobRepository for SqliteJobRepository {
    async fn pop_next(&self, queue: &str) -> Result<Option<Job>> {
        let job = sqlx::query_as!(
            JobRow,
            r#"
            UPDATE jobs
            SET state = 'RUNNING', started_at = ?
            WHERE id = (
                SELECT id FROM jobs
                WHERE state = 'QUEUED' AND queue = ?
                ORDER BY priority DESC, created_at ASC
                LIMIT 1
            )
            RETURNING *
            "#,
            now, queue
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(job.map(Into::into))
    }
}
```

**책임**: JobRepository 구현 (SQLite)

**`migration.rs`**:
```rust
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    let version = get_current_version(pool).await?;
    
    if version < 1 {
        apply_migration(pool, include_str!("../migrations/001_initial_schema.sql")).await?;
    }
    if version < 2 {
        apply_migration(pool, include_str!("../migrations/002_add_execution_retry.sql")).await?;
    }
    // ...
    
    Ok(())
}
```

**책임**: DB 스키마 마이그레이션

#### 6.2.2 infra-system

**위치**: `crates/infra-system/`

**`subprocess_executor.rs`**:
```rust
pub struct SubprocessExecutor;

#[async_trait]
impl TaskExecutor for SubprocessExecutor {
    async fn execute(&self, job: &Job) -> Result<ExecutionResult> {
        let log_path = job.log_path.clone().unwrap_or_default();
        let log_file = File::create(&log_path)?;
        
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&job.payload["command"])
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file))
            .spawn()?;
        
        let status = child.wait().await?;
        
        Ok(ExecutionResult {
            status: if status.success() {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Failure
            },
            duration_ms: /* ... */,
            error_message: None,
        })
    }
}
```

**책임**: Subprocess 실행

**`system_probe_impl.rs`**:
```rust
pub struct SystemProbeImpl {
    sys: Arc<Mutex<System>>,
}

#[async_trait]
impl SystemProbe for SystemProbeImpl {
    async fn cpu_usage(&self) -> Result<f32> {
        let mut sys = self.sys.lock().await;
        sys.refresh_cpu();
        Ok(sys.global_cpu_info().cpu_usage())
    }
    
    async fn memory_usage(&self) -> Result<f32> {
        let mut sys = self.sys.lock().await;
        sys.refresh_memory();
        let used = sys.used_memory() as f32;
        let total = sys.total_memory() as f32;
        Ok((used / total) * 100.0)
    }
}
```

**책임**: 시스템 리소스 조회 (sysinfo)

### 6.3 API Layer

#### 6.3.1 api-rpc

**위치**: `crates/api-rpc/`

**`server.rs`**:
```rust
pub struct RpcServer {
    handler: Arc<RpcHandler>,
}

impl RpcServer {
    pub async fn start(self) -> Result<ServerHandle> {
        let server = Server::builder()
            .build("127.0.0.1:9527")
            .await?;
        
        let mut module = RpcModule::new(());
        
        // Register methods
        let handler = self.handler.clone();
        module.register_async_method("dev.enqueue.v1", move |params, _, _| {
            let handler = handler.clone();
            async move {
                let req: EnqueueRequest = params.parse()?;
                handler.enqueue(req).await
            }
        })?;
        
        // ...
        
        Ok(server.start(module))
    }
}
```

**책임**: JSON-RPC 서버 (jsonrpsee)

**`handler.rs`**:
```rust
pub struct RpcHandler {
    tx_job_repo: Arc<dyn TransactionalJobRepository>,
    id_provider: Arc<dyn IdProvider>,
}

impl RpcHandler {
    pub async fn enqueue(&self, req: EnqueueRequest) -> Result<EnqueueResponse> {
        let job_id = self.id_provider.generate();
        
        let job = Job::new(
            job_id,
            req.queue,
            req.job_type,
            req.subject_key,
            req.payload,
            req.priority,
        );
        
        let created = self.tx_job_repo
            .in_transaction(|repo| {
                Box::pin(async move {
                    repo.create(job).await
                })
            })
            .await?;
        
        Ok(EnqueueResponse {
            job_id: created.id,
            state: created.state,
            queue: created.queue,
        })
    }
}
```

**책임**: RPC 요청 처리

### 6.4 Daemon (Composition Root)

**위치**: `crates/daemon/src/main.rs`

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Telemetry 초기화
    telemetry::init()?;
    
    // 2. DB 연결
    let pool = SqlitePool::connect(&db_path).await?;
    run_migrations(&pool).await?;
    
    // 3. Infrastructure 생성
    let job_repo = Arc::new(SqliteJobRepository::new(pool.clone()));
    let tx_job_repo = Arc::new(SqliteTransactionalJobRepository::new(pool.clone()));
    let executor = Arc::new(SubprocessExecutor::new());
    let system_probe = Arc::new(SystemProbeImpl::new());
    let time_provider = Arc::new(SystemTimeProvider::new());
    let id_provider = Arc::new(UuidProvider::new());
    let maintenance = Arc::new(MaintenanceImpl::new(pool.clone()));
    
    // 4. Application 생성
    let worker = Worker::new(
        "default".to_string(),
        job_repo.clone(),
        executor,
        system_probe.clone(),
    );
    
    let scheduler = Scheduler::new(job_repo.clone(), time_provider.clone());
    let recovery = Recovery::new(job_repo.clone(), time_provider.clone());
    
    // 5. Crash recovery
    recovery.recover_orphaned_jobs().await?;
    
    // 6. RPC 서버 시작
    let rpc_server = RpcServer::new(
        RpcServerConfig::default(),
        tx_job_repo,
        job_repo.clone(),
        id_provider,
        time_provider,
        maintenance,
    );
    let _rpc_handle = rpc_server.start().await?;
    
    // 7. Worker 시작
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());
    tokio::spawn(async move {
        worker.run(shutdown_rx).await
    });
    
    // 8. Scheduler 시작
    tokio::spawn(async move {
        scheduler.run().await
    });
    
    // 9. Graceful shutdown
    tokio::signal::ctrl_c().await?;
    shutdown_tx.send(())?;
    
    Ok(())
}
```

**책임**: 모든 의존성 조립 및 시작

---

## 7. 통신 프로토콜

### 7.1 JSON-RPC 2.0

**전송**: HTTP POST (TCP 9527 포트)

**요청 형식**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "dev.enqueue.v1",
  "params": {
    "job_type": "INDEX_FILE",
    "queue": "default",
    "subject_key": "repo::file.py",
    "payload": {"path": "file.py"},
    "priority": 5
  }
}
```

**응답 형식** (성공):
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "job_id": "c4b2bb3a-72f0-4f1e-8f6b-3aa95b2e18c3",
    "state": "QUEUED",
    "queue": "default"
  }
}
```

**응답 형식** (에러):
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": 4001,
    "message": "Invalid parameters",
    "data": {
      "kind": "validation",
      "details": "job_type is required"
    }
  }
}
```

### 7.2 RPC 메서드

| 메서드 | 설명 | Request | Response |
|--------|------|---------|----------|
| `dev.enqueue.v1` | Job 등록 | `EnqueueRequest` | `EnqueueResponse` |
| `dev.cancel.v1` | Job 취소 | `CancelRequest` | `CancelResponse` |
| `logs.tail.v1` | 로그 조회 | `TailLogsRequest` | `TailLogsResponse` |
| `admin.stats.v1` | 통계 조회 | `StatsRequest` | `StatsResponse` |
| `admin.maintenance.v1` | GC 실행 | `MaintenanceRequest` | `MaintenanceResponse` |

---

## 8. 확장 포인트

### 8.1 새 Port 추가

**시나리오**: Notification 기능 추가

**1단계**: Port 정의 (`crates/core/src/port/notifier.rs`)
```rust
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(&self, job_id: &JobId, message: &str) -> Result<()>;
}
```

**2단계**: Application에서 사용 (`crates/core/src/application/worker/mod.rs`)
```rust
pub struct Worker {
    // ... 기존 필드
    notifier: Arc<dyn Notifier>,
}

impl Worker {
    pub async fn run(&self) -> Result<()> {
        // ...
        if result.status == ExecutionStatus::Success {
            self.notifier.notify(&job.id, "Job completed").await?;
        }
    }
}
```

**3단계**: Infrastructure 구현 (`crates/infra-notify/src/lib.rs`)
```rust
pub struct EmailNotifier {
    smtp_config: SmtpConfig,
}

#[async_trait]
impl Notifier for EmailNotifier {
    async fn notify(&self, job_id: &JobId, message: &str) -> Result<()> {
        // 이메일 전송 로직
        Ok(())
    }
}
```

**4단계**: Daemon에서 주입 (`crates/daemon/src/main.rs`)
```rust
let notifier = Arc::new(EmailNotifier::new(smtp_config));
let worker = Worker::new(/* ... */, notifier);
```

### 8.2 새 RPC 메서드 추가

**1단계**: DTO 정의 (`crates/api-rpc/src/types.rs`)
```rust
#[derive(Serialize, Deserialize)]
pub struct PauseJobRequest {
    pub job_id: JobId,
}

#[derive(Serialize, Deserialize)]
pub struct PauseJobResponse {
    pub job_id: JobId,
    pub paused: bool,
}
```

**2단계**: Handler 구현 (`crates/api-rpc/src/handler.rs`)
```rust
impl RpcHandler {
    pub async fn pause_job(&self, req: PauseJobRequest) -> Result<PauseJobResponse> {
        let mut job = self.job_repo.find_by_id(&req.job_id).await?
            .ok_or(Error::NotFound)?;
        
        job.transition_to(JobState::PAUSED)?;
        self.job_repo.update(&job).await?;
        
        Ok(PauseJobResponse {
            job_id: job.id,
            paused: true,
        })
    }
}
```

**3단계**: RPC 등록 (`crates/api-rpc/src/server.rs`)
```rust
module.register_async_method("dev.pause.v1", move |params, _, _| {
    let handler = handler.clone();
    async move {
        let req: PauseJobRequest = params.parse()?;
        handler.pause_job(req).await
    }
})?;
```

### 8.3 새 DB 마이그레이션

**1단계**: SQL 작성 (`crates/infra-sqlite/migrations/005_add_pause_state.sql`)
```sql
-- Add PAUSED state
-- Extend state enum if needed (SQLite uses TEXT)

ALTER TABLE jobs ADD COLUMN paused_at INTEGER;
CREATE INDEX idx_jobs_paused ON jobs(state, paused_at) WHERE state = 'PAUSED';
```

**2단계**: Migration 적용 (`crates/infra-sqlite/src/migration.rs`)
```rust
if version < 5 {
    apply_migration(pool, include_str!("../migrations/005_add_pause_state.sql")).await?;
}
```

---

## 9. 개발 가이드

### 9.1 로컬 개발 환경 설정

```bash
# 1. Rust 설치
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 프로젝트 클론
git clone <repo-url>
cd semantica-task-engine

# 3. 빌드
cargo build

# 4. 테스트
cargo test

# 5. Daemon 실행 (개발 모드)
RUST_LOG=debug cargo run --package semantica-daemon
```

### 9.2 코드 스타일

```bash
# 포맷팅
cargo fmt

# Lint 검사
cargo clippy -- -D warnings

# 문서 생성
cargo doc --open
```

### 9.3 새 기능 추가 순서

1. **ADR 작성** (`ADR_v2/ADR-XXX-feature-name.md`)
2. **Domain 모델 정의** (`crates/core/src/domain/`)
3. **Port 정의** (`crates/core/src/port/`)
4. **Application 로직** (`crates/core/src/application/`)
5. **Infrastructure 구현** (`crates/infra-*/`)
6. **API 추가** (`crates/api-rpc/`)
7. **Daemon 주입** (`crates/daemon/src/main.rs`)
8. **테스트 작성** (`crates/integration-tests/`)
9. **문서 업데이트** (`README.md`, `docs/`)

### 9.4 디버깅

**로그 레벨 설정**:
```bash
RUST_LOG=semantica=debug,sqlx=info cargo run --package semantica-daemon
```

**DB 직접 조회**:
```bash
sqlite3 ~/.semantica/meta.db

sqlite> SELECT id, job_type, state, priority FROM jobs ORDER BY created_at DESC LIMIT 10;
```

**RPC 테스트 (curl)**:
```bash
curl -X POST http://localhost:9527 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "dev.enqueue.v1",
    "params": {
      "job_type": "TEST",
      "queue": "default",
      "subject_key": "test-1",
      "payload": {},
      "priority": 0
    }
  }'
```

---

## 10. 테스트 전략

### 10.1 테스트 계층

| 계층 | 위치 | 도구 | 범위 |
|------|------|------|------|
| **Unit** | `crates/core/src/**/*.rs` | `#[test]` | Domain, Application 로직 |
| **Contract** | `crates/sdk/src/**/*.rs` | `#[test]` | SDK API 호환성 |
| **Integration** | `crates/integration-tests/` | `#[tokio::test]` | DB, Worker, RPC |
| **Golden** | `tests/golden/` | Snapshot | Scheduler, Planner |
| **E2E** | `tests/integration_e2e.rs` | `#[tokio::test]` | Daemon + SDK |

### 10.2 테스트 실행

```bash
# 모든 테스트
cargo test

# 특정 패키지만
cargo test --package semantica-core

# 통합 테스트만
cargo test --package integration-tests

# 특정 테스트
cargo test test_enqueue_supersede

# 로그 출력 포함
RUST_LOG=debug cargo test -- --nocapture
```

### 10.3 Mock 사용

```rust
use mockall::mock;

mock! {
    JobRepo {}
    
    #[async_trait]
    impl JobRepository for JobRepo {
        async fn create(&self, job: Job) -> Result<Job>;
        async fn find_by_id(&self, id: &JobId) -> Result<Option<Job>>;
    }
}

#[tokio::test]
async fn test_worker_with_mock() {
    let mut mock_repo = MockJobRepo::new();
    mock_repo.expect_pop_next()
        .returning(|_| Ok(Some(Job::new(/* ... */))));
    
    let worker = Worker::new("default".into(), Arc::new(mock_repo), /* ... */);
    // ...
}
```

---

## 11. 요약 (AI 체크리스트)

AI가 코드 수정 시 확인할 사항:

### 11.1 아키텍처 규칙

- [ ] Domain은 **외부 의존성 없음** (순수 Rust)
- [ ] Application은 **Port만 사용** (구체적 구현 의존 X)
- [ ] Infrastructure는 **Port를 구현**
- [ ] Daemon은 **모든 의존성 조립**

### 11.2 코드 작성 규칙

- [ ] `pub fn`에는 **docstring 필수**
- [ ] 함수 길이 **< 30줄**
- [ ] 모듈 크기 **< 200줄**
- [ ] `.unwrap()` 금지 (`.expect()` 또는 `?` 사용)
- [ ] `panic!()` 금지 (lib crate)
- [ ] 에러는 `thiserror` (lib), `anyhow` (bin)

### 11.3 데이터베이스

- [ ] 모든 상태 변경은 **트랜잭션 사용**
- [ ] `pop_next`는 **UPDATE ... RETURNING** (원자성)
- [ ] Index 누락 시 추가 (`idx_jobs_pop`, `idx_jobs_subject_generation`)

### 11.4 테스트

- [ ] 코드 수정 시 **테스트 추가/수정**
- [ ] Unit 테스트: Domain, Application
- [ ] Integration 테스트: DB, Worker
- [ ] `cargo test` 통과 확인

### 11.5 Phase 준수

- [ ] Phase 1 기능만 사용 (현재)
- [ ] Phase 2+ 필드는 사용 X (`execution_mode`, `pid`, ...)
- [ ] 새 기능은 ADR 작성 후 추가

---

## 12. 참고 문서

- [ADR-001: System Architecture](ADR_v2/ADR-001-system-architecture.md)
- [ADR-010: Database Persistence](ADR_v2/ADR-010-database-persistence.md)
- [ADR-020: API Contract](ADR_v2/ADR-020-api-contract.md)
- [API Specification](docs/api-spec.md)
- [Python SDK Guide](python-sdk/README.md)
- [Python SDK AI Context](python-sdk/AI_CONTEXT.md)

---

**문서 버전**: 1.0  
**프로젝트 버전**: 0.1.0 (Phase 4)  
**마지막 업데이트**: 2025-12-05

---

**이 문서로 AI는 다음을 할 수 있습니다**:
- ✅ 전체 시스템 아키텍처 이해
- ✅ 각 컴포넌트의 역할 파악
- ✅ 데이터 플로우 추적
- ✅ 새 기능 추가 방법 습득
- ✅ 코드 수정 시 어디를 고쳐야 할지 판단
- ✅ 테스트 작성 방법 습득

