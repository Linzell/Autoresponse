use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::domain::entities::{AuthConfig, ServiceConfig, ServiceEndpoints, ServiceType};

#[derive(Debug, Default)]
pub struct ServiceConfigRepository {
    configs: Arc<Mutex<HashMap<Uuid, ServiceConfig>>>,
}

impl ServiceConfigRepository {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn save(&self, config: ServiceConfig) -> Result<ServiceConfig, String> {
        let mut configs = self.configs.lock().map_err(|e| e.to_string())?;
        let config_clone = config.clone();
        configs.insert(config.id, config);
        Ok(config_clone)
    }

    pub fn find_by_id(&self, id: &Uuid) -> Result<Option<ServiceConfig>, String> {
        let configs = self.configs.lock().map_err(|e| e.to_string())?;
        Ok(configs.get(id).cloned())
    }

    pub fn find_by_service_type(&self, service_type: &ServiceType) -> Result<Vec<ServiceConfig>, String> {
        let configs = self.configs.lock().map_err(|e| e.to_string())?;
        Ok(configs
            .values()
            .filter(|config| config.service_type == *service_type)
            .cloned()
            .collect())
    }

    pub fn list_all(&self) -> Result<Vec<ServiceConfig>, String> {
        let configs = self.configs.lock().map_err(|e| e.to_string())?;
        Ok(configs.values().cloned().collect())
    }

    pub fn update(
        &self,
        id: &Uuid,
        auth_config: Option<AuthConfig>,
        endpoints: Option<ServiceEndpoints>,
        enabled: Option<bool>,
        metadata: Option<Value>,
    ) -> Result<ServiceConfig, String> {
        let mut configs = self.configs.lock().map_err(|e| e.to_string())?;

        let config = configs
            .get_mut(id)
            .ok_or_else(|| "Service configuration not found".to_string())?;

        if let Some(auth_config) = auth_config {
            config.update_auth_config(auth_config);
        }

        if let Some(endpoints) = endpoints {
            config.update_endpoints(endpoints);
        }

        if let Some(enabled) = enabled {
            config.set_enabled(enabled);
        }

        if let Some(metadata) = metadata {
            config.update_metadata(metadata);
        }

        config.updated_at = Utc::now();
        Ok(config.clone())
    }

    pub fn delete(&self, id: &Uuid) -> Result<(), String> {
        let mut configs = self.configs.lock().map_err(|e| e.to_string())?;
        configs
            .remove(id)
            .ok_or_else(|| "Service configuration not found".to_string())?;
        Ok(())
    }

    pub fn get_active_configs(&self) -> Result<Vec<ServiceConfig>, String> {
        let configs = self.configs.lock().map_err(|e| e.to_string())?;
        Ok(configs
            .values()
            .filter(|config| config.enabled)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        entities::{OAuth2Config, ServiceConfig, ServiceEndpoints, ServiceType},
        AuthType,
    };

    fn create_test_config() -> ServiceConfig {
        let oauth2_config = OAuth2Config {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:8080/callback".to_string(),
            auth_url: "http://auth.example.com/oauth/authorize".to_string(),
            token_url: "http://auth.example.com/oauth/token".to_string(),
            scope: vec!["read".to_string(), "write".to_string()],
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
        };

        let mut endpoints = serde_json::Map::new();
        endpoints.insert(
            "notifications".to_string(),
            serde_json::json!({
                "path": "/api/notifications",
                "method": "GET"
            }),
        );

        let service_endpoints = ServiceEndpoints {
            base_url: "http://api.example.com".to_string(),
            endpoints,
        };

        ServiceConfig::new(
            "Test Service".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth2_config),
            service_endpoints,
        )
    }

    #[test]
    fn test_save_and_find_by_id() {
        let repo = ServiceConfigRepository::new();
        let config = create_test_config();
        let id = config.id;

        let saved_config = repo.save(config).unwrap();
        assert_eq!(saved_config.id, id);

        let found_config = repo.find_by_id(&id).unwrap().unwrap();
        assert_eq!(found_config.id, id);
    }

    #[test]
    fn test_find_by_type() {
        let repo = ServiceConfigRepository::new();
        let config = create_test_config();
        let service_type = config.service_type.clone();

        repo.save(config).unwrap();

        let configs = repo.find_by_type(&service_type).unwrap();
        assert_eq!(configs.len(), 1);
        assert!(matches!(configs[0].service_type, ServiceType::Github));
    }

    #[test]
    fn test_update() {
        let repo = ServiceConfigRepository::new();
        let config = create_test_config();
        let id = config.id;

        repo.save(config).unwrap();

        let metadata = serde_json::json!({
            "key": "value"
        });

        let updated_config = repo
            .update(&id, None, None, Some(false), Some(metadata.clone()))
            .unwrap();

        assert!(!updated_config.enabled);
        assert_eq!(updated_config.metadata, metadata);
    }

    #[test]
    fn test_delete() {
        let repo = ServiceConfigRepository::new();
        let config = create_test_config();
        let id = config.id;

        repo.save(config).unwrap();
        repo.delete(&id).unwrap();

        assert!(repo.find_by_id(&id).unwrap().is_none());
    }

    #[test]
    fn test_get_active_configs() {
        let repo = ServiceConfigRepository::new();
        let mut config = create_test_config();

        repo.save(config.clone()).unwrap();

        config.set_enabled(false);
        config.id = Uuid::new_v4();
        repo.save(config).unwrap();

        let active_configs = repo.get_active_configs().unwrap();
        assert_eq!(active_configs.len(), 1);
    }
}
