pub mod entities;
pub mod error;
pub mod events;
pub mod repositories;
pub mod services;

pub use entities::{
    ApiKeyConfig, AuthConfig, AuthType, BasicAuthConfig, CustomAuthConfig, Notification,
    NotificationMetadata, NotificationPriority, NotificationSource, NotificationStatus,
    OAuth2Config, ServiceConfig, ServiceEndpoints, ServiceType,
};

pub use error::{DomainError, DomainResult};

pub use events::{DynEventPublisher, EventPublisher, NotificationEvent};

pub use repositories::{
    DynNotificationRepository, DynServiceConfigRepository, NotificationRepository,
    ServiceConfigRepository,
};

pub use services::{
    DynNotificationService, DynServiceConfigService, NotificationService, ServiceConfigService,
};
