# Semantica Task Engine - 운영 가이드

Phase 4: Production Operations

## 1. 설치 및 배포

### 빌드

```bash
# Release 빌드 (최적화 + strip)
cargo build --release

# OpenTelemetry 포함 빌드
cargo build --release --features telemetry

# 바이너리 위치
# - Daemon: target/release/semantica
# - CLI: target/release/semantica-cli
```

### 배포

```bash
# 설치 디렉터리 생성
mkdir -p ~/.semantica

# 바이너리 복사
cp target/release/semantica /usr/local/bin/
cp target/release/semantica-cli /usr/local/bin/

# 권한 설정
chmod +x /usr/local/bin/semantica
chmod +x /usr/local/bin/semantica-cli
```

---

## 2. Daemon 실행

### 기본 실행

```bash
# Foreground 실행
semantica

# Background 실행
nohup semantica &> ~/.semantica/daemon.log &
```

### 환경변수 설정

```bash
# DB 경로 (기본: ~/.semantica/meta.db)
export SEMANTICA_DB_PATH="/custom/path/meta.db"

# RPC 포트 (기본: 9527)
export SEMANTICA_RPC_PORT=9999

# 로그 포맷 (기본: pretty, 프로덕션: json)
export SEMANTICA_LOG_FORMAT=json

# 로그 레벨
export RUST_LOG=semantica=info,sqlx=warn

# OpenTelemetry (optional)
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=semantica-prod
```

### systemd 서비스 (Linux)

```ini
# /etc/systemd/system/semantica.service
[Unit]
Description=Semantica Task Engine
After=network.target

[Service]
Type=simple
User=semantica
Group=semantica
WorkingDirectory=/home/semantica
ExecStart=/usr/local/bin/semantica
Restart=always
RestartSec=10

# Environment
Environment="SEMANTICA_DB_PATH=/var/lib/semantica/meta.db"
Environment="SEMANTICA_LOG_FORMAT=json"
Environment="RUST_LOG=semantica=info"

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/semantica /var/log/semantica

[Install]
WantedBy=multi-user.target
```

```bash
# 서비스 등록 및 시작
sudo systemctl daemon-reload
sudo systemctl enable semantica
sudo systemctl start semantica

# 상태 확인
sudo systemctl status semantica

# 로그 확인
sudo journalctl -u semantica -f
```

### launchd (macOS)

```xml
<!-- ~/Library/LaunchAgents/com.semantica.daemon.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.semantica.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/semantica</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>SEMANTICA_LOG_FORMAT</key>
        <string>json</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/Users/you/.semantica/daemon.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/you/.semantica/daemon.err</string>
</dict>
</plist>
```

```bash
# 서비스 등록
launchctl load ~/Library/LaunchAgents/com.semantica.daemon.plist

# 서비스 시작
launchctl start com.semantica.daemon

# 상태 확인
launchctl list | grep semantica
```

---

## 3. CLI 사용법

### 작업 등록 (Enqueue)

```bash
# 기본 사용
semantica-cli enqueue \
  --job-type INDEX_FILE \
  --queue default \
  --subject "src/main.rs" \
  --priority 0 \
  --payload '{"path": "src/main.rs"}'

# 높은 우선순위 작업
semantica-cli enqueue \
  --job-type BUILD \
  --queue ci \
  --subject "release-v1.0" \
  --priority 10 \
  --payload '{"target": "release", "features": ["telemetry"]}'
```

### 작업 취소

```bash
semantica-cli cancel <job-id>
```

### 로그 조회

```bash
# 마지막 100줄
semantica-cli logs <job-id>

# 마지막 500줄
semantica-cli logs <job-id> -n 500
```

### 시스템 상태

```bash
semantica-cli status
```

---

## 4. 모니터링

### 로그 모니터링

```bash
# JSON 로그 파싱 (jq)
tail -f ~/.semantica/daemon.log | jq '.fields | select(.event == "job_state_changed")'

# 에러만 필터링
tail -f ~/.semantica/daemon.log | jq 'select(.level == "ERROR")'

# Job 완료 알림
tail -f ~/.semantica/daemon.log | jq 'select(.fields.state == "DONE") | .fields.job_id'
```

### 메트릭 수집 (OpenTelemetry)

```bash
# Jaeger 실행 (로컬 테스트)
docker run -d --name jaeger \
  -p 4317:4317 \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest

# Daemon 실행 (Telemetry 활성화)
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
OTEL_SERVICE_NAME=semantica-dev \
  semantica

# Jaeger UI 접속
open http://localhost:16686
```

### Health Check

```bash
# RPC 엔드포인트 확인
curl -X POST http://127.0.0.1:9527 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"dev.enqueue.v1","params":{},"id":1}'

# 응답 예시 (에러지만 서버는 살아있음)
# {"jsonrpc":"2.0","error":{"code":4001,"message":"..."},"id":1}
```

---

## 5. 데이터베이스 관리

### 백업

```bash
# DB 백업 (SQLite WAL 모드 안전)
sqlite3 ~/.semantica/meta.db ".backup ~/.semantica/backup-$(date +%Y%m%d).db"

# 압축 백업
tar -czf semantica-backup-$(date +%Y%m%d).tar.gz ~/.semantica/
```

### 복구

```bash
# 백업에서 복구
cp ~/.semantica/backup-20240101.db ~/.semantica/meta.db

# Daemon 재시작 (Crash Recovery 자동 실행)
systemctl restart semantica
```

### Maintenance

자동 실행:
- **VACUUM**: 24시간마다 (오전 3시)
- **GC**: 30일 이상 된 DONE/FAILED 작업 삭제
- **Artifact GC**: 참조 없는 아티팩트 삭제

수동 실행:
```bash
# TODO: admin.maintenance.v1 API 구현 후
semantica-cli maintenance --force-vacuum
```

### Migration 확인

```bash
# Migration 상태 확인
sqlite3 ~/.semantica/meta.db "SELECT name FROM sqlite_master WHERE type='table';"

# 현재 스키마 버전 확인
sqlite3 ~/.semantica/meta.db ".schema jobs" | head -20
```

---

## 6. 장애 대응

### Daemon이 시작 안 될 때

1. **DB 파일 권한 확인**
   ```bash
   ls -la ~/.semantica/meta.db
   # 예상: -rw-r--r-- 1 user user 123456 ...
   ```

2. **포트 충돌 확인**
   ```bash
   lsof -i :9527
   # 다른 프로세스가 사용 중이면 SEMANTICA_RPC_PORT 변경
   ```

3. **로그 확인**
   ```bash
   tail -100 ~/.semantica/daemon.log
   ```

### Daemon이 죽었을 때

```bash
# 1. Crash Recovery 자동 실행됨 (재시작 시)
systemctl restart semantica

# 2. DB 무결성 확인
sqlite3 ~/.semantica/meta.db "PRAGMA integrity_check;"

# 3. Orphaned Jobs 확인
sqlite3 ~/.semantica/meta.db \
  "SELECT id, state, job_type FROM jobs WHERE state = 'RUNNING';"
```

### 높은 메모리 사용

```bash
# 1. 메모리 사용량 확인
ps aux | grep semantica

# 2. DB 크기 확인
du -h ~/.semantica/meta.db

# 3. 수동 VACUUM
sqlite3 ~/.semantica/meta.db "VACUUM;"

# 4. 오래된 작업 정리
sqlite3 ~/.semantica/meta.db \
  "DELETE FROM jobs WHERE state IN ('DONE', 'FAILED') AND finished_at < datetime('now', '-7 days');"
```

### DB Lock 에러 (SQLITE_BUSY)

```bash
# 1. WAL 모드 확인
sqlite3 ~/.semantica/meta.db "PRAGMA journal_mode;"
# 예상 결과: wal

# 2. busy_timeout 확인 (코드 내 설정: 200ms)

# 3. 경합 확인
sqlite3 ~/.semantica/meta.db "PRAGMA wal_checkpoint(FULL);"
```

---

## 7. 성능 튜닝

### Worker 수 조정

현재 Phase 1: 단일 Worker (향후 확장 가능)

```rust
// TODO: Worker pool 구현 (Phase 5)
// worker_count = num_cpus::get();
```

### DB Connection Pool

```bash
# 환경변수로 조정 (향후)
export SEMANTICA_DB_MAX_CONNECTIONS=10
export SEMANTICA_DB_MIN_CONNECTIONS=2
```

### RPC 처리량

- 현재: 단일 TCP 리스너 (jsonrpsee 기본)
- 향후: Unix Socket 지원 (jsonrpsee 0.25+ 대기)

---

## 8. 보안

### IPC 보안

- **Layer 1**: Localhost only binding (`127.0.0.1`)
- **Layer 2**: OS-level peer auth (향후 Unix Socket)

### 환경변수 보호

```bash
# Secret을 포함한 환경변수는 파일로 관리
echo "OTEL_EXPORTER_OTLP_ENDPOINT=http://..." > ~/.semantica/.env
chmod 600 ~/.semantica/.env

# systemd에서 읽기
EnvironmentFile=/home/semantica/.env
```

### Subprocess Sandboxing

- PATH, HOME, USER, TERM만 상속
- 프로젝트별 allowlist는 향후 구현

---

## 9. 문제 해결 체크리스트

시작 실패:
- [ ] DB 디렉터리 존재 (`~/.semantica/`)
- [ ] DB 파일 권한 (읽기/쓰기)
- [ ] 포트 충돌 (9527)
- [ ] 로그 파일 확인

느린 성능:
- [ ] DB 크기 확인 (VACUUM 필요?)
- [ ] 오래된 작업 정리 (GC)
- [ ] CPU/메모리 사용량 확인

데이터 손실:
- [ ] WAL 모드 활성화 확인
- [ ] Crash Recovery 실행됨?
- [ ] DB 백업 복구

---

## 10. 리소스

- **ADR 문서**: `ADR_v2/` 디렉터리
- **API 명세**: `docs/api-spec.md`
- **테스트**: `cargo test --all`
- **로그**: `~/.semantica/daemon.log`
- **DB**: `~/.semantica/meta.db`

---

**최종 업데이트**: Phase 4 완료 (2024-12-01)

