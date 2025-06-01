pub mod application;
pub mod commands;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
#[cfg(test)]
pub mod test_utils;

use application::{use_cases::MCPServerUseCases, NotificationUseCases, ServiceConfigUseCases};
use commands::oauth::{
    delete_oauth_service_config, get_service_configs, handle_oauth_callback, save_oauth_config,
    start_oauth_flow,
};
use domain::{
    services::{
        actions::ActionExecutor,
        ai::{AIConfig, MCPConfig, OllamaService},
        background::BackgroundJobManager,
        DefaultNotificationService, DefaultServiceConfigService, NotificationService,
        ServiceConfigService,
    },
    NotificationRepository, ServiceConfigRepository,
};
use infrastructure::repositories::{SqliteNotificationRepository, SqliteServiceConfigRepository};
use infrastructure::services::oauth::DefaultOAuthService;
use infrastructure::services::oauth::OAuthService;
use presentation::{
    controllers::{NotificationController, ServiceConfigController},
    dtos::{
        CreateNotificationRequest, CreateServiceConfigRequest, NotificationError,
        NotificationFilterRequest, NotificationListResponse, NotificationResponse,
        ServiceConfigError, ServiceConfigResponse, UpdateServiceAuthRequest, ValidationError,
    },
    middleware::validate_command,
    ServiceConfigListResponse,
};
use std::sync::Arc;
use tracing::info;

// Service Config Commands
#[tauri::command(rename_all = "snake_case")]
async fn create_service_config(
    state: tauri::State<'_, ServiceConfigController>,
    request_json: String,
) -> Result<ServiceConfigResponse, ValidationError> {
    validate_command::<CreateServiceConfigRequest, _, _, _, ValidationError>(
        &request_json,
        |request| async move {
            state
                .create_service_config(request)
                .await
                .map_err(|e| ValidationError::from_message(&e.message))
        },
    )
    .await
}

#[tauri::command(rename_all = "snake_case")]
async fn get_service_config(
    state: tauri::State<'_, ServiceConfigController>,
    id: String,
) -> Result<ServiceConfigResponse, ServiceConfigError> {
    state.get_service_config(id).await
}

#[tauri::command(rename_all = "snake_case")]
async fn get_all_service_configs(
    state: tauri::State<'_, ServiceConfigController>,
) -> Result<ServiceConfigListResponse, ServiceConfigError> {
    state.get_all_service_configs().await
}

#[tauri::command(rename_all = "snake_case")]
async fn update_auth_config(
    state: tauri::State<'_, ServiceConfigController>,
    id: String,
    request_json: String,
) -> Result<(), ValidationError> {
    validate_command::<UpdateServiceAuthRequest, _, _, _, ValidationError>(
        &request_json,
        |request| async move {
            state
                .update_auth_config(id, request)
                .await
                .map_err(|e| ValidationError::from_message(&e.message))
        },
    )
    .await
}

#[tauri::command(rename_all = "snake_case")]
async fn enable_service(
    state: tauri::State<'_, ServiceConfigController>,
    id: String,
) -> Result<(), ServiceConfigError> {
    state.enable_service(id).await
}

#[tauri::command(rename_all = "snake_case")]
async fn disable_service(
    state: tauri::State<'_, ServiceConfigController>,
    id: String,
) -> Result<(), ServiceConfigError> {
    state.disable_service(id).await
}

#[tauri::command(rename_all = "snake_case")]
async fn delete_service_config(
    state: tauri::State<'_, ServiceConfigController>,
    id: String,
) -> Result<(), ServiceConfigError> {
    state.delete_service_config(id).await
}

// Notification Commands
#[tauri::command(rename_all = "snake_case")]
async fn create_notification(
    state: tauri::State<'_, NotificationController>,
    request_json: String,
) -> Result<NotificationResponse, ValidationError> {
    validate_command::<CreateNotificationRequest, _, _, _, ValidationError>(
        &request_json,
        |request| async move {
            state
                .create_notification(request)
                .await
                .map_err(|e| ValidationError::from_message(&e.message))
        },
    )
    .await
}

#[tauri::command(rename_all = "snake_case")]
async fn get_notification(
    state: tauri::State<'_, NotificationController>,
    id: String,
) -> Result<NotificationResponse, NotificationError> {
    state.get_notification(id).await
}

#[tauri::command(rename_all = "snake_case")]
async fn get_all_notifications(
    state: tauri::State<'_, NotificationController>,
    filter_json: Option<String>,
) -> Result<NotificationListResponse, ValidationError> {
    match filter_json {
        Some(json) => {
            validate_command::<NotificationFilterRequest, _, _, _, ValidationError>(
                &json,
                |filter| async move {
                    state
                        .get_all_notifications(Some(filter))
                        .await
                        .map_err(|e| ValidationError::from_message(&e.message))
                },
            )
            .await
        }
        None => state
            .get_all_notifications(None)
            .await
            .map_err(|e| ValidationError::from_message(&e.message)),
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn mark_as_read(
    state: tauri::State<'_, NotificationController>,
    id: String,
) -> Result<(), NotificationError> {
    state.mark_as_read(id).await
}

#[tauri::command(rename_all = "snake_case")]
async fn mark_action_required(
    state: tauri::State<'_, NotificationController>,
    id: String,
) -> Result<(), NotificationError> {
    state.mark_action_required(id).await
}

#[tauri::command(rename_all = "snake_case")]
async fn mark_action_taken(
    state: tauri::State<'_, NotificationController>,
    id: String,
) -> Result<(), NotificationError> {
    state.mark_action_taken(id).await
}

#[tauri::command(rename_all = "snake_case")]
async fn archive_notification(
    state: tauri::State<'_, NotificationController>,
    id: String,
) -> Result<(), NotificationError> {
    state.archive_notification(id).await
}

#[tauri::command(rename_all = "snake_case")]
async fn delete_notification(
    state: tauri::State<'_, NotificationController>,
    id: String,
) -> Result<(), NotificationError> {
    state.delete_notification(id).await
}

#[tauri::command(rename_all = "snake_case")]
async fn mark_all_notifications_read(
    state: tauri::State<'_, NotificationController>,
) -> Result<(), NotificationError> {
    let notifications = state.get_all_notifications(None).await?;
    for notification in notifications.notifications {
        state.mark_as_read(notification.id).await?;
    }
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
async fn archive_all_read_notifications(
    state: tauri::State<'_, NotificationController>,
) -> Result<(), NotificationError> {
    let filter = NotificationFilterRequest {
        status: Some(domain::entities::NotificationStatus::Read),
        source: None,
        priority: None,
        tags: None,
        from_date: None,
        to_date: None,
        page: None,
        per_page: None,
    };
    let notifications = state.get_all_notifications(Some(filter)).await?;
    for notification in notifications.notifications {
        state.archive_notification(notification.id).await?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let app_dir = directories::ProjectDirs::from("com", "autoresponse", "app")
        .expect("Failed to get project directories")
        .data_dir()
        .to_path_buf();

    let db_path = app_dir.join("data.db");
    std::fs::create_dir_all(&app_dir).expect("Failed to create app directory");

    // Initialize repositories
    let service_config_repository = Arc::new(
        SqliteServiceConfigRepository::new(db_path.clone())
            .expect("Failed to create service config repository"),
    ) as Arc<dyn ServiceConfigRepository>;

    let notification_repository = Arc::new(
        SqliteNotificationRepository::new(db_path)
            .expect("Failed to create notification repository"),
    ) as Arc<dyn NotificationRepository>;

    // Initialize background job manager
    let job_manager = Arc::new(BackgroundJobManager::new());

    // Initialize services
    let oauth_service = Arc::new(DefaultOAuthService::new(service_config_repository.clone()));
    let service_config_service = Arc::new(DefaultServiceConfigService::new(
        service_config_repository.clone(),
    )) as Arc<dyn ServiceConfigService>;

    // Initialize AI service
    let ai_config = AIConfig::default();
    let ai_service = Arc::new(OllamaService::new(ai_config.clone()));

    // Initialize MCP server
    let mcp_use_cases = Arc::new(MCPServerUseCases::new(
        job_manager.clone(),
        ai_service.clone(),
    ));
    let mcp_config = MCPConfig::default();
    let mcp_job_id = mcp_use_cases
        .start_mcp_server(mcp_config)
        .await
        .expect("Failed to start MCP server");
    info!("MCP server started with job ID: {}", mcp_job_id);

    // Initialize action executor
    let action_executor = Arc::new(ActionExecutor::new());

    let notification_service = Arc::new(DefaultNotificationService::new(
        notification_repository.clone(),
        job_manager.clone(),
        action_executor,
        ai_service,
    )) as Arc<dyn NotificationService>;

    // Initialize use cases
    let service_config_use_cases =
        Arc::new(ServiceConfigUseCases::new(service_config_service.clone()));
    let notification_use_cases = Arc::new(NotificationUseCases::new(notification_service.clone()));

    // Initialize controllers
    let service_config_controller = ServiceConfigController::new(service_config_service);
    let notification_controller = NotificationController::new(notification_service);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(service_config_controller)
        .manage(notification_controller)
        .manage(service_config_use_cases)
        .manage(notification_use_cases)
        .manage(mcp_use_cases)
        .manage(service_config_repository.clone() as Arc<dyn ServiceConfigRepository>)
        .manage(oauth_service as Arc<dyn OAuthService>)
        .invoke_handler(tauri::generate_handler![
            // Service Config Commands
            create_service_config,
            get_service_config,
            get_all_service_configs,
            update_auth_config,
            enable_service,
            disable_service,
            delete_service_config,
            // OAuth Commands
            delete_oauth_service_config,
            save_oauth_config,
            get_service_configs,
            start_oauth_flow,
            handle_oauth_callback,
            // Notification Commands
            create_notification,
            get_notification,
            get_all_notifications,
            mark_as_read,
            mark_action_required,
            mark_action_taken,
            archive_notification,
            delete_notification,
            mark_all_notifications_read,
            archive_all_read_notifications,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(())
}
