pub mod actions;
pub mod ai;
pub mod background;
pub mod notification_service;
pub mod service_config_service;

pub use actions::executor::{ActionExecutor, ActionExecutorTrait, DynActionExecutor};
pub use ai::{AIAnalysis, AIConfig, DynAIService, OllamaService, PriorityLevel};
pub use background::{
    manager::{BackgroundJobManager, BackgroundJobManagerTrait, DynBackgroundJobManager},
    types::{Job, JobHandler, JobPriority, JobStatus, JobType},
    NotificationActionType, NotificationProcessor,
};

pub use notification_service::{
    DefaultNotificationService, DynNotificationService, NotificationService,
};

pub use service_config_service::{
    DefaultServiceConfigService, DynServiceConfigService, ServiceConfigService,
};

#[cfg(test)]
pub use notification_service::MockNotificationService;

#[cfg(test)]
pub use service_config_service::MockServiceConfigService;

#[cfg(test)]
pub use ai::MockAIService;
