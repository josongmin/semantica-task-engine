# Golden Test 전략 (ADR-013)

## 목적
고정된 입력(이벤트/명령) 시퀀스에 대해 기대되는 Job Queue/상태 스냅샷을 파일로 고정하여 회귀 검출

## 디렉토리 구조
```
tests/golden/
  planner/
    fs_event_simple.json
    fs_event_burst.json
    git_pr_merge.json
  scheduler/
    supersede_basic.json
    supersede_multi_repo.json
    backpressure_cpu.json
```

## 테스트 대상
1. Planner: 이벤트 → Job insert/supersede
2. Scheduler: QUEUED → RUNNING → DONE/FAILED/SUPERSEDED 시퀀스

## Golden 파일 포맷

### Planner Golden
```json
{
  "name": "fs_event_simple",
  "events": [
    { "type": "fs_change", "repo_id": "repo123", "path": "src/app/main.py", "kind": "modified" }
  ],
  "expected_jobs": [
    {
      "queue": "code_intel",
      "job_type": "INDEX_FILE_DELTA",
      "subject_key": "cli::repo123::src/app/main.py",
      "generation": 2,
      "state": "QUEUED"
    }
  ]
}
```

테스트 러너:
1. 빈 DB/Repo 상태에서 events 순서대로 주입
2. planner 실행
3. 최종 jobs snapshot을 expected_jobs와 비교

### Scheduler Golden
```json
{
  "name": "supersede_basic",
  "initial_jobs": [
    { "id": "j1", "queue": "code_intel", "subject_key": "repo::file", "generation": 1, "state": "QUEUED" },
    { "id": "j2", "queue": "code_intel", "subject_key": "repo::file", "generation": 2, "state": "QUEUED" }
  ],
  "ticks": 3,
  "expected": {
    "jobs": [
      { "id": "j1", "state": "SUPERSEDED" },
      { "id": "j2", "state": "DONE" }
    ]
  }
}
```

테스트 러너:
1. initial_jobs를 DB에 insert
2. scheduler loop를 ticks만큼 실행
3. 최종 state를 expected.jobs와 비교

## 테스트 러너 요구사항
- Golden 파일을 struct로 deserialize
- 테스트별 격리된 meta.db 사용
- 중요 필드만 비교 (queue, job_type, subject_key, generation, state)
- 예상하지 않은 추가 job 발견 시 failure
- cargo test --test golden_* 형태로 실행

## 유지보수 규칙
- Planner/Scheduler 로직 변경 시 diff 검토
- 의도한 변화면 Golden 파일 업데이트
- 새 기능 추가 시 최소 1개 이상 Golden 케이스 추가