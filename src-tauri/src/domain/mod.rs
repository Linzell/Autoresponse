pub mod entities;
pub mod error;
pub mod repositories;
pub mod services;

pub use entities::{
    NotificationPriority,
    NotificationStatus,
    NotificationSource,
    NotificationMetadata,
    Notification,
    ServiceType,
    AuthType,
    AuthConfig,
    ServiceEndpoints,
    ServiceConfig,
    OAuth2Config,
    BasicAuthConfig,
    ApiKeyConfig,
    CustomAuthConfig,
};

pub use error::{DomainError, DomainResult};

pub use repositories::{
    NotificationRepository,
    ServiceConfigRepository,
    DynNotificationRepository,
    DynServiceConfigRepository,
};

pub use services::{
    NotificationService,
    ServiceConfigService,
    DynNotificationService,
    DynServiceConfigService,
};