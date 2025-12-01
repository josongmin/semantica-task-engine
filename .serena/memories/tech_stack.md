# 기술 스택

## 언어 및 에디션
- Rust 2021 Edition (최소 1.70.0)

## 핵심 의존성

### Runtime
- tokio (1.37.0) - 비동기 런타임, features: ["full"]
- futures (0.3.30) - 비동기 유틸리티

### 직렬화
- serde (1.0.197) - features: ["derive"]
- serde_json (1.0.116)

### 에러 핸들링
- thiserror (1.0.57) - 라이브러리/도메인 레벨 에러
- anyhow - 바이너리 레벨 (향후 추가 예정)

### 로깅
- log (0.4.21)
- env_logger (0.11.3)
- tracing (향후 마이그레이션 예정)

### 개발 의존성
- tokio-test (0.4.3)
- mockall (0.12.1)

## 향후 추가 예정
- sqlx - SQLite 비동기 지원
- jsonrpsee - JSON-RPC 서버/클라이언트
- sysinfo - 시스템 리소스 모니터링
- directories - OS별 경로 관리
- chrono - 시간 관리
- uuid - ID 생성
- config - 설정 파일 관리