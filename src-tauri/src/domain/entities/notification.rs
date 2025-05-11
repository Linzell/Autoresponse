use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationStatus {
    New,
    Read,
    Archived,
    ActionRequired,
    ActionTaken,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationSource {
    Email,
    Github,
    Gitlab,
    Jira,
    Microsoft,
    Google,
    LinkedIn,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMetadata {
    pub source: NotificationSource,
    pub external_id: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub custom_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub priority: NotificationPriority,
    pub status: NotificationStatus,
    pub metadata: NotificationMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub action_taken_at: Option<DateTime<Utc>>,
}

impl Notification {
    pub fn new(
        title: String,
        content: String,
        priority: NotificationPriority,
        metadata: NotificationMetadata,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            content,
            priority,
            status: NotificationStatus::New,
            metadata,
            created_at: now,
            updated_at: now,
            read_at: None,
            action_taken_at: None,
        }
    }

    pub fn mark_as_read(&mut self) {
        self.status = NotificationStatus::Read;
        self.read_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn archive(&mut self) {
        self.status = NotificationStatus::Archived;
        self.updated_at = Utc::now();
    }

    pub fn mark_action_required(&mut self) {
        self.status = NotificationStatus::ActionRequired;
        self.updated_at = Utc::now();
    }

    pub fn mark_action_taken(&mut self) {
        self.status = NotificationStatus::ActionTaken;
        self.action_taken_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn delete(&mut self) {
        self.status = NotificationStatus::Deleted;
        self.updated_at = Utc::now();
    }
}