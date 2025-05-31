use autoresponse_lib::application::use_cases::MCPServerUseCases;
use autoresponse_lib::domain::error::DomainResult;
use autoresponse_lib::domain::services::{
    ai::{AIAnalysis, DynAIService, MCPConfig, PriorityLevel},
    background::types::Job,
    search::{SearchResult, SearchService},
};
use autoresponse_lib::infrastructure::services::mcp_server::{MCPServer, MCPServerConfig};
use parking_lot::Mutex;
use std::{net::TcpListener, sync::Arc, time::Duration};
use tokio;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use crate::common::{MockAIService, MockSearchService};

struct TestContext {
    ai_service: DynAIService,
    search_service: Arc<dyn SearchService>,
    mock_server: Option<MockServer>,
    server_port: u16,
}

impl TestContext {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let mock_server = MockServer::start().await;

        let mut mock_search = MockSearchService::new();
        mock_search.expect_search().returning(|_| {
            Ok(vec![SearchResult {
                title: "Test Result".to_string(),
                description: "Test Description".to_string(),
                url: "http://test.com".to_string(),
            }])
        });
        let search_service = Arc::new(mock_search);
        let mut mock_ai = MockAIService::new_with_defaults();
        mock_ai.expect_analyze_content().returning(|_| {
            Ok(AIAnalysis {
                requires_action: true,
                priority_level: PriorityLevel::High,
                summary: "Test summary".to_string(),
                suggested_actions: vec!["Action 1".to_string()],
            })
        });
        mock_ai
            .expect_generate_response()
            .returning(|_| Ok("test response".to_string()));
        let ai_service = Arc::new(mock_ai);

        Self {
            ai_service,
            search_service,
            mock_server: Some(mock_server),
            server_port: port,
        }
    }

    fn get_server_config(&self) -> MCPServerConfig {
        MCPServerConfig {
            host: "127.0.0.1".to_string(),
            port: self.server_port,
            allowed_origins: vec!["http://localhost:1420".to_string()],
        }
    }
}

#[tokio::test]
async fn test_mcp_server_startup() -> DomainResult<()> {
    let ctx = TestContext::new().await;
    let server_config = ctx.get_server_config();
    let server = MCPServer::new(
        server_config.clone(),
        ctx.ai_service.clone(),
        ctx.search_service.clone(),
    );

    // Spawn the server in a separate task
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test server health endpoint
    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "http://{}:{}/health",
            server_config.host, server_config.port
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("success"));

    // Clean up
    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_mcp_server_analyze_endpoint() -> DomainResult<()> {
    let ctx = TestContext::new().await;
    let mock_server = ctx.mock_server.as_ref().unwrap();

    // Setup mock response for analysis
    let sample_response = r#"{
        "success": true,
        "response": "{\"requires_action\": true, \"priority_level\": \"High\", \"summary\": \"Test summary\", \"suggested_actions\": [\"Action 1\"]}",
        "error": null
    }"#;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sample_response, "application/json"))
        .mount(mock_server)
        .await;

    let server_config = ctx.get_server_config();
    let server = MCPServer::new(
        server_config.clone(),
        ctx.ai_service.clone(),
        ctx.search_service.clone(),
    );

    // Start server
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test analyze endpoint
    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://{}:{}/api/analyze",
            server_config.host, server_config.port
        ))
        .json(&serde_json::json!({
            "content": "test content",
            "api_key": "test-key",
            "service_type": "analyze"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Clean up
    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_mcp_use_cases() -> DomainResult<()> {
    let ctx = TestContext::new().await;
    let executed_jobs = Arc::new(Mutex::new(Vec::new()));
    let job_manager = Arc::new(TestJobManager {
        executed_jobs: executed_jobs.clone(),
    });

    let use_cases = MCPServerUseCases::new(job_manager.clone(), ctx.ai_service.clone());

    // Test starting MCP server
    let config = MCPConfig::default();
    let job_id = use_cases.start_mcp_server(config.clone()).await?;

    // Verify job was submitted
    {
        let jobs = executed_jobs.lock();
        assert!(!jobs.is_empty());
        assert_eq!(jobs[0].id, job_id);
    }

    // Test stopping MCP server
    use_cases.stop_mcp_server(job_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_mcp_server_search_endpoint() -> DomainResult<()> {
    let ctx = TestContext::new().await;
    let mock_server = ctx.mock_server.as_ref().unwrap();

    // Setup mock response for search
    let sample_response = r#"{
        "success": true,
        "response": [
            {
                "title": "Test Result",
                "description": "Test Description",
                "url": "http://test.com"
            }
        ],
        "error": null
    }"#;

    Mock::given(method("POST"))
        .and(path("/api/search"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sample_response, "application/json"))
        .mount(mock_server)
        .await;

    let server_config = ctx.get_server_config();
    let server = MCPServer::new(
        server_config.clone(),
        ctx.ai_service.clone(),
        ctx.search_service.clone(),
    );

    // Start server
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test search endpoint
    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://{}:{}/api/search",
            server_config.host, server_config.port
        ))
        .json(&serde_json::json!({
            "query": "test query",
            "api_key": "test-key"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let results = body["response"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["title"], "Test Result");
    assert_eq!(results[0]["description"], "Test Description");
    assert_eq!(results[0]["url"], "http://test.com");

    // Clean up
    server_handle.abort();
    Ok(())
}

// Test job manager implementation
#[derive(Debug)]
struct TestJobManager {
    executed_jobs: Arc<Mutex<Vec<Job>>>,
}

#[async_trait::async_trait]
impl autoresponse_lib::domain::services::background::manager::BackgroundJobManagerTrait
    for TestJobManager
{
    async fn submit_job(&self, job: Job) -> DomainResult<Uuid> {
        let job_id = job.id;
        let mut jobs = self.executed_jobs.lock();
        jobs.push(job);
        Ok(job_id)
    }

    async fn cancel_job(&self, job_id: Uuid) -> DomainResult<()> {
        let mut jobs = self.executed_jobs.lock();
        if let Some(pos) = jobs.iter().position(|job| job.id == job_id) {
            jobs.remove(pos);
        }
        Ok(())
    }

    async fn get_job_status(
        &self,
        job_id: Uuid,
    ) -> Option<autoresponse_lib::domain::services::background::types::JobStatus> {
        let jobs = self.executed_jobs.lock();
        jobs.iter()
            .find(|job| job.id == job_id)
            .map(|job| job.status.clone())
    }

    async fn register_handler(
        &self,
        _handler: Arc<dyn autoresponse_lib::domain::services::background::types::JobHandler>,
    ) -> DomainResult<()> {
        // Simulate some processing time
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }
}
