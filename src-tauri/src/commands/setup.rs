use crate::{
    domain::{
        entities::NotificationPreferences,
        services::{ai::AIService, NotificationService, ServiceConfigService},
    },
    presentation::dtos::{AIConfigRequest, NotificationPreferencesRequest, SetupError},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct SetupStatus {
    pub is_first_run: bool,
    pub has_services_configured: bool,
    pub has_notifications_configured: bool,
    pub has_ai_configured: bool,
}

const SETUP_STATUS_FILE: &str = "setup_status.json";

/// Check if this is the first run of the application
#[tauri::command]
pub async fn check_first_run() -> Result<bool, SetupError> {
    let app_dir = directories::ProjectDirs::from("com", "autoresponse", "app")
        .ok_or_else(|| SetupError::new("Failed to get app directory"))?
        .data_dir()
        .to_path_buf();

    let status_file = app_dir.join(SETUP_STATUS_FILE);
    if !status_file.exists() {
        return Ok(true);
    }

    let status = std::fs::read_to_string(status_file)
        .map_err(|e| SetupError::new(&format!("Failed to read setup status: {}", e)))?;

    let status: SetupStatus = serde_json::from_str(&status)
        .map_err(|e| SetupError::new(&format!("Failed to parse setup status: {}", e)))?;

    Ok(!status.has_services_configured
        || !status.has_notifications_configured
        || !status.has_ai_configured)
}

#[tauri::command]
pub async fn save_notification_preferences(
    state: tauri::State<'_, Arc<dyn NotificationService>>,
    preferences: NotificationPreferencesRequest,
) -> Result<(), SetupError> {
    // Convert preferences to domain type
    let domain_preferences: NotificationPreferences = preferences.into();

    // Save preferences
    (**state)
        .save_preferences(domain_preferences)
        .await
        .map_err(|e| SetupError::new(&format!("Failed to save notification preferences: {}", e)))?;

    update_setup_status(|status| {
        status.has_notifications_configured = true;
    })
    .map_err(|e| SetupError::new(&format!("Failed to update setup status: {}", e)))?;

    Ok(())
}

#[tauri::command]
pub async fn save_ai_config(
    ai_service: tauri::State<'_, Arc<dyn AIService>>,
    config: AIConfigRequest,
) -> Result<(), SetupError> {
    // Configure AI service - no need for Arc::get_mut since we made configure take &self
    (**ai_service)
        .configure(config.clone())
        .await
        .map_err(|e| SetupError::new(&format!("Failed to configure AI service: {}", e)))?;

    // Save AI configuration to settings
    std::fs::create_dir_all(
        directories::ProjectDirs::from("com", "autoresponse", "app")
            .ok_or_else(|| SetupError::new("Failed to get app directory"))?
            .config_dir(),
    )
    .map_err(|e| SetupError::new(&format!("Failed to create config directory: {}", e)))?;

    let config_file = directories::ProjectDirs::from("com", "autoresponse", "app")
        .ok_or_else(|| SetupError::new("Failed to get app directory"))?
        .config_dir()
        .join("ai_config.json");

    std::fs::write(
        config_file,
        serde_json::to_string_pretty(&config)
            .map_err(|e| SetupError::new(&format!("Failed to serialize AI config: {}", e)))?,
    )
    .map_err(|e| SetupError::new(&format!("Failed to write AI config: {}", e)))?;

    update_setup_status(|status| {
        status.has_ai_configured = true;
    })
    .map_err(|e| SetupError::new(&format!("Failed to update setup status: {}", e)))?;

    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn test_ai_connection(
    ai_service: tauri::State<'_, Arc<dyn AIService>>,
    config: AIConfigRequest,
) -> Result<(), SetupError> {
    (**ai_service)
        .test_connection(&config)
        .await
        .map_err(|e| SetupError::new(&format!("Failed to test AI connection: {}", e)))?;
    Ok(())
}

/// Complete the setup process
#[tauri::command]
pub async fn complete_setup(
    service_config_state: tauri::State<'_, Arc<dyn ServiceConfigService>>,
) -> Result<(), SetupError> {
    // Verify that at least one service is configured
    let services = service_config_state
        .get_enabled_configs()
        .await
        .map_err(|e| SetupError::new(&format!("Failed to get service configs: {}", e)))?;

    if services.is_empty() {
        return Err(SetupError::new(
            "At least one service must be configured to complete setup",
        ));
    }

    update_setup_status(|status| {
        status.has_services_configured = true;
        status.is_first_run = false;
    })
    .map_err(|e| SetupError::new(&format!("Failed to update setup status: {}", e)))?;

    Ok(())
}

fn update_setup_status<F>(update_fn: F) -> Result<(), std::io::Error>
where
    F: FnOnce(&mut SetupStatus),
{
    let app_dir = directories::ProjectDirs::from("com", "autoresponse", "app")
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Failed to get app directory")
        })?
        .data_dir()
        .to_path_buf();

    std::fs::create_dir_all(&app_dir)?;
    let status_file = app_dir.join(SETUP_STATUS_FILE);

    let mut status = if status_file.exists() {
        let status_str = std::fs::read_to_string(&status_file)?;
        serde_json::from_str(&status_str).unwrap_or(SetupStatus {
            is_first_run: true,
            has_services_configured: false,
            has_notifications_configured: false,
            has_ai_configured: false,
        })
    } else {
        SetupStatus {
            is_first_run: true,
            has_services_configured: false,
            has_notifications_configured: false,
            has_ai_configured: false,
        }
    };

    update_fn(&mut status);
    std::fs::write(status_file, serde_json::to_string_pretty(&status)?)?;
    Ok(())
}
