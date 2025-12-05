# Semantica Task Engine - Quick Start Guide

**개발자가 5분 안에 시작할 수 있도록 작성된 가이드**

---

## 🚀 가장 빠른 시작 (3가지 방법)

### 방법 1: 대화형 스크립트 (추천 ⭐)

```bash
./dev.sh
```

메뉴가 나타나면 숫자를 선택:
- `1` - Daemon 시작
- `4` - Python SDK 테스트
- `7` - 상태 확인

### 방법 2: Just 명령어

```bash
# Daemon 시작
just start

# Python 예제 실행 (별도 터미널)
just py-example

# 상태 확인
just status
```

### 방법 3: Docker (설정 불필요)

```bash
# 한 번에 실행
just docker-dev

# 또는
docker-compose -f docker-compose.dev.yml up
```

---

## 📋 설치 (처음 한 번만)

### 1. Rust 설치

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Just 설치 (선택사항)

```bash
# macOS
brew install just

# Linux
cargo install just

# 또는 ./dev.sh 사용 (설치 불필요)
```

### 3. 프로젝트 빌드

```bash
# 빠른 빌드 (3-5분)
just build

# 또는
cargo build
```

---

## 🎯 사용 시나리오별 가이드

### 시나리오 1: Daemon만 실행하고 싶음

```bash
# 터미널 1: Daemon 시작
just start

# 확인
just status
```

**출력 예시**:
```
🚀 Starting Semantica Daemon...
✅ System ready. Waiting for tasks...
Press Ctrl+C to shutdown
```

### 시나리오 2: Python으로 Job 실행하고 싶음

```bash
# 터미널 1: Daemon 시작
just start

# 터미널 2: Python SDK 설치 + 예제 실행
just py-install
just py-example
```

**Python 코드**:
```python
import asyncio
from semantica import SemanticaTaskClient, EnqueueRequest

async def main():
    async with SemanticaTaskClient("http://localhost:9527") as client:
        response = await client.enqueue(
            EnqueueRequest(
                job_type="TEST",
                queue="default",
                subject_key="test-1",
                payload={"message": "hello"}
            )
        )
        print(f"Job ID: {response.job_id}")

asyncio.run(main())
```

### 시나리오 3: Docker로 전부 실행하고 싶음

```bash
# 한 줄로 끝
just docker-dev

# 백그라운드로 실행
just docker-dev-bg

# 로그 확인
just docker-logs

# 종료
just docker-stop
```

### 시나리오 4: 개발 중 (코드 수정 → 테스트)

```bash
# 코드 수정 후
just dev

# 또는 개별 실행
just fmt      # 포맷팅
just lint     # Lint 검사
just test     # 테스트

# Daemon 재시작
just restart
```

### 시나리오 5: DB 초기화하고 싶음

```bash
# DB 삭제
just db-reset

# Daemon 재시작 (DB 자동 재생성)
just start
```

---

## 📚 주요 명령어 치트시트

### Daemon 관련

| 명령어 | 설명 |
|--------|------|
| `just start` | Daemon 시작 (포트 9527) |
| `just start-debug` | 디버그 모드로 시작 |
| `just start-port 7701` | 포트 7701로 시작 |
| `just kill` | Daemon 종료 |
| `just restart` | Daemon 재시작 |
| `just status` | 실행 상태 확인 |

### Python SDK

| 명령어 | 설명 |
|--------|------|
| `just py-install` | Python SDK 설치 |
| `just py-example` | Python 예제 실행 |
| `just py-test` | Python 테스트 |

### Docker

| 명령어 | 설명 |
|--------|------|
| `just docker-dev` | Docker로 시작 |
| `just docker-dev-bg` | 백그라운드로 시작 |
| `just docker-stop` | Docker 종료 |
| `just docker-logs` | Docker 로그 확인 |
| `just docker-build` | Docker 이미지 빌드 |

### 개발

| 명령어 | 설명 |
|--------|------|
| `just dev` | 포맷 + Lint + 테스트 |
| `just fmt` | 코드 포맷팅 |
| `just lint` | Lint 검사 |
| `just test` | 테스트 실행 |
| `just build` | 빌드 (debug) |
| `just build-release` | 빌드 (release) |
| `just watch` | 파일 변경 시 자동 테스트 |

### DB

| 명령어 | 설명 |
|--------|------|
| `just db-reset` | DB 초기화 |
| `just db-view` | SQLite 콘솔 열기 |
| `just db-jobs` | 최근 Job 조회 |

---

## 🐛 트러블슈팅

### 문제 1: "Daemon이 실행 중이 아닙니다"

```bash
# 상태 확인
just status

# Daemon 시작
just start
```

### 문제 2: "포트가 이미 사용 중입니다"

```bash
# 실행 중인 프로세스 확인
lsof -i :9527

# 기존 Daemon 종료
just kill

# 재시작
just start
```

### 문제 3: "DB 에러 발생"

```bash
# DB 초기화
just db-reset

# Daemon 재시작
just start
```

### 문제 4: "Python SDK를 찾을 수 없습니다"

```bash
# SDK 설치
just py-install

# 또는
cd python-sdk
pip install -e .
```

### 문제 5: "Docker 빌드 실패"

```bash
# 캐시 없이 재빌드
docker-compose -f docker-compose.dev.yml build --no-cache

# 또는
just docker-build
```

---

## 🎨 개발 워크플로우

### 일반적인 개발 흐름

```bash
# 1. 코드 수정
vim crates/core/src/domain/job.rs

# 2. 포맷팅 + Lint + 테스트
just dev

# 3. Daemon 재시작
just restart

# 4. Python으로 테스트
just py-example

# 5. 확인
just status
```

### 빠른 반복 (Watch 모드)

```bash
# 터미널 1: 파일 변경 시 자동 테스트
just watch

# 터미널 2: 파일 변경 시 자동 재시작
just watch-daemon

# 터미널 3: 코드 수정
vim crates/core/src/...
```

---

## 📊 환경변수

### 자주 사용하는 환경변수

```bash
# DB 경로 변경
SEMANTICA_DB_PATH=/tmp/test.db just start

# 포트 변경
SEMANTICA_RPC_PORT=7701 just start

# 로그 형식 변경 (json)
SEMANTICA_LOG_FORMAT=json just start

# 로그 레벨 변경
RUST_LOG=debug just start

# 조합
SEMANTICA_RPC_PORT=7701 RUST_LOG=info just start
```

---

## 🔗 다음 단계

### 기본 사용법을 익혔다면:

1. **Python SDK 문서**: `python-sdk/README.md`
2. **전체 아키텍처**: `AI_ARCHITECTURE_GUIDE.md`
3. **API 명세**: `docs/api-spec.md`

### 코드 수정/기여하려면:

1. **ADR 문서**: `ADR_v2/` (설계 결정 문서)
2. **테스트 전략**: `ADR_v2/ADR-030-testing-strategy.md`
3. **개발 규칙**: 프로젝트 루트 `.ai/` 참고

---

## 🆘 도움말

### 명령어 도움말

```bash
# Just 명령어 목록
just

# dev.sh 도움말
./dev.sh --help

# 특정 명령어 도움말
just --help
```

### 문제가 해결되지 않으면:

1. `just status` 실행
2. 로그 확인: `just logs`
3. DB 확인: `just db-jobs`
4. Issue 생성 또는 문의

---

**Happy Coding! 🚀**

