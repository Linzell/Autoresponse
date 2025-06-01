use crate::domain::{
    error::DomainResult,
    services::{
        ai::{DynAIService, MCPConfig},
        background::types::{Job, JobHandler, JobPriority, JobType},
    },
};
use std::{fmt::Debug, sync::Arc};
use tokio::sync::broadcast;
use tracing::{error, info};

#[derive(Debug)]
pub struct MCPServerJob {
    config: MCPConfig,
    ai_service: DynAIService,
    stop_signal: broadcast::Sender<()>,
}

impl MCPServerJob {
    pub fn new(config: MCPConfig, ai_service: DynAIService) -> Self {
        let (stop_signal, _) = broadcast::channel(1);
        Self {
            config,
            ai_service,
            stop_signal,
        }
    }

    pub fn get_stop_signal(&self) -> broadcast::Receiver<()> {
        self.stop_signal.subscribe()
    }

    pub fn stop(&self) {
        if let Err(e) = self.stop_signal.send(()) {
            error!("Failed to send stop signal to MCP server: {}", e);
        }
    }
}

#[async_trait::async_trait]
impl JobHandler for MCPServerJob {
    async fn handle(&self, job: &mut Job) -> Result<(), String> {
        info!("Starting MCP server job");
        let server_config = serde_json::to_value(&self.config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        // Test AI service
        let ai_test = self
            .ai_service
            .generate_response("test")
            .await
            .map_err(|e| format!("AI service test failed: {}", e))?;

        // Update the job payload with the test result
        job.payload = serde_json::json!({
            "config": server_config,
            "test_response": ai_test
        });

        // Create MCP server test payload
        let mcp_test = format!("MCP server test with AI response: {}", ai_test);
        let test_result = self
            .ai_service
            .generate_response(&mcp_test)
            .await
            .map_err(|e| format!("MCP server integration test failed: {}", e))?;

        info!("MCP server integration test successful: {}", test_result);
        job.metadata.job_type = JobType::Custom("MCPServer".to_string());

        info!("MCP server initialized with config: {:?}", self.config);
        Ok(())
    }

    fn job_type(&self) -> JobType {
        JobType::Custom("MCPServer".to_string())
    }
}

impl MCPServerJob {
    #[allow(dead_code)]
    async fn run_server(&self) -> DomainResult<()> {
        // TODO: Implement MCP server startup
        // This will be implemented when we add the server code
        Ok(())
    }
}

pub struct MCPServerJobBuilder {
    config: MCPConfig,
    ai_service: Option<DynAIService>,
}

impl MCPServerJobBuilder {
    pub fn new(config: MCPConfig) -> Self {
        Self {
            config,
            ai_service: None,
        }
    }

    pub fn with_ai_service(mut self, ai_service: DynAIService) -> Self {
        self.ai_service = Some(ai_service);
        self
    }

    pub fn build(self) -> DomainResult<Arc<dyn JobHandler>> {
        let ai_service = self.ai_service.ok_or_else(|| {
            crate::domain::error::DomainError::ValidationError("AI service is required".to_string())
        })?;

        Ok(Arc::new(MCPServerJob::new(self.config, ai_service)))
    }
}

impl Job {
    pub fn mcp_server(config: MCPConfig, ai_service: DynAIService) -> Self {
        let server_job = MCPServerJob::new(config, ai_service);
        let payload = serde_json::to_value(&server_job.config).unwrap_or(serde_json::Value::Null);
        let metadata = JobType::Custom("MCPServer".to_string());
        Self::new(payload, JobPriority::High, metadata, 3)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::domain::services::{ai::MockAIService, JobStatus};
    use std::time::Duration;

    #[tokio::test]
    async fn test_mcp_server_job_builder() {
        let config = MCPConfig::default();
        let mut mock_ai = MockAIService::new();
        mock_ai
            .expect_generate_response()
            .returning(|_| Ok("test response".to_string()));
        let ai_service: DynAIService = Arc::new(mock_ai);

        let builder = MCPServerJobBuilder::new(config.clone()).with_ai_service(ai_service);
        let handler = builder.build().unwrap();

        assert!(Arc::strong_count(&handler) >= 1);
    }

    #[tokio::test]
    async fn test_mcp_server_job_builder_validation() {
        let config = MCPConfig::default();
        let builder = MCPServerJobBuilder::new(config);
        let result = builder.build();

        assert!(result.is_err(), "Expected error for missing AI service");
    }

    #[tokio::test]
    async fn test_mcp_server_job_stop_signal() {
        let config = MCPConfig::default();
        let mut mock_ai = MockAIService::new();
        mock_ai
            .expect_generate_response()
            .returning(|_| Ok("test response".to_string()));
        let ai_service: DynAIService = Arc::new(mock_ai);
        let job = MCPServerJob::new(config, ai_service);

        let mut stop_receiver = job.get_stop_signal();
        job.stop();

        tokio::select! {
            _ = stop_receiver.recv() => {
                // Test passed: stop signal received
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                panic!("Stop signal not received within timeout");
            }
        }
    }

    #[tokio::test]
    async fn test_job_creation() {
        let config = MCPConfig::default();
        let mock_ai = MockAIService::new();
        // No expectations needed for job creation since we're not executing the job
        let ai_service: DynAIService = Arc::new(mock_ai);

        let job = Job::mcp_server(config, ai_service);

        assert_eq!(job.priority, JobPriority::High);
        assert_eq!(job.status, JobStatus::Pending);
        assert_ne!(job.id, uuid::Uuid::nil());
        assert!(matches!(
            job.metadata.job_type,
            JobType::Custom(ref s) if s == "MCPServer"
        ));
    }

    #[tokio::test]
    async fn test_job_execution() {
        let config = MCPConfig::default();
        let mut mock_ai = MockAIService::new();

        // Expect two calls to generate_response:
        // 1. Initial AI service test
        // 2. MCP server test with the response from #1
        mock_ai
            .expect_generate_response()
            .times(2)
            .returning(|_| Ok("test response".to_string()));

        let ai_service: DynAIService = Arc::new(mock_ai);
        let server_job = MCPServerJob::new(config.clone(), ai_service);
        let mut job = Job::mcp_server(config.clone(), server_job.ai_service.clone());

        // Execute the job handler
        let result = server_job.handle(&mut job).await;
        assert!(result.is_ok(), "Job execution should succeed");

        // Verify job payload was updated with test results
        if let serde_json::Value::Object(payload) = job.payload {
            assert!(payload.contains_key("config"));
            assert!(payload.contains_key("test_response"));
            assert_eq!(
                payload.get("test_response").and_then(|v| v.as_str()),
                Some("test response")
            );
        } else {
            panic!("Job payload should be an object");
        }
    }
}
