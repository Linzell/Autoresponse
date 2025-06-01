use crate::infrastructure::repositories::cached_repository::CachedEntity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
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

impl fmt::Display for NotificationSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NotificationSource::Email => write!(f, "Email"),
            NotificationSource::Github => write!(f, "Github"),
            NotificationSource::Gitlab => write!(f, "Gitlab"),
            NotificationSource::Jira => write!(f, "Jira"),
            NotificationSource::Microsoft => write!(f, "Microsoft"),
            NotificationSource::Google => write!(f, "Google"),
            NotificationSource::LinkedIn => write!(f, "LinkedIn"),
            NotificationSource::Custom(s) => write!(f, "{}", s),
        }
    }
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

impl CachedEntity for Notification {
    fn get_id(&self) -> Uuid {
        self.id
    }
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
        let now = Utc::now();
        self.status = NotificationStatus::Read;
        self.read_at = Some(now);
        self.updated_at = now;
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
