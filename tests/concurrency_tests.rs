// Concurrency and Race Condition Tests

use semantica_task_engine::{
    application::{DevTaskService, Worker, shutdown_channel},
    domain::JobState,
    infrastructure::sqlite::{create_pool, run_migrations, SqliteJobRepository},
    port::{JobRepository, TransactionalJobRepository, time_provider::SystemTimeProvider},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

#[tokio::test]
async fn test_sequential_enqueue_supersede() {
    // Test generation-based supersede logic
    // Note: SQLite in-memory has limited write concurrency, so we test sequentially
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let sqlite_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let repo_tx: Arc<dyn TransactionalJobRepository> = sqlite_repo.clone();
    let repo_job: Arc<dyn JobRepository> = sqlite_repo;

    let service = DevTaskService::new(repo_tx);

    // Enqueue 5 jobs sequentially for the same subject
    let mut job_ids = vec![];
    for i in 0..5 {
        let req = semantica_task_engine::application::dev_task::EnqueueRequest {
            job_type: "SUPERSEDE_TEST".to_string(),
            queue: "test_queue".to_string(),
            subject_key: "concurrent::subject".to_string(),
            payload: serde_json::json!({"index": i}),
            priority: 0,
        };
        let job_id = service.enqueue(req).await.unwrap();
        job_ids.push(job_id);
    }

    assert_eq!(job_ids.len(), 5);

    // Check that exactly 1 job is QUEUED and 4 are SUPERSEDED
    let queued_count = repo_job.count_by_state("test_queue", JobState::Queued).await.unwrap();
    let superseded_count = repo_job.count_by_state("test_queue", JobState::Superseded).await.unwrap();

    assert_eq!(queued_count, 1, "Expected exactly 1 QUEUED job");
    assert_eq!(superseded_count, 4, "Expected exactly 4 SUPERSEDED jobs");

    // Verify the last job is the one still queued
    let last_job = repo_job.find_by_id(&job_ids[4]).await.unwrap().unwrap();
    assert_eq!(last_job.state, JobState::Queued);
    assert_eq!(last_job.generation, 5);
}

#[tokio::test]
async fn test_concurrent_workers_pop_next() {
    // Test that pop_next is atomic and prevents duplicate processing
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let sqlite_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let repo_tx: Arc<dyn TransactionalJobRepository> = sqlite_repo.clone();
    let repo_job: Arc<dyn JobRepository> = sqlite_repo;

    let service = DevTaskService::new(repo_tx);

    // Enqueue 5 jobs
    for i in 0..5 {
        let req = semantica_task_engine::application::dev_task::EnqueueRequest {
            job_type: "WORKER_TEST".to_string(),
            queue: "test_queue".to_string(),
            subject_key: format!("subject::{}", i),
            payload: serde_json::json!({}),
            priority: 0,
        };
        service.enqueue(req).await.unwrap();
    }

    // Spawn 3 concurrent workers
    let mut tasks = JoinSet::new();
    for _ in 0..3 {
        let repo = repo_job.clone();
        tasks.spawn(async move {
            let worker = Worker::new("test_queue", repo);
            let mut processed_count = 0;
            
            // Try to process jobs
            for _ in 0..5 {
                if worker.process_next_job().await.unwrap() {
                    processed_count += 1;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            
            processed_count
        });
    }

    // Wait for all workers
    let mut total_processed = 0;
    while let Some(result) = tasks.join_next().await {
        total_processed += result.unwrap();
    }

    // Exactly 5 jobs should have been processed (no duplicates)
    assert_eq!(total_processed, 5, "Expected exactly 5 jobs processed (no duplicates)");

    // All jobs should be DONE
    let done_count = repo_job.count_by_state("test_queue", JobState::Done).await.unwrap();
    assert_eq!(done_count, 5);
}

#[tokio::test]
async fn test_worker_shutdown() {
    // Test graceful shutdown
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let sqlite_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let repo_job: Arc<dyn JobRepository> = sqlite_repo;

    let (shutdown_tx, shutdown_rx) = shutdown_channel();
    let worker = Worker::new("test_queue", repo_job.clone());

    // Spawn worker in background
    let worker_handle = tokio::spawn(async move {
        worker.run(shutdown_rx).await
    });

    // Let worker run for a bit
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Signal shutdown
    shutdown_tx.shutdown();

    // Worker should terminate gracefully
    let result = tokio::time::timeout(Duration::from_secs(2), worker_handle).await;
    assert!(result.is_ok(), "Worker should shutdown within 2 seconds");
    assert!(result.unwrap().unwrap().is_ok(), "Worker should shutdown without error");
}

#[tokio::test]
async fn test_multiple_workers_with_shutdown() {
    // Test multiple workers with coordinated shutdown
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    let time_provider = Arc::new(SystemTimeProvider);
    let sqlite_repo = Arc::new(SqliteJobRepository::new(pool, time_provider));
    let repo_tx: Arc<dyn TransactionalJobRepository> = sqlite_repo.clone();
    let repo_job: Arc<dyn JobRepository> = sqlite_repo;

    let service = DevTaskService::new(repo_tx);

    // Enqueue 20 jobs
    for i in 0..20 {
        let req = semantica_task_engine::application::dev_task::EnqueueRequest {
            job_type: "MULTI_WORKER".to_string(),
            queue: "test_queue".to_string(),
            subject_key: format!("subject::{}", i),
            payload: serde_json::json!({}),
            priority: 0,
        };
        service.enqueue(req).await.unwrap();
    }

    let (shutdown_tx, shutdown_rx) = shutdown_channel();

    // Spawn 5 workers
    let mut worker_handles = vec![];
    for _ in 0..5 {
        let repo = repo_job.clone();
        let token = shutdown_rx.clone();
        let handle = tokio::spawn(async move {
            let worker = Worker::new("test_queue", repo);
            worker.run(token).await
        });
        worker_handles.push(handle);
    }

    // Let workers process some jobs
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Signal shutdown
    shutdown_tx.shutdown();

    // All workers should terminate gracefully
    for handle in worker_handles {
        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "All workers should shutdown within 2 seconds");
        assert!(result.unwrap().unwrap().is_ok());
    }

    // Check how many jobs were processed (at least some, possibly all)
    let done_count = repo_job.count_by_state("test_queue", JobState::Done).await.unwrap();
    assert!(done_count > 0, "At least some jobs should have been processed");
}

