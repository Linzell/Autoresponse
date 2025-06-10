use chrono::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkDay {
    pub active_hours: TimeRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub desktop_notifications: bool,
    pub sound_enabled: bool,
    pub min_priority: super::notification::NotificationPriority,
    pub auto_archive_delay: Duration,
    pub work_day: WorkDay,
    pub quiet_hours: Option<TimeRange>,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            desktop_notifications: true,
            sound_enabled: true,
            min_priority: super::notification::NotificationPriority::Low,
            auto_archive_delay: Duration::hours(24),
            work_day: WorkDay {
                active_hours: TimeRange {
                    start: 9.0,
                    end: 17.0,
                },
            },
            quiet_hours: Some(TimeRange {
                start: 22.0,
                end: 7.0,
            }),
        }
    }
}

impl NotificationPreferences {
    pub fn is_within_work_hours(&self, hour: f32) -> bool {
        hour >= self.work_day.active_hours.start && hour <= self.work_day.active_hours.end
    }

    pub fn is_within_quiet_hours(&self, hour: f32) -> bool {
        if let Some(quiet_hours) = &self.quiet_hours {
            if quiet_hours.start > quiet_hours.end {
                // Handle overnight range (e.g., 22:00 - 07:00)
                hour >= quiet_hours.start || hour <= quiet_hours.end
            } else {
                hour >= quiet_hours.start && hour <= quiet_hours.end
            }
        } else {
            false
        }
    }

    pub fn should_notify(&self, priority: &super::notification::NotificationPriority) -> bool {
        use super::notification::NotificationPriority;
        match (priority, &self.min_priority) {
            (NotificationPriority::Critical, _) => true,
            (NotificationPriority::High, NotificationPriority::High) => true,
            (NotificationPriority::High, NotificationPriority::Medium) => true,
            (NotificationPriority::High, NotificationPriority::Low) => true,
            (NotificationPriority::Medium, NotificationPriority::Medium) => true,
            (NotificationPriority::Medium, NotificationPriority::Low) => true,
            (NotificationPriority::Low, NotificationPriority::Low) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::notification::NotificationPriority;
    use super::*;

    #[test]
    fn test_work_hours() {
        let prefs = NotificationPreferences::default();
        assert!(!prefs.is_within_work_hours(8.5));
        assert!(prefs.is_within_work_hours(9.0));
        assert!(prefs.is_within_work_hours(13.0));
        assert!(prefs.is_within_work_hours(17.0));
        assert!(!prefs.is_within_work_hours(17.5));
    }

    #[test]
    fn test_quiet_hours() {
        let prefs = NotificationPreferences::default();
        assert!(prefs.is_within_quiet_hours(23.0)); // 11 PM
        assert!(prefs.is_within_quiet_hours(2.0)); // 2 AM
        assert!(prefs.is_within_quiet_hours(6.0)); // 6 AM
        assert!(!prefs.is_within_quiet_hours(8.0)); // 8 AM
        assert!(!prefs.is_within_quiet_hours(21.0)); // 9 PM
    }

    #[test]
    fn test_notification_priority() {
        let mut prefs = NotificationPreferences::default();

        // Test with Low minimum priority
        prefs.min_priority = NotificationPriority::Low;
        assert!(prefs.should_notify(&NotificationPriority::Critical));
        assert!(prefs.should_notify(&NotificationPriority::High));
        assert!(prefs.should_notify(&NotificationPriority::Medium));
        assert!(prefs.should_notify(&NotificationPriority::Low));

        // Test with Medium minimum priority
        prefs.min_priority = NotificationPriority::Medium;
        assert!(prefs.should_notify(&NotificationPriority::Critical));
        assert!(prefs.should_notify(&NotificationPriority::High));
        assert!(prefs.should_notify(&NotificationPriority::Medium));
        assert!(!prefs.should_notify(&NotificationPriority::Low));

        // Test with High minimum priority
        prefs.min_priority = NotificationPriority::High;
        assert!(prefs.should_notify(&NotificationPriority::Critical));
        assert!(prefs.should_notify(&NotificationPriority::High));
        assert!(!prefs.should_notify(&NotificationPriority::Medium));
        assert!(!prefs.should_notify(&NotificationPriority::Low));
    }
}
