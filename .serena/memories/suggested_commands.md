# 개발 명령어

## 빌드
```bash
# 디버그 빌드
cargo build

# 릴리스 빌드
cargo build --release
```

## 실행
```bash
# 디버그 모드 실행
cargo run

# 릴리스 모드 실행
cargo run --release
```

## 테스트
```bash
# 모든 테스트 실행
cargo test

# 통합 테스트만 실행
cargo test --test integration_test

# 특정 테스트 실행
cargo test <test_name>

# 로그 출력과 함께 테스트
cargo test -- --nocapture
```

## 코드 품질
```bash
# 포맷팅 검사
cargo fmt -- --check

# 포맷팅 적용
cargo fmt

# Clippy 린팅
cargo clippy

# Clippy (warning을 error로)
cargo clippy -- -D warnings
```

## 정리
```bash
# 빌드 아티팩트 삭제
cargo clean
```

## 의존성 관리
```bash
# 의존성 업데이트 확인
cargo update --dry-run

# 의존성 업데이트
cargo update
```

## macOS 특화 명령어
```bash
# 프로세스 확인
ps aux | grep semantica

# 파일 찾기
find . -name "*.rs"

# 로그 확인 (향후)
tail -f ~/.semantica/logs/*.log
```