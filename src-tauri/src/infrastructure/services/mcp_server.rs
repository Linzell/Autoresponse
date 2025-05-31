use crate::domain::{
    error::{DomainError, DomainResult},
    services::{
        ai::{AIAnalysis, DynAIService},
        search::{SearchResult, SearchService},
    },
};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

#[derive(Debug, Clone, Serialize)]
pub struct MCPServerConfig {
    pub host: String,
    pub port: u16,
    pub allowed_origins: Vec<String>,
}

impl Default for MCPServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 5000,
            allowed_origins: vec!["http://localhost:1420".to_string()],
        }
    }
}

#[derive(Debug, Clone)]
pub struct MCPServer {
    config: MCPServerConfig,
    ai_service: DynAIService,
    search_service: Arc<dyn SearchService>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MCPRequest {
    content: String,
    api_key: String,
    service_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchRequest {
    query: String,
    api_key: String,
}

// Using domain SearchResult instead of local definition

#[derive(Debug, Serialize, Deserialize)]
struct MCPResponse<T> {
    success: bool,
    response: Option<T>,
    error: Option<String>,
}

impl<T> MCPResponse<T> {
    fn success(response: T) -> Self {
        Self {
            success: true,
            response: Some(response),
            error: None,
        }
    }

    fn error(error: String) -> Self {
        Self {
            success: false,
            response: None,
            error: Some(error),
        }
    }
}

impl MCPServer {
    pub fn new(
        config: MCPServerConfig,
        ai_service: DynAIService,
        search_service: Arc<dyn SearchService>,
    ) -> Self {
        Self {
            config,
            ai_service,
            search_service,
        }
    }

    pub async fn start(&self) -> DomainResult<()> {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app_state = Arc::new(AppState {
            ai_service: self.ai_service.clone(),
            search_service: self.search_service.clone(),
        });

        let app = Router::new()
            .route("/health", get(health_check))
            .route("/api/analyze", post(analyze_content))
            .route("/api/generate", post(generate_response))
            .route("/api/search", post(web_search))
            .layer(cors)
            .with_state(app_state);

        let addr = format!("{}:{}", self.config.host, self.config.port)
            .parse::<SocketAddr>()
            .map_err(|e| DomainError::ConfigurationError(format!("Invalid address: {}", e)))?;

        info!("Starting MCP server on {}", addr);

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| DomainError::ConfigurationError(format!("Failed to bind: {}", e)))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| DomainError::ExternalServiceError(format!("Server error: {}", e)))?;

        Ok(())
    }
}

#[derive(Clone)]
struct AppState {
    ai_service: DynAIService,
    search_service: Arc<dyn SearchService>,
}

async fn health_check() -> (StatusCode, Json<MCPResponse<String>>) {
    (
        StatusCode::OK,
        Json(MCPResponse::success("MCP Server is running".to_string())),
    )
}

async fn analyze_content(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MCPRequest>,
) -> (StatusCode, Json<MCPResponse<AIAnalysis>>) {
    // Validate request
    if request.api_key.is_empty() || request.service_type != "analyze" {
        return (
            StatusCode::BAD_REQUEST,
            Json(MCPResponse::error("Invalid request parameters".to_string())),
        );
    }

    match state.ai_service.analyze_content(&request.content).await {
        Ok(analysis) => (StatusCode::OK, Json(MCPResponse::success(analysis))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(MCPResponse::error(e.to_string())),
        ),
    }
}

async fn generate_response(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MCPRequest>,
) -> (StatusCode, Json<MCPResponse<String>>) {
    // Validate request
    if request.api_key.is_empty() || request.service_type != "generate" {
        return (
            StatusCode::BAD_REQUEST,
            Json(MCPResponse::error("Invalid request parameters".to_string())),
        );
    }

    match state.ai_service.generate_response(&request.content).await {
        Ok(response) => (StatusCode::OK, Json(MCPResponse::success(response))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(MCPResponse::error(e.to_string())),
        ),
    }
}

async fn web_search(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SearchRequest>,
) -> (StatusCode, Json<MCPResponse<Vec<SearchResult>>>) {
    // Validate request
    if request.api_key.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(MCPResponse::error("Invalid API key".to_string())),
        );
    }

    match state.search_service.search(&request.query).await {
        Ok(results) => (StatusCode::OK, Json(MCPResponse::success(results))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(MCPResponse::error(e.to_string())),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::services::{
        ai::{MockAIService, PriorityLevel},
        search::MockSearchService,
    };
    use axum::body::Body;
    use axum::http::{Method, Request};
    use http_body_util::BodyExt;
    use mockall::predicate::*;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let state = Arc::new(AppState {
            ai_service: Arc::new(MockAIService::new()),
            search_service: Arc::new(MockSearchService::new()),
        });

        let app = Router::new()
            .route("/health", get(health_check))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .method(Method::GET)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let response: MCPResponse<String> = serde_json::from_slice(&body).unwrap();
        assert!(response.success);
        assert_eq!(
            response.response.unwrap(),
            "MCP Server is running".to_string()
        );
    }

    #[tokio::test]
    async fn test_analyze_content() {
        let mut mock_ai = MockAIService::new();
        mock_ai
            .expect_analyze_content()
            .with(eq("test content"))
            .returning(|_| {
                Ok(AIAnalysis {
                    requires_action: true,
                    priority_level: PriorityLevel::High,
                    summary: "Test summary".to_string(),
                    suggested_actions: vec!["Action 1".to_string()],
                })
            });

        let state = Arc::new(AppState {
            ai_service: Arc::new(mock_ai),
            search_service: Arc::new(MockSearchService::new()),
        });

        let app = Router::new()
            .route("/api/analyze", post(analyze_content))
            .with_state(state);

        let request_body = json!({
            "content": "test content",
            "api_key": "test-key",
            "service_type": "analyze"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/analyze")
                    .method(Method::POST)
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let response: MCPResponse<AIAnalysis> = serde_json::from_slice(&body).unwrap();
        assert!(response.success);
        assert!(response.response.unwrap().requires_action);
    }

    #[tokio::test]
    async fn test_generate_response() {
        let mut mock_ai = MockAIService::new();
        mock_ai
            .expect_generate_response()
            .with(eq("test context"))
            .returning(|_| Ok("Generated response".to_string()));

        let state = Arc::new(AppState {
            ai_service: Arc::new(mock_ai),
            search_service: Arc::new(MockSearchService::new()),
        });

        let app = Router::new()
            .route("/api/generate", post(generate_response))
            .with_state(state);

        let request_body = json!({
            "content": "test context",
            "api_key": "test-key",
            "service_type": "generate"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/generate")
                    .method(Method::POST)
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let response: MCPResponse<String> = serde_json::from_slice(&body).unwrap();
        assert!(response.success);
        assert_eq!(response.response.unwrap(), "Generated response");
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mut mock_ai = MockAIService::new();
        mock_ai
            .expect_generate_response()
            .with(eq("error test"))
            .returning(|_| Err(DomainError::ValidationError("Test error".to_string())));

        let state = Arc::new(AppState {
            ai_service: Arc::new(mock_ai),
            search_service: Arc::new(MockSearchService::new()),
        });

        let app = Router::new()
            .route("/api/generate", post(generate_response))
            .with_state(state);

        let request_body = json!({
            "content": "error test",
            "api_key": "test-key",
            "service_type": "generate"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/generate")
                    .method(Method::POST)
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let response: MCPResponse<String> = serde_json::from_slice(&body).unwrap();
        assert!(!response.success);
        assert!(response.error.unwrap().contains("Test error"));
    }

    #[tokio::test]
    async fn test_web_search() {
        let mut mock_search = MockSearchService::new();
        mock_search
            .expect_search()
            .with(eq("test query"))
            .returning(|_| {
                Ok(vec![SearchResult {
                    title: "Test Result".to_string(),
                    description: "Test Description".to_string(),
                    url: "http://test.com".to_string(),
                }])
            });

        let state = Arc::new(AppState {
            ai_service: Arc::new(MockAIService::new()),
            search_service: Arc::new(mock_search),
        });

        let app = Router::new()
            .route("/api/search", post(web_search))
            .with_state(state);

        let request_body = json!({
            "query": "test query",
            "api_key": "test-key"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/search")
                    .method(Method::POST)
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let response: MCPResponse<Vec<SearchResult>> = serde_json::from_slice(&body).unwrap();
        assert!(response.success);
        let results = response.response.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Test Result");
        assert_eq!(results[0].description, "Test Description");
        assert_eq!(results[0].url, "http://test.com");
    }
}
