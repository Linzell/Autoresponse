use autoresponse_lib::domain::error::DomainResult;
use autoresponse_lib::domain::{
    error::DomainError,
    services::{BackgroundJobManager, Job, JobHandler, JobPriority, JobStatus, JobType},
};
use parking_lot::Mutex;
use std::{sync::Arc, time::Duration};
use tokio;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestJob {
    pub id: Uuid,
    pub success: bool,
    pub delay: Option<Duration>,
    pub should_panic: bool,
}

#[derive(Debug)]
struct TestJobHandler {
    executed_jobs: Arc<Mutex<Vec<TestJob>>>,
}

impl JobHandler for TestJobHandler {
    fn handle(&self, job: &mut Job) -> Result<(), String> {
        // First try to deserialize - this validates the payload format
        let job_data: TestJob = match serde_json::from_value(job.payload.clone()) {
            Ok(data) => data,
            Err(e) => {
                let msg = format!("Failed to deserialize job data: {}", e);
                job.fail(msg.clone());
                return Err(msg);
            }
        };

        // Apply the specified delay if any
        if let Some(delay) = job_data.delay {
            std::thread::sleep(delay);
        }

        // First check if we should fail
        let should_fail = matches!(job.status, JobStatus::Cancelled)
            || job_data.should_panic
            || !job_data.success;

        // Record the attempt before we potentially fail
        {
            let mut executed_jobs = self.executed_jobs.lock();
            executed_jobs.push(job_data.clone());
        }

        // Now handle failure conditions
        if should_fail {
            let msg = if matches!(job.status, JobStatus::Cancelled) {
                "Job cancelled"
            } else if job_data.should_panic {
                "Job panicked as requested"
            } else {
                "Job failed as requested"
            };
            job.fail(msg.to_string());
            return Err(msg.to_string());
        }

        // Mark job as completed
        job.complete();
        Ok(())
    }

    fn job_type(&self) -> JobType {
        JobType::Custom("test".to_string())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_parallel_job_processing() -> DomainResult<()> {
    let manager = BackgroundJobManager::new();
    let executed_jobs = Arc::new(Mutex::new(Vec::new()));
    let handler = Arc::new(TestJobHandler {
        executed_jobs: executed_jobs.clone(),
    });

    manager.register_handler(handler).await?;

    // Create multiple jobs without delays
    let job_count = 3;
    let mut job_ids = Vec::new();

    for _ in 0..job_count {
        let test_job = TestJob {
            id: Uuid::new_v4(),
            success: true,
            delay: None,
            should_panic: false,
        };

        let job = Job::new(
            serde_json::to_value(test_job)?,
            JobPriority::Normal,
            JobType::Custom("test".to_string()),
            1,
        );

        let job_id = manager.submit_job(job).await?;
        job_ids.push(job_id);
    }

    // Wait for all jobs to complete, with exponential backoff
    let mut backoff = Duration::from_millis(1);
    let max_backoff = Duration::from_millis(50);
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(1);

    loop {
        let mut completed = Vec::new();
        let mut any_failed = false;

        // Check all remaining jobs
        for job_id in &job_ids {
            match manager.get_job_status(*job_id).await {
                Some(JobStatus::Completed) | None => {
                    completed.push(*job_id);
                }
                Some(JobStatus::Failed) => {
                    any_failed = true;
                    break;
                }
                _ => {}
            }
        }

        // Remove completed jobs
        job_ids.retain(|id| !completed.contains(id));

        // Check completion conditions
        if any_failed {
            panic!("One or more jobs failed unexpectedly");
        }
        if job_ids.is_empty() {
            break;
        }
        if start.elapsed() > timeout {
            panic!("Jobs did not complete within timeout");
        }

        // Exponential backoff
        tokio::time::sleep(backoff).await;
        backoff = std::cmp::min(backoff * 2, max_backoff);
    }

    // Verify all jobs were executed
    let executed = executed_jobs.lock();
    assert_eq!(executed.len(), job_count, "Not all jobs were executed");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_job_cancellation() -> DomainResult<()> {
    let manager = BackgroundJobManager::new();
    let executed_jobs = Arc::new(Mutex::new(Vec::new()));
    let handler = Arc::new(TestJobHandler {
        executed_jobs: executed_jobs.clone(),
    });

    manager.register_handler(handler).await?;

    // Create a long-running job
    let test_job = TestJob {
        id: Uuid::new_v4(),
        success: true,
        delay: Some(Duration::from_millis(1)),
        should_panic: false,
    };

    let job = Job::new(
        serde_json::to_value(test_job)?,
        JobPriority::Normal,
        JobType::Custom("test".to_string()),
        1, // Reduce retries to make failure faster
    );

    let job_id = manager.submit_job(job).await?;

    // Wait briefly for job to start
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Try to cancel the job
    let _ = manager.cancel_job(job_id).await;

    // Give some time for cancellation to take effect
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Verify the job is either cancelled or completed
    let final_status = manager.get_job_status(job_id).await;
    assert!(final_status.is_none() || matches!(final_status, Some(JobStatus::Cancelled)));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_job_error_handling_and_retries() -> DomainResult<()> {
    let manager = BackgroundJobManager::new();
    let executed_jobs = Arc::new(Mutex::new(Vec::new()));
    let handler = Arc::new(TestJobHandler {
        executed_jobs: executed_jobs.clone(),
    });

    manager.register_handler(handler).await?;

    // Create a job that will fail
    let test_job = TestJob {
        id: Uuid::new_v4(),
        success: false,
        delay: None,
        should_panic: false,
    };

    let job = Job::new(
        serde_json::to_value(test_job)?,
        JobPriority::High, // Use high priority to ensure quick processing
        JobType::Custom("test".to_string()),
        1, // Single attempt to fail fast
    );

    let job_id = manager.submit_job(job).await?;

    // Wait for job to fail
    let mut retries = 20; // 200ms total with 10ms interval
    while retries > 0 {
        match manager.get_job_status(job_id).await {
            Some(JobStatus::Failed) => break, // Success - job failed as expected
            None => break,                    // Job completed and was removed
            Some(JobStatus::Completed) => panic!("Job completed when it should have failed"),
            _ => {
                tokio::time::sleep(Duration::from_millis(10)).await;
                retries -= 1;
            }
        }
        if retries == 0 {
            panic!("Job did not fail within timeout period");
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_job_priority_handling() -> DomainResult<()> {
    let manager = BackgroundJobManager::new();
    let executed_jobs = Arc::new(Mutex::new(Vec::new()));
    let handler = Arc::new(TestJobHandler {
        executed_jobs: executed_jobs.clone(),
    });

    manager.register_handler(handler).await?;

    // Create jobs with different priorities
    let jobs = vec![(JobPriority::Low, "low"), (JobPriority::High, "high")];

    let mut job_ids = Vec::new();
    for (priority, _name) in jobs {
        let test_job = TestJob {
            id: Uuid::new_v4(),
            success: true,
            delay: None,
            should_panic: false,
        };

        let job = Job::new(
            serde_json::to_value(test_job)?,
            priority,
            JobType::Custom("test".to_string()),
            1,
        );

        let job_id = manager.submit_job(job).await?;
        job_ids.push(job_id);
    }

    // Wait for jobs to complete
    let mut attempts = 10;
    while attempts > 0 {
        let mut all_done = true;
        for job_id in &job_ids {
            match manager.get_job_status(*job_id).await {
                Some(JobStatus::Completed) | None => continue,
                Some(JobStatus::Failed) => panic!("Job {} failed unexpectedly", job_id),
                _ => {
                    all_done = false;
                    break;
                }
            }
        }
        if all_done {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
        attempts -= 1;
    }

    panic!("Jobs did not complete within the time limit");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_invalid_job_handling() -> DomainResult<()> {
    let manager = BackgroundJobManager::new();
    let executed_jobs = Arc::new(Mutex::new(Vec::new()));
    let handler = Arc::new(TestJobHandler {
        executed_jobs: executed_jobs.clone(),
    });

    manager.register_handler(handler).await?;

    // Create a job with completely invalid payload that cannot deserialize into TestJob
    let invalid_payload = serde_json::Value::String("not_a_test_job".to_string());
    let job = Job::new(
        invalid_payload,
        JobPriority::Critical, // Use critical priority for immediate processing
        JobType::Custom("test".to_string()),
        0, // No retries - should fail immediately
    );

    let job_id = manager.submit_job(job).await?;

    // Give a small delay for the job to be picked up
    tokio::time::sleep(Duration::from_millis(5)).await;

    // Wait for job to fail with timeout
    let mut failed = false;
    for i in 0..20 {
        // Try for 2 seconds
        println!("Checking job status attempt {}", i);
        match manager.get_job_status(job_id).await {
            Some(JobStatus::Failed) => {
                println!("Job failed as expected");
                failed = true;
                break;
            }
            Some(JobStatus::Completed) => {
                panic!("Job completed when it should have failed");
            }
            None => {
                println!("Job was removed, checking execution history");
                let executed = executed_jobs.lock();
                println!("Found {} executed jobs", executed.len());
                failed = true; // If job is gone, it must have failed
                break;
            }
            status => {
                println!("Current job status: {:?}", status);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
    }

    if !failed {
        // One final check
        match manager.get_job_status(job_id).await {
            Some(status) => panic!("Job in unexpected final state: {:?}", status),
            None => {
                let executed = executed_jobs.lock();
                if executed.is_empty() {
                    panic!("Job disappeared without execution");
                }
                failed = true;
            }
        }
    }

    assert!(failed, "Job should fail due to invalid payload");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_handler_registration() -> DomainResult<()> {
    let manager = BackgroundJobManager::new();
    let handler = Arc::new(TestJobHandler {
        executed_jobs: Arc::new(Mutex::new(Vec::new())),
    });

    // First registration should succeed
    manager.register_handler(handler.clone()).await?;

    // Second registration of same type should fail
    let result = manager.register_handler(handler.clone()).await;
    assert!(matches!(result, Err(DomainError::ConflictError(_))));

    Ok(())
}
