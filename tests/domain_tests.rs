// Domain Layer Integration Tests
// Phase 1: Job entity and state transitions

use semantica_task_engine::domain::{Job, JobPayload, JobState, JobType};

#[test]
fn test_job_creation_and_state() {
    let job = Job::new(
        "test-job-1",
        1000, // created_at
        "test_queue",
        JobType::new("TEST_TYPE"),
        "test::subject::key",
        1,
        JobPayload::new(serde_json::json!({
            "test": "data",
            "value": 123
        })),
    );

    assert_eq!(job.state, JobState::Queued);
    assert_eq!(job.queue, "test_queue");
    assert_eq!(job.generation, 1);
    assert_eq!(job.priority, 0);
    assert!(job.started_at.is_none());
    assert!(job.finished_at.is_none());
}

#[test]
fn test_job_lifecycle() {
    let mut job = Job::new(
        "test-job-2",
        2000,
        "code_intel",
        JobType::new("INDEX_FILE"),
        "repo::file.rs",
        1,
        JobPayload::new(serde_json::json!({"path": "file.rs"})),
    );

    // Initial state: QUEUED
    assert_eq!(job.state, JobState::Queued);

    // Start job: QUEUED -> RUNNING
    assert!(job.start(3000).is_ok());
    assert_eq!(job.state, JobState::Running);
    assert!(job.started_at.is_some());

    // Complete job: RUNNING -> DONE
    assert!(job.complete(4000).is_ok());
    assert_eq!(job.state, JobState::Done);
    assert!(job.finished_at.is_some());
}

#[test]
fn test_invalid_state_transitions() {
    let mut job = Job::new(
        "test-job-3",
        5000,
        "test_queue",
        JobType::new("TEST"),
        "subject",
        1,
        JobPayload::new(serde_json::json!({})),
    );

    // Cannot complete without starting
    assert!(job.complete(6000).is_err());

    // Start successfully
    assert!(job.start(7000).is_ok());

    // Cannot start again
    assert!(job.start(8000).is_err());
}

#[test]
fn test_supersede() {
    let mut job = Job::new(
        "test-job-4",
        9000,
        "test_queue",
        JobType::new("TEST"),
        "subject",
        1,
        JobPayload::new(serde_json::json!({})),
    );

    job.supersede(10000);
    assert_eq!(job.state, JobState::Superseded);
    assert!(job.finished_at.is_some());
}

#[test]
fn test_fail() {
    let mut job = Job::new(
        "test-job-5",
        11000,
        "test_queue",
        JobType::new("TEST"),
        "subject",
        1,
        JobPayload::new(serde_json::json!({})),
    );

    job.fail(12000);
    assert_eq!(job.state, JobState::Failed);
    assert!(job.finished_at.is_some());
}

#[test]
fn test_job_serialization() {
    let job = Job::new(
        "test-job-6",
        13000,
        "test_queue",
        JobType::new("TEST"),
        "subject",
        1,
        JobPayload::new(serde_json::json!({"key": "value"})),
    );

    // Test that Job can be serialized/deserialized
    let json = serde_json::to_string(&job).expect("serialize");
    let deserialized: Job = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(job.id, deserialized.id);
    assert_eq!(job.queue, deserialized.queue);
    assert_eq!(job.state, deserialized.state);
}
