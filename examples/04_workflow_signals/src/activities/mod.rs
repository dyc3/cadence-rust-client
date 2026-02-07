//! Activity implementations for workflow signals example.
//!
//! This example focuses on signals, so activities are minimal helpers.

use serde::{Deserialize, Serialize};
use tracing::info;
use uber_cadence_activity::ActivityContext;
use uber_cadence_worker::ActivityError;

/// Input for notification activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationInput {
    pub recipient: String,
    pub message: String,
    pub notification_type: NotificationType,
}

/// Type of notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Email,
    Sms,
    Push,
}

/// Result of notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResult {
    pub sent: bool,
    pub notification_id: String,
}

/// Send notification to user
pub async fn send_notification_activity(
    _ctx: &ActivityContext,
    input: NotificationInput,
) -> Result<NotificationResult, ActivityError> {
    info!(
        "Sending {:?} notification to {}: {}",
        input.notification_type, input.recipient, input.message
    );

    // Simulate sending notification
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let notification_id = format!(
        "notif_{}_{}",
        input.recipient.replace("@", "_"),
        chrono::Utc::now().timestamp()
    );

    Ok(NotificationResult {
        sent: true,
        notification_id,
    })
}

/// Input for status update activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdateInput {
    pub entity_id: String,
    pub new_status: String,
    pub updated_by: String,
}

/// Result of status update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdateResult {
    pub success: bool,
    pub previous_status: String,
}

/// Update entity status in database
pub async fn update_status_activity(
    _ctx: &ActivityContext,
    input: StatusUpdateInput,
) -> Result<StatusUpdateResult, ActivityError> {
    info!(
        "Updating status for {} to '{}' (by {})",
        input.entity_id, input.new_status, input.updated_by
    );

    // Simulate database update
    tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

    Ok(StatusUpdateResult {
        success: true,
        previous_status: "pending".to_string(),
    })
}
