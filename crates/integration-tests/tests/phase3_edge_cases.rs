//! Phase 3 Edge Cases - Persistence & Field Tests
//!
//! Tests that verify Phase 3 fields persist correctly in DB

use semantica_core::application::dev_task::DevTaskService;
use semantica_core::application::dev_task::EnqueueRequest;
use semantica_core::domain::JobState;
use semantica_core::port::job_repository::JobRepository;
use semantica_core::port::time_provider::SystemTimeProvider;
use semantica_infra_sqlite::{create_pool, run_migrations, SqliteJobRepository};
use std::sync::Arc;

/// Edge Case 1: schedule_at field persists
#[tokio::test]
async fn test_schedule_at_persists() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let service = DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        Arc::new(semantica_core::port::time_provider::SystemTimeProvider),
    );

    let req = EnqueueRequest {
        job_type: "test".to_string(),
        queue: "default".to_string(),
        subject_key: "test.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };

    let job_id = service.enqueue(req).await.unwrap();

    // Set schedule_at
    let future = chrono::Utc::now().timestamp_millis() + 3600000;
    let mut job = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    job.schedule_at = Some(future);
    job_repo.update(&job).await.unwrap();

    // Verify
    let job_after = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    assert_eq!(job_after.schedule_at, Some(future));

    println!("✅ Edge Case 1: schedule_at persists");
}

/// Edge Case 2: wait_for_idle field persists
#[tokio::test]
async fn test_wait_for_idle_persists() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let service = DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        Arc::new(semantica_core::port::time_provider::SystemTimeProvider),
    );

    let req = EnqueueRequest {
        job_type: "test".to_string(),
        queue: "default".to_string(),
        subject_key: "test.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };

    let job_id = service.enqueue(req).await.unwrap();

    // Set wait_for_idle
    let mut job = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    job.wait_for_idle = true;
    job_repo.update(&job).await.unwrap();

    // Verify
    let job_after = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    assert!(job_after.wait_for_idle);

    println!("✅ Edge Case 2: wait_for_idle persists");
}

/// Edge Case 3: require_charging field persists
#[tokio::test]
async fn test_require_charging_persists() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let service = DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        Arc::new(semantica_core::port::time_provider::SystemTimeProvider),
    );

    let req = EnqueueRequest {
        job_type: "test".to_string(),
        queue: "default".to_string(),
        subject_key: "test.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };

    let job_id = service.enqueue(req).await.unwrap();

    // Set require_charging
    let mut job = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    job.require_charging = true;
    job_repo.update(&job).await.unwrap();

    // Verify
    let job_after = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    assert!(job_after.require_charging);

    println!("✅ Edge Case 3: require_charging persists");
}

/// Edge Case 4: wait_for_event field persists
#[tokio::test]
async fn test_wait_for_event_persists() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let service = DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        Arc::new(semantica_core::port::time_provider::SystemTimeProvider),
    );

    let req = EnqueueRequest {
        job_type: "test".to_string(),
        queue: "default".to_string(),
        subject_key: "test.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };

    let job_id = service.enqueue(req).await.unwrap();

    // Set wait_for_event
    let mut job = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    job.wait_for_event = Some("file_changed".to_string());
    job_repo.update(&job).await.unwrap();

    // Verify
    let job_after = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    assert_eq!(job_after.wait_for_event, Some("file_changed".to_string()));

    println!("✅ Edge Case 4: wait_for_event persists");
}

/// Edge Case 5: Multiple conditions can be set together
#[tokio::test]
async fn test_multiple_conditions_persist() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let service = DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        Arc::new(semantica_core::port::time_provider::SystemTimeProvider),
    );

    let req = EnqueueRequest {
        job_type: "test".to_string(),
        queue: "default".to_string(),
        subject_key: "test.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };

    let job_id = service.enqueue(req).await.unwrap();

    // Set multiple conditions
    let future = chrono::Utc::now().timestamp_millis() + 3600000;
    let mut job = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    job.schedule_at = Some(future);
    job.wait_for_idle = true;
    job.require_charging = true;
    job.wait_for_event = Some("event".to_string());
    job_repo.update(&job).await.unwrap();

    // Verify all conditions persist
    let job_after = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    assert_eq!(job_after.schedule_at, Some(future));
    assert!(job_after.wait_for_idle);
    assert!(job_after.require_charging);
    assert_eq!(job_after.wait_for_event, Some("event".to_string()));

    println!("✅ Edge Case 5: Multiple conditions persist");
}

/// Edge Case 6: Default values are correct
#[tokio::test]
async fn test_default_values() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let service = DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        Arc::new(semantica_core::port::time_provider::SystemTimeProvider),
    );

    let req = EnqueueRequest {
        job_type: "test".to_string(),
        queue: "default".to_string(),
        subject_key: "test.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };

    let job_id = service.enqueue(req).await.unwrap();

    // Verify default values
    let job = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    assert_eq!(job.schedule_at, None);
    assert!(!job.wait_for_idle);
    assert!(!job.require_charging);
    assert_eq!(job.wait_for_event, None);

    println!("✅ Edge Case 6: Default values correct");
}

/// Edge Case 7: Fields survive state transitions
#[tokio::test]
async fn test_fields_survive_state_transitions() {
    let pool = create_pool(":memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let job_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let service = DevTaskService::new(
        job_repo.clone(),
        Arc::new(semantica_core::port::id_provider::UuidProvider),
        Arc::new(semantica_core::port::time_provider::SystemTimeProvider),
    );

    let req = EnqueueRequest {
        job_type: "test".to_string(),
        queue: "default".to_string(),
        subject_key: "test.rs".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };

    let job_id = service.enqueue(req).await.unwrap();

    // Set conditions
    let future = chrono::Utc::now().timestamp_millis() + 3600000;
    let mut job = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    job.schedule_at = Some(future);
    job.wait_for_idle = true;
    job_repo.update(&job).await.unwrap();

    // Change state to RUNNING
    let mut job = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    job.state = JobState::Running;
    job_repo.update(&job).await.unwrap();

    // Verify conditions still exist
    let job = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    assert_eq!(job.state, JobState::Running);
    assert_eq!(job.schedule_at, Some(future));
    assert!(job.wait_for_idle);

    // Change state back to QUEUED
    let mut job = job;
    job.state = JobState::Queued;
    job_repo.update(&job).await.unwrap();

    // Verify conditions still exist
    let job = job_repo.find_by_id(&job_id).await.unwrap().unwrap();
    assert_eq!(job.state, JobState::Queued);
    assert_eq!(job.schedule_at, Some(future));
    assert!(job.wait_for_idle);

    println!("✅ Edge Case 7: Fields survive state transitions");
}
