# 코드 스타일 및 컨벤션

## Rust 스타일 가이드
- Rust 공식 스타일 가이드 준수
- rustfmt로 포맷팅 자동화
- clippy로 코드 품질 검증

## 네이밍 컨벤션
- 타입: PascalCase (예: JobRepository, TaskEngine)
- 함수/변수: snake_case (예: enqueue_task, job_id)
- 상수: SCREAMING_SNAKE_CASE (예: VERSION, MAX_RETRIES)
- 모듈: snake_case (예: task_engine, job_repository)

## 에러 핸들링
- 라이브러리 레벨: thiserror로 명시적 에러 타입 정의
- 바이너리 레벨: anyhow로 간편한 에러 전파
- Result<T, E> 타입 적극 활용
- ? 연산자로 에러 전파

## 비동기 패턴
- async/await 사용
- tokio 런타임 기반
- Send + Sync trait bound 명시

## Serde 활용
- #[derive(Serialize, Deserialize)] 적극 사용
- DTO 구조체에 serde 속성 활용
- JSON 직렬화 기본

## 문서화
- 공개 API에 대한 rustdoc 주석 작성
- 복잡한 로직에 대한 인라인 주석
- ADR 문서로 아키텍처 결정 기록

## 의존성 규칙 (Hexagonal)
- core는 외부 의존성 없음 (trait만 정의)
- infra는 core의 trait 구현
- api는 core의 application 서비스 사용
- daemon은 모든 것을 조립