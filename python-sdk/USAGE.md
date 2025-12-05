# Semantica Task SDK - 완벽 사용 가이드

## 📑 목차

1. [설치](#1-설치)
2. [기본 사용법](#2-기본-사용법)
3. [고급 사용법](#3-고급-사용법)
4. [배포](#4-배포)
5. [문제 해결](#5-문제-해결)

---

## 1. 설치

### 1-1. 최종 사용자 (배포 후)

```bash
# PyPI에서 설치 (배포 후)
pip install semantica-task-engine

# 확인
python3 -c "import semantica_task_engine; print('✅ 설치 완료')"
```

### 1-2. 개발자 (로컬)

```bash
# 1. 저장소 클론
git clone https://github.com/your-org/semantica-task-engine.git
cd semantica-task-engine

# 2. Python SDK 설치
cd python-sdk
pip install -e .

# 3. Daemon 빌드 (선택 사항 - auto_start 사용 시 필요)
cd ..
cargo build --release

# 4. 확인
python3 -c "import semantica_task_engine; print('✅ 설치 완료')"
```

---

## 2. 기본 사용법

### 2-1. 자동 Daemon (추천 ⭐)

**Daemon을 신경 쓸 필요 없음!**

```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest

async def main():
    # Daemon이 없으면 자동으로 시작!
    async with SemanticaTaskClient() as client:
        
        # Job 등록
        response = await client.enqueue(
            EnqueueRequest(
                job_type="INDEX_FILE",
                queue="default",
                subject_key="src/main.py",
                payload={"path": "src/main.py", "repo": "my-project"},
                priority=5
            )
        )
        
        print(f"✅ Job ID: {response.job_id}")
        print(f"   State: {response.state}")
        print(f"   Queue: {response.queue}")
        
        # 로그 조회 (1초 후)
        await asyncio.sleep(1)
        logs = await client.tail_logs(response.job_id, lines=10)
        
        print(f"\n📋 Logs:")
        for line in logs.lines:
            print(f"   {line}")

asyncio.run(main())
```

**실행**:
```bash
python my_script.py
# ✅ Daemon 자동 시작
# ✅ Job 실행
# ✅ 로그 출력
# ✅ Daemon 자동 종료
```

---

### 2-2. 수동 Daemon (프로덕션)

**Daemon을 별도로 관리**

**터미널 1 - Daemon 시작**:
```bash
# 방법 1: justfile 사용
just start

# 방법 2: dev.sh 사용
./dev.sh start

# 방법 3: cargo 직접 실행
SEMANTICA_RPC_PORT=9527 cargo run --release --package semantica-daemon

# 확인
just status
# 또는
lsof -i :9527
```

**터미널 2 - Python 스크립트**:
```python
import asyncio
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest

async def main():
    # auto_start_daemon=False (Daemon 수동 관리)
    async with SemanticaTaskClient(auto_start_daemon=False) as client:
        response = await client.enqueue(
            EnqueueRequest(
                job_type="INDEX_FILE",
                queue="default",
                subject_key="src/main.py",
                payload={"path": "src/main.py"}
            )
        )
        print(f"Job ID: {response.job_id}")

asyncio.run(main())
```

**실행**:
```bash
python my_script.py
```

**Daemon 종료**:
```bash
just stop
# 또는
./dev.sh stop
```

---

### 2-3. API 메서드

#### ① `enqueue()` - Job 등록

```python
response = await client.enqueue(
    EnqueueRequest(
        job_type="INDEX_FILE",        # Job 타입 (필수)
        queue="default",               # Queue 이름 (필수)
        subject_key="src/main.py",     # Supersede 키 (필수)
        payload={"path": "..."},       # Job 데이터 (필수)
        priority=5                     # 우선순위 0-10 (선택, 기본값: 0)
    )
)

print(response.job_id)   # "job-abc123"
print(response.state)    # "QUEUED"
print(response.queue)    # "default"
```

#### ② `cancel()` - Job 취소

```python
response = await client.cancel("job-abc123")

print(response.job_id)      # "job-abc123"
print(response.cancelled)   # True
```

#### ③ `tail_logs()` - 로그 조회

```python
response = await client.tail_logs("job-abc123", lines=50)

print(response.job_id)      # "job-abc123"
print(response.log_path)    # "/path/to/log"
for line in response.lines:
    print(line)             # 로그 라인
```

---

## 3. 고급 사용법

### 3-1. 에러 핸들링

```python
from semantica_task_engine import SemanticaTaskClient, EnqueueRequest
from semantica_task_engine.errors import ConnectionError, RpcError

async def main():
    try:
        async with SemanticaTaskClient() as client:
            response = await client.enqueue(...)
            
    except ConnectionError as e:
        print(f"❌ 연결 실패: {e}")
        # Daemon이 실행 중인지 확인
        
    except RpcError as e:
        print(f"❌ RPC 에러: {e.message} (code: {e.code})")
        if e.code == 4001:
            print("   Job이 이미 존재함")
        elif e.code == 4004:
            print("   Job을 찾을 수 없음")
        
    except Exception as e:
        print(f"❌ 알 수 없는 에러: {e}")
```

### 3-2. 커스텀 설정

```python
# 포트 변경
async with SemanticaTaskClient(
    url="http://localhost:7701",
    timeout=60.0,
    auto_start_daemon=True
) as client:
    ...
```

```bash
# 환경변수로 설정
export SEMANTICA_DAEMON_PATH=/usr/local/bin/semantica
export SEMANTICA_RPC_PORT=7701
export RUST_LOG=debug

python my_script.py
```

### 3-3. 여러 Job 동시 처리

```python
async def main():
    async with SemanticaTaskClient() as client:
        
        # 여러 Job 등록
        jobs = []
        for i in range(10):
            response = await client.enqueue(
                EnqueueRequest(
                    job_type="BATCH_PROCESS",
                    queue="batch",
                    subject_key=f"item-{i}",
                    payload={"id": i}
                )
            )
            jobs.append(response.job_id)
        
        print(f"✅ {len(jobs)} jobs enqueued")
        
        # 모든 Job 로그 확인
        await asyncio.sleep(5)
        for job_id in jobs:
            logs = await client.tail_logs(job_id, lines=5)
            print(f"\nJob {job_id}:")
            for line in logs.lines:
                print(f"  {line}")
```

### 3-4. Superseding (중복 제거)

```python
# 같은 subject_key를 가진 Job은 이전 것을 취소하고 새로운 것으로 대체
async with SemanticaTaskClient() as client:
    
    # 첫 번째 Job
    r1 = await client.enqueue(
        EnqueueRequest(
            job_type="INDEX_FILE",
            queue="default",
            subject_key="src/main.py",  # 같은 키
            payload={"version": 1}
        )
    )
    
    # 두 번째 Job (첫 번째를 Supersede)
    r2 = await client.enqueue(
        EnqueueRequest(
            job_type="INDEX_FILE",
            queue="default",
            subject_key="src/main.py",  # 같은 키
            payload={"version": 2}
        )
    )
    
    # r1은 자동으로 SUPERSEDED 상태가 되고, r2만 실행됨
```

---

## 4. 배포

### 4-1. Python SDK 배포 (PyPI)

```bash
cd python-sdk

# 방법 1: 스크립트 사용 (추천)
./deploy.sh
# -> 1번 선택: TestPyPI (테스트)
# -> 2번 선택: PyPI (정식)

# 방법 2: 수동
python -m build
python -m twine upload dist/*
```

**배포 후 사용자 설치**:
```bash
pip install semantica-task-engine
```

### 4-2. Daemon 배포 (Docker)

```bash
# Docker 이미지 빌드
docker build -t your-dockerhub/semantica:0.1.0 .
docker build -t your-dockerhub/semantica:latest .

# Docker Hub 푸시
docker push your-dockerhub/semantica:0.1.0
docker push your-dockerhub/semantica:latest
```

**배포 후 사용자 실행**:
```bash
docker run -d \
  -p 9527:9527 \
  -v semantica-data:/var/lib/semantica \
  your-dockerhub/semantica:latest
```

### 4-3. Daemon 배포 (Binary)

```bash
# 릴리스 빌드
cargo build --release

# Binary 위치
ls -lh target/release/semantica

# GitHub Releases에 업로드
# https://github.com/your-org/semantica/releases
```

**배포 후 사용자 설치**:
```bash
# 다운로드
wget https://github.com/your-org/semantica/releases/download/v0.1.0/semantica-linux-x86_64

# 설치
chmod +x semantica-linux-x86_64
sudo mv semantica-linux-x86_64 /usr/local/bin/semantica

# 실행
semantica
```

---

## 5. 문제 해결

### 5-1. Daemon이 시작되지 않음

**증상**:
```
ConnectionError: HTTP error: ConnectError
```

**해결**:
```bash
# 1. Daemon 상태 확인
just status
# 또는
lsof -i :9527

# 2. 포트 충돌 확인
lsof -i :9527
# 다른 프로세스가 사용 중이면 종료

# 3. 수동으로 Daemon 시작
just start

# 4. 로그 확인
tail -f ~/.semantica/logs/daemon.log
```

### 5-2. ModuleNotFoundError

**증상**:
```
ModuleNotFoundError: No module named 'semantica'
```

**해결**:
```bash
# SDK 설치 확인
pip list | grep semantica

# 없으면 설치
cd python-sdk
pip install -e .

# 확인
python3 -c "import semantica_task_engine"
```

### 5-3. RPC Error 4001 (Already Exists)

**증상**:
```
RpcError: Job already exists (code: 4001)
```

**해결**:
```python
# subject_key를 변경하거나 기존 Job을 취소
await client.cancel(old_job_id)
await client.enqueue(...)
```

### 5-4. Database Migration Error

**증상**:
```
Error: Migration failed: duplicate column name
```

**해결**:
```bash
# DB 초기화
rm -rf ~/.semantica/meta.db*
mkdir -p ~/.semantica

# Daemon 재시작
just restart
```

### 5-5. Port Conflict

**증상**:
```
Error: Address already in use
```

**해결**:
```bash
# 1. 포트 사용 중인 프로세스 확인
lsof -i :9527

# 2. 다른 포트로 실행
SEMANTICA_RPC_PORT=7701 just start

# 3. Python 코드에서 포트 변경
async with SemanticaTaskClient("http://localhost:7701") as client:
    ...
```

---

## 📋 Quick Reference

### 설치
```bash
pip install semantica-task-engine  # 배포 후
# 또는
cd python-sdk && pip install -e .  # 개발용
```

### 자동 Daemon (간편)
```python
async with SemanticaTaskClient() as client:
    response = await client.enqueue(...)
```

### 수동 Daemon (프로덕션)
```bash
just start  # Daemon 시작
```
```python
async with SemanticaTaskClient(auto_start_daemon=False) as client:
    response = await client.enqueue(...)
```

### API
```python
# Job 등록
await client.enqueue(EnqueueRequest(...))

# Job 취소
await client.cancel(job_id)

# 로그 조회
await client.tail_logs(job_id, lines=50)
```

### 배포
```bash
cd python-sdk && ./deploy.sh  # PyPI
docker push your-dockerhub/semantica:latest  # Docker
```

---

## 🔗 참고 문서

- [API 명세](../docs/api-spec.md)
- [아키텍처 가이드](../AI_ARCHITECTURE_GUIDE.md)
- [Python SDK README](./README.md)
- [빠른 시작](./QUICKSTART.md)
- [AI 컨텍스트](./AI_CONTEXT.md)

---

**끝!** 🎉

