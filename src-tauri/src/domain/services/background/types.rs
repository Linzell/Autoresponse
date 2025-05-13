use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum JobType {
    NotificationProcessing,
    ResponseGeneration,
    ActionExecution,
    ServiceSync,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetadata {
    pub job_type: JobType,
    pub retry_count: u32,
    pub max_retries: u32,
    pub last_error: Option<String>,
    pub custom_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub payload: serde_json::Value,
    pub priority: JobPriority,
    pub status: JobStatus,
    pub metadata: JobMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Job {
    pub fn new(
        payload: serde_json::Value,
        priority: JobPriority,
        job_type: JobType,
        max_retries: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            payload,
            priority,
            status: JobStatus::Pending,
            metadata: JobMetadata {
                job_type,
                retry_count: 0,
                max_retries,
                last_error: None,
                custom_data: None,
            },
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }

    pub fn start(&mut self) {
        let now = Utc::now();
        self.status = JobStatus::Running;
        self.started_at = Some(now);
        self.updated_at = now;
    }

    pub fn complete(&mut self) {
        let now = Utc::now();
        self.status = JobStatus::Completed;
        self.completed_at = Some(now);
        self.updated_at = now;
    }

    pub fn fail(&mut self, error: String) {
        let now = Utc::now();
        self.metadata.retry_count += 1;
        self.metadata.last_error = Some(error);

        if self.metadata.retry_count >= self.metadata.max_retries {
            self.status = JobStatus::Failed;
        } else {
            self.status = JobStatus::Pending;
        }

        self.updated_at = now;
    }

    pub fn cancel(&mut self) {
        let now = Utc::now();
        self.status = JobStatus::Cancelled;
        self.updated_at = now;
    }

    pub fn can_retry(&self) -> bool {
        self.metadata.retry_count < self.metadata.max_retries
    }
}

pub trait JobHandler: Send + Sync + Debug {
    fn handle(&self, job: &mut Job) -> Result<(), String>;
    fn job_type(&self) -> JobType;
}
