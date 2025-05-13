use crate::common::{
    mock::MockServiceConfig,
    test_utils::{create_test_oauth2_config, create_test_service_config},
};
use anyhow::Result;
use autoresponse_lib::{
    application::use_cases::service_config_use_cases::ServiceConfigUseCases,
    domain::entities::{
        ApiKeyConfig, AuthConfig, AuthType, BasicAuthConfig, ServiceConfig, ServiceEndpoints,
        ServiceType,
    },
};
use std::sync::Arc;
use tokio;
use uuid::Uuid;

#[tokio::test]
async fn test_service_config_lifecycle() -> Result<()> {
    let mut mock = MockServiceConfig::new();
    let service_config_id = Uuid::new_v4();

    mock.expect_create_service_config().times(1).returning(
        move |name, service_type, auth_type, auth_config, endpoints| {
            let mut config =
                ServiceConfig::new(name, service_type, auth_type, auth_config, endpoints);
            config.id = service_config_id;
            Ok(config)
        },
    );

    mock.expect_get_service_config()
        .times(1)
        .returning(move |id| {
            Ok(ServiceConfig {
                id,
                name: "Test Service".to_string(),
                service_type: ServiceType::Github,
                auth_type: AuthType::OAuth2,
                auth_config: AuthConfig::OAuth2(create_test_oauth2_config()),
                endpoints: ServiceEndpoints {
                    base_url: "http://api.example.com".to_string(),
                    endpoints: serde_json::Map::new(),
                },
                enabled: true,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                last_sync: None,
                metadata: serde_json::Value::Object(serde_json::Map::new()),
            })
        });

    mock.expect_enable_service().times(1).returning(|_| Ok(()));
    mock.expect_disable_service().times(1).returning(|_| Ok(()));
    mock.expect_delete_service_config()
        .times(1)
        .returning(|_| Ok(()));

    let use_cases = ServiceConfigUseCases::new(Arc::new(mock));
    let oauth2_config = create_test_oauth2_config();
    let endpoints = ServiceEndpoints {
        base_url: "http://api.example.com".to_string(),
        endpoints: serde_json::Map::new(),
    };

    let config = use_cases
        .create_service_config(
            "Test Service".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth2_config.clone()),
            endpoints,
        )
        .await?;

    assert_eq!(config.name, "Test Service");
    assert!(matches!(config.service_type, ServiceType::Github));
    assert!(matches!(config.auth_type, AuthType::OAuth2));

    let retrieved = use_cases.get_service_config(config.id).await?;
    assert_eq!(retrieved.name, "Test Service");
    assert_eq!(retrieved.service_type, config.service_type);

    use_cases.enable_service(config.id).await?;
    use_cases.disable_service(config.id).await?;
    use_cases.delete_service_config(config.id).await?;

    Ok(())
}

#[tokio::test]
async fn test_service_config_auth_types() -> Result<()> {
    let mut mock = MockServiceConfig::new();

    mock.expect_create_service_config().times(3).returning(
        |name, service_type, auth_type, auth_config, endpoints| {
            Ok(ServiceConfig::new(
                name,
                service_type,
                auth_type,
                auth_config,
                endpoints,
            ))
        },
    );

    let use_cases = ServiceConfigUseCases::new(Arc::new(mock));
    let endpoints = ServiceEndpoints {
        base_url: "http://api.example.com".to_string(),
        endpoints: serde_json::Map::new(),
    };

    // Test OAuth2
    let oauth2_config = create_test_oauth2_config();
    let oauth2_service = use_cases
        .create_service_config(
            "OAuth2 Service".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth2_config),
            endpoints.clone(),
        )
        .await?;
    assert!(matches!(oauth2_service.auth_type, AuthType::OAuth2));

    // Test Basic Auth
    let basic_auth = BasicAuthConfig {
        username: "test_user".to_string(),
        password: "test_pass".to_string(),
    };
    let basic_auth_service = use_cases
        .create_service_config(
            "Basic Auth Service".to_string(),
            ServiceType::Gitlab,
            AuthType::BasicAuth,
            AuthConfig::BasicAuth(basic_auth),
            endpoints.clone(),
        )
        .await?;
    assert!(matches!(basic_auth_service.auth_type, AuthType::BasicAuth));

    // Test API Key
    let api_key = ApiKeyConfig {
        key: "test_key".to_string(),
        header_name: Some("X-API-Key".to_string()),
    };
    let api_key_service = use_cases
        .create_service_config(
            "API Key Service".to_string(),
            ServiceType::Jira,
            AuthType::ApiKey,
            AuthConfig::ApiKey(api_key),
            endpoints.clone(),
        )
        .await?;
    assert!(matches!(api_key_service.auth_type, AuthType::ApiKey));

    Ok(())
}

#[tokio::test]
async fn test_service_config_filtering() -> Result<()> {
    let mut mock = MockServiceConfig::new();

    mock.expect_get_configs_by_service_type()
        .times(1)
        .returning(|service_type| {
            Ok(vec![create_test_service_config(
                service_type,
                AuthType::OAuth2,
                AuthConfig::OAuth2(create_test_oauth2_config()),
            )])
        });

    mock.expect_get_enabled_configs().times(1).returning(|| {
        Ok(vec![
            create_test_service_config(
                ServiceType::Github,
                AuthType::OAuth2,
                AuthConfig::OAuth2(create_test_oauth2_config()),
            ),
            create_test_service_config(
                ServiceType::Gitlab,
                AuthType::BasicAuth,
                AuthConfig::BasicAuth(BasicAuthConfig {
                    username: "test".to_string(),
                    password: "test".to_string(),
                }),
            ),
        ])
    });

    let use_cases = ServiceConfigUseCases::new(Arc::new(mock));

    let github_configs = use_cases
        .get_service_configs_by_type(ServiceType::Github)
        .await?;
    assert_eq!(github_configs.len(), 1);
    assert!(matches!(
        github_configs[0].service_type,
        ServiceType::Github
    ));

    let enabled_configs = use_cases.get_enabled_service_configs().await?;
    assert_eq!(enabled_configs.len(), 2);

    Ok(())
}
