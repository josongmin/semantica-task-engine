// End-to-End Integration Test
// Tests the full flow: Enqueue -> Worker -> Done

use semantica_task_engine::{
    application::{DevTaskService, Worker},
    domain::JobState,
    infrastructure::sqlite::{create_pool, run_migrations, SqliteJobRepository},
    port::{JobRepository, TransactionalJobRepository, time_provider::SystemTimeProvider},
};
use std::sync::Arc;

async fn setup_test_system() -> (
    DevTaskService,
    Worker,
    Arc<dyn JobRepository>,
) {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let sqlite_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let repo_tx: Arc<dyn TransactionalJobRepository> = sqlite_repo.clone();
    let repo_job: Arc<dyn JobRepository> = sqlite_repo;
    
    let service = DevTaskService::new(repo_tx);
    let worker = Worker::new("test_queue", repo_job.clone());

    (service, worker, repo_job)
}

#[tokio::test]
async fn test_e2e_job_flow() {
    let (service, worker, repo) = setup_test_system().await;

    // Enqueue a job via service
    let req = semantica_task_engine::application::dev_task::EnqueueRequest {
        job_type: "TEST_JOB".to_string(),
        queue: "test_queue".to_string(),
        subject_key: "test::file::path".to_string(),
        payload: serde_json::json!({
            "file": "test.rs",
            "action": "index"
        }),
        priority: 0,
    };

    let job_id = service.enqueue(req).await.unwrap();

    // Verify job is queued (but pop_next already set it to RUNNING)
    // So we need to check if job exists and can be processed
    let processed = worker.process_next_job().await.unwrap();
    assert!(processed);

    // Verify job is done
    let job = repo.find_by_id(&job_id).await.unwrap().unwrap();
    assert_eq!(job.state, JobState::Done);
    assert!(job.started_at.is_some());
    assert!(job.finished_at.is_some());
}

#[tokio::test]
async fn test_e2e_supersede_flow() {
    let (service, _worker, repo) = setup_test_system().await;

    // Enqueue 3 jobs with same subject_key
    let mut job_ids = vec![];
    for i in 1..=3 {
        let req = semantica_task_engine::application::dev_task::EnqueueRequest {
            job_type: "INDEX_FILE".to_string(),
            queue: "test_queue".to_string(),
            subject_key: "same::subject".to_string(),
            payload: serde_json::json!({"version": i}),
            priority: 0,
        };
        let job_id = service.enqueue(req).await.unwrap();
        job_ids.push(job_id);
        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // First two should be superseded
    let job1 = repo.find_by_id(&job_ids[0]).await.unwrap().unwrap();
    let job2 = repo.find_by_id(&job_ids[1]).await.unwrap().unwrap();
    assert_eq!(job1.state, JobState::Superseded);
    assert_eq!(job2.state, JobState::Superseded);

    // Last one should be queued
    let job3 = repo.find_by_id(&job_ids[2]).await.unwrap().unwrap();
    assert_eq!(job3.state, JobState::Queued);
}

#[tokio::test]
async fn test_e2e_priority_ordering() {
    let (service, worker, repo) = setup_test_system().await;

    // Enqueue jobs with different priorities
    let req_low = semantica_task_engine::application::dev_task::EnqueueRequest {
        job_type: "TEST".to_string(),
        queue: "test_queue".to_string(),
        subject_key: "low::priority".to_string(),
        payload: serde_json::json!({}),
        priority: 0,
    };

    let req_high = semantica_task_engine::application::dev_task::EnqueueRequest {
        job_type: "TEST".to_string(),
        queue: "test_queue".to_string(),
        subject_key: "high::priority".to_string(),
        payload: serde_json::json!({}),
        priority: 10,
    };

    // Enqueue low priority first
    let job_id_low = service.enqueue(req_low).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    // Then high priority
    let job_id_high = service.enqueue(req_high).await.unwrap();

    // Worker should process high priority first
    worker.process_next_job().await.unwrap();

    let job_high = repo.find_by_id(&job_id_high).await.unwrap().unwrap();
    let job_low = repo.find_by_id(&job_id_low).await.unwrap().unwrap();

    assert_eq!(job_high.state, JobState::Done);
    assert_eq!(job_low.state, JobState::Queued); // Not processed yet
}

#[tokio::test]
async fn test_e2e_multiple_jobs() {
    let (service, worker, repo) = setup_test_system().await;

    // Enqueue 10 jobs
    let mut job_ids = vec![];
    for i in 0..10 {
        let req = semantica_task_engine::application::dev_task::EnqueueRequest {
            job_type: "BATCH_JOB".to_string(),
            queue: "test_queue".to_string(),
            subject_key: format!("job::{}", i),
            payload: serde_json::json!({"index": i}),
            priority: 0,
        };
        let job_id = service.enqueue(req).await.unwrap();
        job_ids.push(job_id);
    }

    // Process all jobs
    for _ in 0..10 {
        worker.process_next_job().await.unwrap();
    }

    // All jobs should be done
    let done_count = repo.count_by_state("test_queue", JobState::Done).await.unwrap();
    assert_eq!(done_count, 10);
}
