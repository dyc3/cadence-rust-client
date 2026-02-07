//! Activity implementations for workflow external example.
//!
//! This example focuses on external workflow interactions,
//! so activities support notification and logging.

use serde::{Deserialize, Serialize};
use tracing::info;
use uber_cadence_activity::ActivityContext;
use uber_cadence_worker::ActivityError;

/// Input for logging activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogActivityInput {
    pub workflow_id: String,
    pub activity_name: String,
    pub message: String,
    pub level: LogLevel,
}

/// Log level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Result of logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogResult {
    pub logged: bool,
    pub log_id: String,
}

/// Log activity execution
pub async fn log_activity_execution(
    _ctx: &ActivityContext,
    input: LogActivityInput,
) -> Result<LogResult, ActivityError> {
    let log_message = format!(
        "[{}] {} - {}: {}",
        input.workflow_id,
        input.activity_name,
        match input.level {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        },
        input.message
    );

    info!("{}", log_message);

    Ok(LogResult {
        logged: true,
        log_id: format!("log_{}", chrono::Utc::now().timestamp()),
    })
}

/// Input for notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyInput {
    pub target_workflow_id: String,
    pub notification_type: String,
    pub message: String,
}

/// Result of notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyResult {
    pub sent: bool,
    pub notification_id: String,
}

/// Send notification to external system
pub async fn send_external_notification(
    _ctx: &ActivityContext,
    input: NotifyInput,
) -> Result<NotifyResult, ActivityError> {
    info!(
        "Sending {} notification to workflow {}: {}",
        input.notification_type, input.target_workflow_id, input.message
    );

    // Simulate external notification (webhook, message queue, etc.)
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    Ok(NotifyResult {
        sent: true,
        notification_id: format!("notif_{}", chrono::Utc::now().timestamp()),
    })
}
