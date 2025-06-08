# Autoresponse Development Guide

## Table of Contents

1. [Project Structure](#project-structure)
2. [Implementation Patterns](#implementation-patterns)
3. [Adding New Features](#adding-new-features)
4. [Testing Guidelines](#testing-guidelines)
5. [Error Handling](#error-handling)
6. [Best Practices](#best-practices)

## Project Structure

```
src-tauri/src/
├── application/           # Application use cases and business logic
│   └── use_cases/        # Core application functionality
├── domain/               # Core domain model and business rules
│   ├── entities/         # Domain entities and value objects
│   ├── repositories/     # Repository interfaces
│   ├── services/         # Domain service interfaces
│   └── events/          # Domain events and handlers
├── infrastructure/       # External implementations and adapters
│   ├── repositories/    # Repository implementations
│   └── services/        # External service integrations
└── presentation/        # API and UI layer
    ├── controllers/     # Request handlers
    └── dtos/           # Data transfer objects
```

## Implementation Patterns

### 1. Adding a New Service

#### a. Define Domain Entity

```rust
// domain/entities/example_service.rs
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleService {
    pub id: Uuid,
    pub name: String,
    pub service_type: ServiceType,
    pub config: ExampleConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>
}

impl ExampleService {
    pub fn new(name: String, config: ExampleConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            service_type: ServiceType::Example,
            config,
            created_at: Utc::now(),
            updated_at: Utc::now()
        }
    }
}
```

#### b. Create Service Interface

```rust
// domain/services/example_service.rs
use async_trait::async_trait;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait ExampleService: Send + Sync {
    async fn create_example(
        &self,
        name: String,
        config: ExampleConfig
    ) -> DomainResult<ExampleService>;

    async fn get_example(&self, id: Uuid) -> DomainResult<ExampleService>;
    async fn get_all_examples(&self) -> DomainResult<Vec<ExampleService>>;
    async fn update_example(&self, id: Uuid, config: ExampleConfig) -> DomainResult<()>;
    async fn delete_example(&self, id: Uuid) -> DomainResult<()>;
}

pub type DynExampleService = Arc<dyn ExampleService>;
```

#### c. Implement Service

```rust
// domain/services/example_service_impl.rs
pub struct DefaultExampleService {
    repository: DynExampleRepository,
}

#[async_trait]
impl ExampleService for DefaultExampleService {
    async fn create_example(
        &self,
        name: String,
        config: ExampleConfig
    ) -> DomainResult<ExampleService> {
        let mut example = ExampleService::new(name, config);
        self.repository.save(&mut example).await?;
        Ok(example)
    }

    // Implement other methods...
}
```

#### d. Define DTOs

```rust
// presentation/dtos/example.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateExampleRequest {
    pub name: String,
    pub config: ExampleConfig
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExampleResponse {
    pub id: String,
    pub name: String,
    pub config: ExampleConfig,
    pub created_at: String,
    pub updated_at: String
}

impl From<ExampleService> for ExampleResponse {
    fn from(service: ExampleService) -> Self {
        Self {
            id: service.id.to_string(),
            name: service.name,
            config: service.config,
            created_at: service.created_at.to_rfc3339(),
            updated_at: service.updated_at.to_rfc3339()
        }
    }
}
```

#### e. Create Controller

```rust
// presentation/controllers/example_controller.rs
pub struct ExampleController {
    service: Arc<dyn ExampleService>,
}

impl ExampleController {
    pub fn new(service: Arc<dyn ExampleService>) -> Self {
        Self { service }
    }

    pub async fn create_example(
        &self,
        request: CreateExampleRequest,
    ) -> Result<ExampleResponse, ExampleError> {
        let example = self
            .service
            .create_example(request.name, request.config)
            .await
            .map_err(ExampleError::from)?;

        Ok(example.into())
    }
}
```

### 2. Testing Patterns

#### a. Service Testing Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_request() -> CreateExampleRequest {
        CreateExampleRequest {
            name: "Test Example".to_string(),
            config: ExampleConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_create_example() {
        let mut mock_service = MockExampleService::new();
        let request = create_test_request();

        mock_service
            .expect_create_example()
            .withf(move |name: &String, config: &ExampleConfig| {
                name == "Test Example" && config == &ExampleConfig::default()
            })
            .returning(|name, config| {
                Ok(ExampleService::new(name, config))
            });

        let controller = ExampleController::new(Arc::new(mock_service));
        let result = controller.create_example(request).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.name, "Test Example");
    }
}
```

#### b. MCP Server Testing Pattern

```rust
struct TestContext {
    ai_service: DynAIService,
    search_service: Arc<dyn SearchService>,
    mock_server: Option<MockServer>,
    server_port: u16,
}

#[tokio::test]
async fn test_mcp_server_endpoint() -> DomainResult<()> {
    // 1. Set up test context
    let ctx = TestContext::new().await;
    let server_config = ctx.get_server_config();

    // 2. Configure mock responses
    let mock_server = ctx.mock_server.as_ref().unwrap();
    Mock::given(method("POST"))
        .and(path("/api/endpoint"))
        .respond_with(ResponseTemplate::new(200).set_body_json(expected_response))
        .mount(mock_server)
        .await;

    // 3. Create and start server
    let server = MCPServer::new(
        server_config.clone(),
        ctx.ai_service.clone(),
        ctx.search_service.clone(),
    );
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    // 4. Send test request
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}:{}/api/endpoint", host, port))
        .json(&test_request)
        .send()
        .await?;

    // 5. Verify response
    assert_eq!(response.status(), 200);
    let body: MCPResponse<T> = response.json().await?;
    assert!(body.success);

    // 6. Clean up
    server_handle.abort();
    Ok(())
}
```

## Adding New Features

1. **Domain First Approach**

   - Start with domain entities and interfaces
   - Define clear boundaries and behaviors
   - Use value objects for complex attributes

2. **Layer Implementation Order**

   1. Domain layer (entities, interfaces)
   2. Infrastructure layer (repositories, external services)
   3. Application layer (use cases)
   4. Presentation layer (controllers, DTOs)

3. **Feature Checklist**
   - [ ] Domain entities and value objects
   - [ ] Repository interface and implementation
   - [ ] Service interface and implementation
   - [ ] DTOs for requests and responses
   - [ ] Controller implementation
   - [ ] Unit tests for all components
   - [ ] Integration tests for critical flows

## Testing Guidelines

1. **Unit Tests**

   - Test each component in isolation
   - Use mocks for dependencies
   - Cover error cases and edge scenarios
   - Aim for 100% coverage
   - Follow DRY principle with test utilities and fixtures
   - Use descriptive test names that indicate scenario and expected outcome

2. **Integration Tests**

   - Test complete workflows
   - Use test databases and mock servers
   - Cover primary use cases
   - Test error handling
   - Implement proper test context management
   - Use TestContext pattern for complex service testing
   - Handle async operations and cleanup properly
   - Test both success and failure scenarios

3. **Test Structure**
   - Arrange: Set up test data and mocks
   - Act: Execute the code under test
   - Assert: Verify the results
   - Cleanup: Clean up resources

## Error Handling

1. **Domain Errors**

   - Use custom error types
   - Include context and details
   - Map to appropriate HTTP status codes

2. **Error Types**

   - ValidationError: Input validation failures
   - NotFoundError: Resource not found
   - AuthError: Authentication/authorization failures
   - ServiceError: External service failures

3. **Error Response Format**

```rust
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub details: Vec<String>,
}
```

## Best Practices

1. **Code Organization**

   - Maximum file size: 500 lines
   - Maximum function size: 50 lines
   - Maximum nesting depth: 3 levels

2. **Documentation**

   - Document public interfaces
   - Include examples in doc comments
   - Keep documentation up to date

3. **Error Handling**

   - Use Result types consistently
   - Provide context with errors
   - Handle all error cases

4. **Testing**

   - Write tests first (TDD)
   - Mock external dependencies
   - Use meaningful test names

5. **Performance**

   - Use async/await appropriately
   - Implement proper caching
   - Monitor resource usage

6. **Security**
   - Validate all inputs
   - Handle sensitive data securely
   - Implement proper authentication

For additional assistance or to report issues, please refer to the GitHub repository or contact the maintainers.
