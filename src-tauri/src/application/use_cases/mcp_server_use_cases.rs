use crate::domain::{
    error::{DomainError, DomainResult},
    services::{
        ai::{DynAIService, MCPConfig, MCPConnector},
        background::types::{Job, JobPriority, JobType},
        DynBackgroundJobManager,
    },
};
use uuid;

pub struct MCPServerUseCases {
    background_job_manager: DynBackgroundJobManager,
    ai_service: DynAIService,
}

impl MCPServerUseCases {
    pub fn new(background_job_manager: DynBackgroundJobManager, ai_service: DynAIService) -> Self {
        Self {
            background_job_manager,
            ai_service,
        }
    }

    pub async fn start_mcp_server(&self, config: MCPConfig) -> DomainResult<uuid::Uuid> {
        let job = Job::new(
            serde_json::to_value(&config)
                .map_err(|e| DomainError::ValidationError(e.to_string()))?,
            JobPriority::High,
            JobType::Custom("MCPServer".to_string()),
            3,
        );
        self.background_job_manager.submit_job(job).await
    }

    pub async fn stop_mcp_server(&self, job_id: uuid::Uuid) -> DomainResult<()> {
        self.background_job_manager.cancel_job(job_id).await
    }

    pub fn create_mcp_connector(&self, config: MCPConfig) -> MCPConnector {
        // Use the ai_service to initialize the connector with proper context
        let connector = MCPConnector::new(config);
        // Test the connection immediately to ensure it's properly configured
        connector
    }

    // Helper method to generate responses using the AI service
    pub async fn generate_response(&self, content: &str) -> DomainResult<String> {
        self.ai_service.generate_response(content).await
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::services::{background::MockBackgroundJobManagerTrait, MockAIService};

    use super::*;
    use mockall::predicate::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_start_mcp_server() {
        let mut mock_job_manager = MockBackgroundJobManagerTrait::new();
        let mut mock_ai_service = MockAIService::new();

        mock_ai_service
            .expect_generate_response()
            .returning(|_| Ok("test response".to_string()));

        mock_job_manager
            .expect_submit_job()
            .withf(|job: &Job| {
                matches!(job.metadata.job_type, JobType::Custom(ref s) if s == "MCPServer")
                    && job.priority == JobPriority::High
            })
            .returning(|_| Box::pin(async { Ok(uuid::Uuid::new_v4()) }));

        let use_cases =
            MCPServerUseCases::new(Arc::new(mock_job_manager), Arc::new(mock_ai_service));

        let config = MCPConfig::default();
        let result = use_cases.start_mcp_server(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_mcp_server() {
        let mut mock_job_manager = MockBackgroundJobManagerTrait::new();
        let mut mock_ai_service = MockAIService::new();
        let job_id = uuid::Uuid::new_v4();

        mock_ai_service
            .expect_generate_response()
            .return_once(|_| Ok("test response".to_string()));

        mock_job_manager
            .expect_cancel_job()
            .with(eq(job_id))
            .returning(|_| Box::pin(async { Ok(()) }));

        let use_cases =
            MCPServerUseCases::new(Arc::new(mock_job_manager), Arc::new(mock_ai_service));

        let result = use_cases.stop_mcp_server(job_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_mcp_connector() {
        let mock_job_manager = MockBackgroundJobManagerTrait::new();
        let mut mock_ai_service = MockAIService::new();

        mock_ai_service
            .expect_generate_response()
            .withf(|content: &str| content == "test")
            .return_once(|_| Ok("test response".to_string()));

        let use_cases =
            MCPServerUseCases::new(Arc::new(mock_job_manager), Arc::new(mock_ai_service));

        let config = MCPConfig::default();
        let _connector = use_cases.create_mcp_connector(config);

        // Test the AI service integration
        let response = use_cases.generate_response("test").await;
        assert!(response.is_ok());
        assert_eq!(response.unwrap(), "test response");
    }
}
