# Semantica SDK

Rust client library for [Semantica Task Engine](../../README.md).

## 기능

- **Job 관리**: Enqueue, Cancel
- **로그 조회**: Tail logs
- **타입 안전**: Rust type safety with JSON-RPC
- **비동기**: Tokio-based async client
- **에러 처리**: Comprehensive error types

## 설치

```toml
[dependencies]
semantica-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## 사용법

### 기본 사용

```rust
use semantica_sdk::{SematicaClient, EnqueueRequest};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connect to daemon
    let client = SematicaClient::connect("http://127.0.0.1:9527").await?;

    // 2. Enqueue a job
    let response = client.enqueue(EnqueueRequest {
        job_type: "INDEX_FILE".to_string(),
        queue: "default".to_string(),
        subject_key: "src/main.rs".to_string(),
        priority: 0,
        payload: json!({"path": "src/main.rs"}),
    }).await?;

    println!("Job ID: {}", response.job_id);

    // 3. Tail logs
    let logs = client.tail_logs(&response.job_id, Some(50)).await?;
    for line in logs.lines {
        println!("{}", line);
    }

    // 4. Cancel if needed
    client.cancel(&response.job_id).await?;

    Ok(())
}
```

### 예제 실행

```bash
# 1. Daemon 실행 (터미널 1)
cargo run --package semantica-daemon

# 2. SDK 예제 실행 (터미널 2)
cargo run --package semantica-sdk --example simple
```

## API

### `SematicaClient`

```rust
// Connect to daemon
let client = SematicaClient::connect("http://127.0.0.1:9527").await?;

// Enqueue a job
let response = client.enqueue(EnqueueRequest { ... }).await?;

// Cancel a job
let response = client.cancel("job-123").await?;

// Tail logs
let response = client.tail_logs("job-123", Some(100)).await?;
```

## 에러 처리

```rust
use semantica_sdk::{SematicaClient, SdkError};

match client.enqueue(request).await {
    Ok(response) => println!("Success: {}", response.job_id),
    Err(SdkError::Connection(msg)) => eprintln!("Connection error: {}", msg),
    Err(SdkError::Rpc { code, message }) => eprintln!("RPC error {}: {}", code, message),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## 환경변수

SDK는 daemon의 RPC 엔드포인트를 환경변수로 설정할 수 있습니다:

```bash
export SEMANTICA_RPC_URL="http://localhost:9999"
```

기본값: `http://127.0.0.1:9527`

## 라이선스

MIT

