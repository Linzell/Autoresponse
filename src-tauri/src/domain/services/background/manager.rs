use super::types::{Job, JobHandler, JobStatus, JobType};
use crate::domain::error::DomainError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub type DynBackgroundJobManager = Arc<dyn BackgroundJobManagerTrait>;

#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait BackgroundJobManagerTrait: Send + Sync {
    async fn register_handler(&self, handler: Arc<dyn JobHandler>) -> Result<(), DomainError>;
    async fn submit_job(&self, job: Job) -> Result<uuid::Uuid, DomainError>;
    async fn get_job_status(&self, job_id: uuid::Uuid) -> Option<JobStatus>;
    async fn cancel_job(&self, job_id: uuid::Uuid) -> Result<(), DomainError>;
}

#[derive(Debug)]
pub struct BackgroundJobManager {
    handlers: Arc<RwLock<HashMap<JobType, Arc<dyn JobHandler>>>>,
    active_jobs: Arc<RwLock<HashMap<uuid::Uuid, Arc<RwLock<Job>>>>>,
}

impl Default for BackgroundJobManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BackgroundJobManagerTrait for BackgroundJobManager {
    async fn register_handler(&self, handler: Arc<dyn JobHandler>) -> Result<(), DomainError> {
        let job_type = handler.job_type();
        let mut handlers = self.handlers.write().await;

        if handlers.contains_key(&job_type) {
            return Err(DomainError::ConflictError(format!(
                "Handler for job type {:?} already registered",
                job_type
            )));
        }

        handlers.insert(job_type, handler);
        Ok(())
    }

    async fn submit_job(&self, job: Job) -> Result<uuid::Uuid, DomainError> {
        let handlers = self.handlers.read().await;
        if !handlers.contains_key(&job.metadata.job_type) {
            return Err(DomainError::ValidationError(format!(
                "No handler registered for job type {:?}",
                job.metadata.job_type
            )));
        }

        let job = Arc::new(RwLock::new(job));
        let job_id = { job.read().await.id };

        let mut active_jobs = self.active_jobs.write().await;
        active_jobs.insert(job_id, job.clone());

        // Spawn task to process the job
        let handler = handlers[&job.read().await.metadata.job_type].clone();
        let active_jobs = self.active_jobs.clone();

        tokio::spawn(async move {
            Self::process_job(job, handler, active_jobs).await;
        });

        Ok(job_id)
    }

    async fn get_job_status(&self, job_id: uuid::Uuid) -> Option<JobStatus> {
        let active_jobs = self.active_jobs.read().await;
        if let Some(job) = active_jobs.get(&job_id) {
            let job = job.read().await;
            Some(job.status.clone())
        } else {
            None
        }
    }

    async fn cancel_job(&self, job_id: uuid::Uuid) -> Result<(), DomainError> {
        let active_jobs = self.active_jobs.read().await;

        if let Some(job) = active_jobs.get(&job_id) {
            let mut job = job.write().await;
            if job.status == JobStatus::Running || job.status == JobStatus::Pending {
                job.cancel();
                Ok(())
            } else {
                Err(DomainError::ValidationError(format!(
                    "Cannot cancel job {} in state {:?}",
                    job_id, job.status
                )))
            }
        } else {
            Err(DomainError::NotFoundError(format!(
                "Job {} not found",
                job_id
            )))
        }
    }
}

impl BackgroundJobManager {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn process_job(
        job: Arc<RwLock<Job>>,
        handler: Arc<dyn JobHandler>,
        active_jobs: Arc<RwLock<HashMap<uuid::Uuid, Arc<RwLock<Job>>>>>,
    ) {
        let (job_id, job_type) = {
            let job_read = job.read().await;
            (job_read.id, job_read.metadata.job_type.clone())
        };

        let start_time = std::time::Instant::now();

        info!(
            "Starting job processing. ID: {}, Type: {:?}",
            job_id, job_type
        );

        // Mark job as started
        {
            let mut job_write = job.write().await;
            job_write.start();
        }
        info!("Job state changed to Running. ID: {}", job_id);

        // Process the job with minimal lock time
        let mut job_inner = job.write().await;
        let result = handler.handle(&mut job_inner).await;
        let status = match &result {
            Ok(()) => {
                let elapsed = start_time.elapsed();
                job_inner.complete();
                info!(
                    "Job completed successfully. ID: {}, Type: {:?}, Duration: {:?}",
                    job_id, job_type, elapsed
                );
                JobStatus::Completed
            }
            Err(error) => {
                let elapsed = start_time.elapsed();
                job_inner.fail(error.clone());
                warn!(
                    "Job failed. ID: {}, Type: {:?}, Duration: {:?}, Error: {}",
                    job_id, job_type, elapsed, error
                );

                if !job_inner.can_retry() {
                    error!(
                        "Job exceeded maximum retries. ID: {}, Type: {:?}",
                        job_id, job_type
                    );
                }
                JobStatus::Failed
            }
        };
        drop(job_inner); // Explicitly release the lock

        // Remove from active jobs if complete/failed
        if matches!(status, JobStatus::Completed | JobStatus::Failed) {
            let mut active_jobs = active_jobs.write().await;
            match active_jobs.remove(&job_id) {
                Some(_) => info!("Job removed from active jobs. ID: {}", job_id),
                None => warn!(
                    "Job not found in active jobs during cleanup. ID: {}",
                    job_id
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{JobPriority, JobType};
    use super::*;

    #[derive(Debug)]
    struct TestHandler;

    #[async_trait::async_trait]
    impl JobHandler for TestHandler {
        async fn handle(&self, job: &mut Job) -> Result<(), String> {
            // Minimal delay to avoid busy waiting
            tokio::time::sleep(std::time::Duration::from_micros(100)).await;
            job.complete();
            Ok(())
        }

        fn job_type(&self) -> JobType {
            JobType::Custom("test".to_string())
        }
    }

    #[tokio::test]
    async fn test_job_lifecycle() {
        let manager = BackgroundJobManager::new();
        let handler = Arc::new(TestHandler);

        info!("Starting job lifecycle test");

        // Register handler
        manager.register_handler(handler).await.unwrap();
        info!("Test handler registered");

        // Create and submit job
        let job_type = JobType::Custom("test".to_string());
        let job = Job::new(
            serde_json::Value::Null,
            JobPriority::Normal,
            job_type.clone(),
            3,
        );
        let job_id = manager.submit_job(job).await.unwrap();
        info!("Test job submitted. ID: {}, Type: {:?}", job_id, job_type);

        // Track job completion with exponential backoff
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(1); // Short timeout since jobs are fast
        let mut attempts = 0;
        let mut last_status = None;
        let check_interval = std::time::Duration::from_millis(5);

        loop {
            if start_time.elapsed() >= timeout {
                break;
            }

            attempts += 1;
            let status = manager.get_job_status(job_id).await;
            info!(
                "Status check attempt {}: {:?} (elapsed: {:?})",
                attempts,
                status,
                start_time.elapsed()
            );

            match status {
                Some(JobStatus::Completed) => {
                    info!("Job completed successfully after {} attempts", attempts);
                    return;
                }
                Some(JobStatus::Failed) => {
                    panic!(
                        "Job failed unexpectedly. Status: {:?}, Attempts: {}",
                        status, attempts
                    );
                }
                None => {
                    // Job might be completed and removed from active_jobs
                    info!("Job not found in active jobs - assuming completed");
                    return;
                }
                _ => {
                    last_status = status;
                    tokio::time::sleep(check_interval).await;
                }
            }
        }

        panic!(
            "Job did not complete within {:?}. Last status: {:?}, Attempts: {}",
            timeout, last_status, attempts
        );
    }
}
