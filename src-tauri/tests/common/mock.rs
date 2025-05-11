use autoresponse_lib::domain::{
    entities::{
        AuthConfig, AuthType, Notification, NotificationMetadata, NotificationPriority,
        NotificationSource, NotificationStatus, ServiceConfig, ServiceEndpoints, ServiceType,
    },
    error::DomainResult,
    services::{NotificationService, ServiceConfigService},
};
use uuid::Uuid;

mockall::mock! {
    #[derive(Clone)]
    pub NotificationService {}
    #[async_trait::async_trait]
    impl NotificationService for NotificationService {
        async fn create_notification(
            &self,
            title: String,
            content: String,
            priority: NotificationPriority,
            metadata: NotificationMetadata,
        ) -> DomainResult<Notification>;
        async fn get_notification(&self, id: Uuid) -> DomainResult<Notification>;
        async fn get_all_notifications(&self) -> DomainResult<Vec<Notification>>;
        async fn get_notifications_by_status(&self, status: NotificationStatus) -> DomainResult<Vec<Notification>>;
        async fn get_notifications_by_source(&self, source: NotificationSource) -> DomainResult<Vec<Notification>>;
        async fn mark_as_read(&self, id: Uuid) -> DomainResult<()>;
        async fn mark_action_required(&self, id: Uuid) -> DomainResult<()>;
        async fn mark_action_taken(&self, id: Uuid) -> DomainResult<()>;
        async fn archive_notification(&self, id: Uuid) -> DomainResult<()>;
        async fn delete_notification(&self, id: Uuid) -> DomainResult<()>;
    }
}

mockall::mock! {
    #[derive(Clone)]
    pub ServiceConfig {}
    #[async_trait::async_trait]
    impl ServiceConfigService for ServiceConfig {
        async fn create_service_config(
            &self,
            name: String,
            service_type: ServiceType,
            auth_type: AuthType,
            auth_config: AuthConfig,
            endpoints: ServiceEndpoints,
        ) -> DomainResult<ServiceConfig>;
        async fn get_service_config(&self, id: Uuid) -> DomainResult<ServiceConfig>;
        async fn get_all_service_configs(&self) -> DomainResult<Vec<ServiceConfig>>;
        async fn get_enabled_configs(&self) -> DomainResult<Vec<ServiceConfig>>;
        async fn get_configs_by_service_type(&self, service_type: ServiceType) -> DomainResult<Vec<ServiceConfig>>;
        async fn update_auth_config(&self, id: Uuid, auth_config: AuthConfig) -> DomainResult<()>;
        async fn enable_service(&self, id: Uuid) -> DomainResult<()>;
        async fn disable_service(&self, id: Uuid) -> DomainResult<()>;
        async fn delete_service_config(&self, id: Uuid) -> DomainResult<()>;
        async fn update_last_sync(&self, id: Uuid) -> DomainResult<()>;
    }
}
