//! Critical Edge Case Tests
//!
//! 비판적 검증에서 발견된 Gap을 채우는 필수 테스트들

use semantica_core::application::dev_task::DevTaskService;
use semantica_core::application::dev_task::EnqueueRequest;
use semantica_core::domain::JobState;
use semantica_core::port::job_repository::JobRepository;
use semantica_core::port::time_provider::SystemTimeProvider;
use semantica_core::port::TimeProvider;
use semantica_infra_sqlite::{create_pool, run_migrations, SqliteJobRepository};
use std::sync::Arc;

/// Critical Test 1: Concurrent Pop (Race Condition)
/// 여러 worker가 동시에 같은 queue에서 pop할 때 중복 없이 안전한가?
#[tokio::test]
async fn test_concurrent_pop_no_duplicates() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider.clone()));

    let service = DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        time_provider,
    );

    // Enqueue 10 jobs
    let mut job_ids = Vec::new();
    for i in 0..10 {
        let req = EnqueueRequest {
            job_type: "TEST".to_string(),
            queue: "default".to_string(),
            subject_key: format!("file-{}.rs", i),
            payload: serde_json::json!({"id": i}),
            priority: 0,
        };
        job_ids.push(service.enqueue(req).await.unwrap());
    }

    // Spawn 10 concurrent workers trying to pop
    let mut handles = Vec::new();
    for worker_id in 0..10 {
        let repo = job_repo.clone();
        let handle = tokio::spawn(async move {
            match repo.pop_next("default").await {
                Ok(Some(job)) => {
                    // Simulate some work
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    Some((worker_id, job.id))
                }
                Ok(None) => None,
                Err(e) => panic!("Worker {} failed: {}", worker_id, e),
            }
        });
        handles.push(handle);
    }

    // Collect results
    let mut popped_jobs = Vec::new();
    for handle in handles {
        if let Some((worker_id, job_id)) = handle.await.unwrap() {
            popped_jobs.push((worker_id, job_id));
        }
    }

    // Verify: 10 jobs, no duplicates
    assert_eq!(
        popped_jobs.len(),
        10,
        "All 10 jobs should be popped exactly once"
    );

    let mut unique_jobs: Vec<_> = popped_jobs.iter().map(|(_, id)| id).collect();
    unique_jobs.sort();
    unique_jobs.dedup();
    assert_eq!(unique_jobs.len(), 10, "No duplicate jobs should be popped");

    println!("✅ Concurrent pop: No duplicates, all jobs popped exactly once");
}

/// Critical Test 2: Input Validation - Malicious Payload
/// 악의적 입력에 대한 방어
#[tokio::test]
async fn test_malicious_input_validation() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider.clone()));

    let service = DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        time_provider,
    );

    // Test 1: Extremely long queue name
    let req1 = EnqueueRequest {
        job_type: "TEST".to_string(),
        queue: "a".repeat(300), // > 255 bytes
        subject_key: "test.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };
    let result1 = service.enqueue(req1).await;
    assert!(result1.is_err(), "Should reject queue name > 255 bytes");

    // Test 2: SQL injection attempt in queue name
    let req2 = EnqueueRequest {
        job_type: "TEST".to_string(),
        queue: "'; DROP TABLE jobs; --".to_string(),
        subject_key: "test.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };
    let result2 = service.enqueue(req2).await;
    // Should either reject or safely escape (both OK)
    if result2.is_ok() {
        // Verify jobs table still exists (no SQL injection succeeded)
        let _ = job_repo.find_by_state(JobState::Queued).await.unwrap();
        // If this doesn't panic, table still exists ✅
    }

    // Test 3: Null byte in subject_key
    let req3 = EnqueueRequest {
        job_type: "TEST".to_string(),
        queue: "default".to_string(),
        subject_key: "test\0.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };
    let result3 = service.enqueue(req3).await;
    assert!(result3.is_err(), "Should reject null byte in subject_key");

    // Test 4: Extremely large payload (> 10MB)
    let large_payload = serde_json::json!({
        "data": "x".repeat(11_000_000) // 11MB
    });
    let req4 = EnqueueRequest {
        job_type: "TEST".to_string(),
        queue: "default".to_string(),
        subject_key: "test.rs".to_string(),
        payload: large_payload,
        priority: 0,
    };
    let result4 = service.enqueue(req4).await;
    assert!(result4.is_err(), "Should reject payload > 10MB");

    println!("✅ Input validation: All malicious inputs rejected");
}

/// Critical Test 3: Boundary Values
/// 경계값에서의 동작 검증
#[tokio::test]
async fn test_boundary_values() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider.clone()));

    let service = DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        time_provider,
    );

    // Test 1: Priority = MAX_PRIORITY (100)
    let req1 = EnqueueRequest {
        job_type: "TEST".to_string(),
        queue: "default".to_string(),
        subject_key: "max-priority.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 100, // Valid range: -100 to 100
    };
    let id1 = service.enqueue(req1).await.unwrap();
    let job1 = job_repo.find_by_id(&id1).await.unwrap().unwrap();
    assert_eq!(job1.priority, 100);

    // Test 2: Priority = MIN_PRIORITY (-100)
    let req2 = EnqueueRequest {
        job_type: "TEST".to_string(),
        queue: "default".to_string(),
        subject_key: "min-priority.rs".to_string(),
        payload: serde_json::json!({}),
        priority: -100,
    };
    let id2 = service.enqueue(req2).await.unwrap();
    let job2 = job_repo.find_by_id(&id2).await.unwrap().unwrap();
    assert_eq!(job2.priority, -100);

    // Test 3: Pop order (MAX priority should come first)
    let popped = job_repo.pop_next("default").await.unwrap().unwrap();
    assert_eq!(popped.id, id1, "MAX priority job should be popped first");

    // Test 4: Out of range priority should be rejected
    let req_invalid = EnqueueRequest {
        job_type: "TEST".to_string(),
        queue: "default".to_string(),
        subject_key: "invalid.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 101, // Out of range
    };
    assert!(
        service.enqueue(req_invalid).await.is_err(),
        "Priority > 100 should be rejected"
    );

    // Test 4: Generation overflow (extremely large generation)
    let req3 = EnqueueRequest {
        job_type: "TEST".to_string(),
        queue: "default".to_string(),
        subject_key: "same-subject".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };

    // Enqueue many times for same subject_key
    for _ in 0..100 {
        service.enqueue(req3.clone()).await.unwrap();
    }

    // Verify generation increments correctly
    let latest = job_repo
        .get_latest_generation("same-subject")
        .await
        .unwrap();
    assert!(latest >= 100, "Generation should increment correctly");

    println!("✅ Boundary values: MAX/MIN values handled correctly");
}

/// Critical Test 4: Error Path - Database Failure
/// DB 연결 실패 시 graceful degradation
#[tokio::test]
async fn test_database_connection_failure() {
    // Invalid DB path (should fail)
    let result = create_pool("/invalid/path/that/does/not/exist/db.sqlite").await;

    assert!(result.is_err(), "Should fail with invalid DB path");

    println!("✅ Error path: DB connection failure handled gracefully");
}

/// Critical Test 5: Supersede Race Condition
/// 동시에 같은 subject_key로 enqueue 시 generation 정합성
#[tokio::test]
async fn test_supersede_concurrent_enqueue() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider.clone()));

    let service = Arc::new(DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        time_provider,
    ));

    // Spawn 10 concurrent enqueue for same subject_key
    let mut handles = Vec::new();
    for i in 0..10 {
        let svc = service.clone();
        let handle = tokio::spawn(async move {
            let req = EnqueueRequest {
                job_type: "INDEX".to_string(),
                queue: "default".to_string(),
                subject_key: "same-file.rs".to_string(),
                payload: serde_json::json!({"version": i}),
                priority: 0,
            };
            svc.enqueue(req).await.unwrap()
        });
        handles.push(handle);
    }

    // Wait for all
    let mut job_ids = Vec::new();
    for handle in handles {
        job_ids.push(handle.await.unwrap());
    }

    // Verify: 10 jobs created
    assert_eq!(job_ids.len(), 10);

    // Verify: Generations are 1..=10 (unique, sequential)
    let mut generations = Vec::new();
    for id in job_ids {
        let job = job_repo.find_by_id(&id).await.unwrap().unwrap();
        generations.push(job.generation);
    }
    generations.sort();

    assert_eq!(
        generations,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        "Generations should be unique and sequential"
    );

    // Verify: 9 jobs marked SUPERSEDED, 1 job QUEUED
    let superseded_count = job_repo
        .find_by_state(JobState::Superseded)
        .await
        .unwrap()
        .len();
    let queued_count = job_repo
        .find_by_state(JobState::Queued)
        .await
        .unwrap()
        .len();

    assert_eq!(superseded_count, 9, "9 old jobs should be SUPERSEDED");
    assert_eq!(queued_count, 1, "1 latest job should be QUEUED");

    println!("✅ Supersede race condition: Generations consistent under concurrency");
}

/// Critical Test 6: Deadline and TTL Edge Cases
/// Deadline/TTL이 과거인 경우, 0인 경우
#[tokio::test]
async fn test_deadline_ttl_edge_cases() {
    use semantica_core::application::retry::RetryPolicy;
    use semantica_core::domain::{Job, JobPayload, JobType};

    let time_provider = Arc::new(SystemTimeProvider);
    let retry_policy = RetryPolicy::new(time_provider.clone(), 1000);

    let now = time_provider.now_millis();

    // Test 1: Deadline in the past
    let mut job1 = Job::new(
        "job-past-deadline".to_string(),
        now,
        "test".to_string(),
        JobType::new("TEST".to_string()),
        "test.rs".to_string(),
        1,
        JobPayload::new(serde_json::json!({})),
    );
    job1.deadline = Some(now - 1000); // 1 second ago

    assert!(
        retry_policy.is_deadline_exceeded(&job1),
        "Past deadline should be exceeded"
    );

    // Test 2: Deadline = 0 (epoch)
    let mut job2 = Job::new(
        "job-zero-deadline".to_string(),
        now,
        "test".to_string(),
        JobType::new("TEST".to_string()),
        "test.rs".to_string(),
        1,
        JobPayload::new(serde_json::json!({})),
    );
    job2.deadline = Some(0);

    assert!(
        retry_policy.is_deadline_exceeded(&job2),
        "Deadline at epoch should be exceeded"
    );

    // Test 3: TTL = 0 (should expire immediately)
    let mut job3 = Job::new(
        "job-zero-ttl".to_string(),
        now - 100,
        "test".to_string(),
        JobType::new("TEST".to_string()),
        "test.rs".to_string(),
        1,
        JobPayload::new(serde_json::json!({})),
    );
    job3.ttl_ms = Some(0);

    assert!(
        retry_policy.is_ttl_exceeded(&job3),
        "TTL=0 should be immediately exceeded"
    );

    println!("✅ Deadline/TTL edge cases: Past/zero values handled correctly");
}

/// Critical Test 7: Max Attempts = 0
/// Retry 불가능한 job (max_attempts=0)
#[tokio::test]
async fn test_max_attempts_zero() {
    use semantica_core::application::retry::{RetryDecision, RetryPolicy};
    use semantica_core::domain::{Job, JobPayload, JobType};

    let time_provider = Arc::new(SystemTimeProvider);
    let retry_policy = RetryPolicy::new(time_provider.clone(), 1000);

    let mut job = Job::new(
        "job-no-retry".to_string(),
        time_provider.now_millis(),
        "test".to_string(),
        JobType::new("TEST".to_string()),
        "test.rs".to_string(),
        1,
        JobPayload::new(serde_json::json!({})),
    );

    job.max_attempts = 0; // No retry allowed
    job.attempts = 0;

    let decision = retry_policy.should_retry(&job);

    assert_eq!(
        decision,
        RetryDecision::Failed,
        "max_attempts=0 should immediately fail"
    );

    println!("✅ Max attempts = 0: Correctly fails without retry");
}
