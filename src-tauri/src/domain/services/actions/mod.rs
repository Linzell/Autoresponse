use crate::domain::{entities::Notification, error::DomainResult, NotificationSource};

pub mod executor;
pub use executor::ActionExecutor;

pub trait ActionHandler: Send + Sync {
    fn handle<'a>(&'a self, notification: &'a Notification) -> std::pin::Pin<Box<dyn std::future::Future<Output = DomainResult<()>> + Send + 'a>>;
}

pub enum ActionType {
    Email,
    Github,
    Gitlab,
    Jira,
    Microsoft,
    Google,
    LinkedIn,
    Custom(String),
}

impl From<NotificationSource> for ActionType {
    fn from(source: NotificationSource) -> Self {
        match source {
            NotificationSource::Email => Self::Email,
            NotificationSource::Github => Self::Github,
            NotificationSource::Gitlab => Self::Gitlab,
            NotificationSource::Jira => Self::Jira,
            NotificationSource::Microsoft => Self::Microsoft,
            NotificationSource::Google => Self::Google,
            NotificationSource::LinkedIn => Self::LinkedIn,
            NotificationSource::Custom(service) => Self::Custom(service),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type_from_source() {
        assert!(matches!(
            ActionType::from(NotificationSource::Email),
            ActionType::Email
        ));
        assert!(matches!(
            ActionType::from(NotificationSource::Github),
            ActionType::Github
        ));
        assert!(matches!(
            ActionType::from(NotificationSource::Custom("test".to_string())),
            ActionType::Custom(s) if s == "test"
        ));
    }
}
