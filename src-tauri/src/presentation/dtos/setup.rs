use serde::{Deserialize, Serialize};
use std::fmt;

use crate::domain::{
    entities::{NotificationPreferences, TimeRange, WorkDay},
    NotificationPriority,
};

#[derive(Debug, Serialize)]
pub struct SetupError {
    pub code: String,
    pub message: String,
}

impl SetupError {
    pub fn new(message: &str) -> Self {
        Self {
            code: "SETUP_ERROR".to_string(),
            message: message.to_string(),
        }
    }
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SetupError {}

#[derive(Debug, Deserialize)]
pub struct NotificationPreferencesRequest {
    pub desktop_notifications: bool,
    pub sound_enabled: bool,
    pub notification_priority: String,
    pub auto_archive_delay: i32,
    pub working_hours: WorkingHours,
    pub quiet_hours: QuietHours,
}

#[derive(Debug, Deserialize)]
pub struct WorkingHours {
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Deserialize)]
pub struct QuietHours {
    pub enabled: bool,
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AIConfigRequest {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: i32,
    pub response_style: String,
    pub auto_response_enabled: bool,
    pub custom_prompt: Option<String>,
    pub use_local_model: bool,
    pub local_model_path: Option<String>,
    pub fallback_to_cloud: bool,
}

impl From<NotificationPreferencesRequest> for NotificationPreferences {
    fn from(req: NotificationPreferencesRequest) -> Self {
        let priority = match req.notification_priority.as_str() {
            "all" => NotificationPriority::Low,
            "important" => NotificationPriority::Medium,
            "urgent" => NotificationPriority::High,
            _ => NotificationPriority::Medium,
        };

        NotificationPreferences {
            desktop_notifications: req.desktop_notifications,
            sound_enabled: req.sound_enabled,
            min_priority: priority,
            auto_archive_delay: chrono::Duration::hours(i64::from(req.auto_archive_delay)),
            work_day: WorkDay {
                active_hours: TimeRange {
                    start: req.working_hours.start,
                    end: req.working_hours.end,
                },
            },
            quiet_hours: if req.quiet_hours.enabled {
                Some(TimeRange {
                    start: req.quiet_hours.start,
                    end: req.quiet_hours.end,
                })
            } else {
                None
            },
        }
    }
}
