# 작업 완료 체크리스트

## 코드 작성 후 필수 작업

### 1. 포맷팅
```bash
cargo fmt
```

### 2. 린팅
```bash
cargo clippy -- -D warnings
```

### 3. 테스트
```bash
cargo test
```

### 4. 빌드 검증
```bash
cargo build
```

## 추가 권장 사항

### 문서화
- 공개 API에 rustdoc 주석 추가
- 복잡한 로직에 설명 주석 추가
- 필요시 ADR 업데이트

### 에러 핸들링 검증
- 모든 에러 케이스 처리 확인
- Result 타입 적절히 사용
- 에러 메시지 명확성 확인

### 코드 리뷰 전
- 불필요한 주석/코드 제거
- TODO/FIXME 확인
- 네이밍 일관성 확인

### 커밋 전
- git status로 변경 사항 확인
- 의미 있는 커밋 메시지 작성
- .gitignore 적절성 확인