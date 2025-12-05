# Semantica Task Engine - 다른 프로젝트 통합 가이드

다른 Python/Node.js 프로젝트에서 Semantica Task Engine을 Docker로 함께 실행하는 방법.

## 1. 준비물

### Semantica Docker 이미지 빌드

Semantica 프로젝트 루트에서:

```bash
cd /path/to/semantica-task-engine

# 개발용 이미지 빌드 (빠름)
docker build -f Dockerfile.dev -t semantica-task-engine:latest .

# 또는 프로덕션 이미지 (최적화됨)
docker build -f Dockerfile -t semantica-task-engine:prod .
```

## 2. 통합 방법

### 방법 A: 기존 docker-compose.yml에 추가 (권장)

**당신의 프로젝트/docker-compose.yml**:

```yaml
version: '3.8'

services:
  # Semantica Daemon 추가
  semantica:
    image: semantica-task-engine:latest
    container_name: semantica-daemon
    ports:
      - "9527:9527"
    volumes:
      - semantica-data:/var/lib/semantica
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD-SHELL", "curl -f http://localhost:9527/health || exit 1"]
      interval: 10s
      timeout: 5s
      retries: 3
    networks:
      - myapp-network

  # 당신의 앱
  web:
    build: .
    depends_on:
      semantica:
        condition: service_healthy  # semantica 준비될 때까지 대기
    environment:
      - SEMANTICA_RPC_URL=http://semantica:9527
    networks:
      - myapp-network

volumes:
  semantica-data:

networks:
  myapp-network:
    driver: bridge
```

### 방법 B: 별도 compose 파일 사용

**semantica.docker-compose.yml** (프로젝트 루트에 생성):

```yaml
version: '3.8'

services:
  semantica:
    image: semantica-task-engine:latest
    container_name: semantica-daemon
    ports:
      - "9527:9527"
    volumes:
      - ./semantica-data:/var/lib/semantica
    environment:
      - RUST_LOG=info

volumes:
  semantica-data:
```

실행:

```bash
# Semantica만 별도 실행
docker-compose -f semantica.docker-compose.yml up -d

# 당신의 앱 실행 (localhost:9527로 연결)
docker-compose up
```

## 3. Python 코드에서 사용

### semantica_client.py 복사

```bash
# Semantica SDK를 당신의 프로젝트로 복사
cp /path/to/semantica-task-engine/examples/python/semantica_client.py \
   ./your_project/
```

### requirements.txt 또는 pyproject.toml

```txt
# requirements.txt
requests>=2.31.0
```

```toml
# pyproject.toml
[tool.poetry.dependencies]
requests = "^2.31.0"
```

### 코드 예제

```python
import os
from semantica_client import SemanticaTaskClient

# Docker Compose에서 환경변수 주입됨
client = SemanticaTaskClient(
    url=os.getenv("SEMANTICA_RPC_URL", "http://localhost:9527")
)

# Job 등록
response = client.enqueue(
    job_type="INDEX_FILE",
    queue="default",
    subject_key="src/main.py",
    payload={"path": "src/main.py", "action": "index"}
)

print(f"Job ID: {response['job_id']}")

# 로그 확인
logs = client.tail_logs(response['job_id'], limit=50)
print(logs)
```

## 4. 실행 순서

### Docker Compose 통합한 경우

```bash
# 한 번에 실행 (semantica + 당신의 앱)
docker-compose up

# 백그라운드 실행
docker-compose up -d

# 로그 확인
docker-compose logs semantica
docker-compose logs -f  # 실시간
```

### 별도 실행하는 경우

```bash
# 1. Semantica 먼저 실행
docker-compose -f semantica.docker-compose.yml up -d

# 2. 당신의 앱 실행
docker-compose up

# 3. 종료
docker-compose down
docker-compose -f semantica.docker-compose.yml down
```

## 5. 연결 확인

### 테스트 스크립트

**test_connection.py**:

```python
from semantica_client import SemanticaTaskClient, SemanticaError

try:
    client = SemanticaTaskClient()
    stats = client.stats()
    print(f"✅ Connected to Semantica! Queues: {stats}")
except SemanticaError as e:
    print(f"❌ Connection failed: {e}")
```

실행:

```bash
python test_connection.py
```

## 6. 헬스체크 / 디버깅

```bash
# Daemon 상태 확인
curl http://localhost:9527/health

# 또는 stats API
curl -X POST http://localhost:9527 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"admin.stats.v1","params":{}}'

# Container 로그
docker logs semantica-daemon

# Container 내부 접속
docker exec -it semantica-daemon bash
```

## 7. 환경변수 설정

### Docker Compose 환경변수

```yaml
services:
  semantica:
    environment:
      - SEMANTICA_DB_PATH=/var/lib/semantica/meta.db
      - SEMANTICA_RPC_PORT=9527
      - RUST_LOG=debug  # 로그 레벨
  
  your-app:
    environment:
      - SEMANTICA_RPC_URL=http://semantica:9527  # Python에서 사용
```

### .env 파일 (선택사항)

**.env**:
```bash
SEMANTICA_RPC_URL=http://localhost:9527
RUST_LOG=info
```

**docker-compose.yml**:
```yaml
services:
  semantica:
    env_file: .env
```

## 8. 데이터 영속화

### Named Volume (권장)

```yaml
volumes:
  semantica-data:  # Docker가 관리
```

### Bind Mount (개발용)

```yaml
services:
  semantica:
    volumes:
      - ./semantica-data:/var/lib/semantica  # 호스트 경로
```

데이터 위치:
- Named Volume: `docker volume inspect semantica-data`
- Bind Mount: `./semantica-data/` 디렉토리

## 9. 프로덕션 배포

```yaml
services:
  semantica:
    image: semantica-task-engine:prod  # 프로덕션 이미지
    restart: unless-stopped
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
    logging:
      driver: "json-file"
      options:
        max-size: "100m"
        max-file: "3"
```

## 10. 트러블슈팅

### "Connection refused"
- Daemon 실행 확인: `docker ps | grep semantica`
- 헬스체크 확인: `docker inspect semantica-daemon`
- 네트워크 확인: `docker network ls`

### "RPC Error"
- 로그 확인: `docker logs semantica-daemon`
- API 버전 확인: method 이름이 정확한지 (`dev.enqueue.v1`)

### "Too slow"
- CPU/메모리 제한 늘리기 (deploy.resources)
- 로그 레벨 낮추기 (RUST_LOG=info)
- Volume을 SSD에 배치

## 예제 프로젝트 구조

```
your-project/
├── docker-compose.yml        # semantica 서비스 추가됨
├── semantica_client.py       # SDK 복사
├── app.py                    # 당신의 앱 코드
├── requirements.txt          # requests 추가
├── Dockerfile                # 당신의 앱 이미지
└── .env                      # 환경변수 (선택)
```

## 참고 자료

- Semantica API Spec: `/docs/api-spec.md`
- Python SDK: `/examples/python/semantica_client.py`
- Docker Hub: (준비 중)


