//! Workflow implementations for workflow signals example.
//!
//! This example demonstrates various signal handling patterns.

use crate::activities::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};
use uber_cadence_core::ActivityOptions;
use uber_cadence_workflow::context::WorkflowError;
use uber_cadence_workflow::{SignalChannel, WorkflowContext};

/// Signal to update order status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatusSignal {
    pub new_status: OrderStatus,
    pub updated_by: String,
    pub reason: Option<String>,
}

/// Order status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Pending,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
}

/// Signal to add item to order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddItemSignal {
    pub item_id: String,
    pub quantity: u32,
    pub unit_price: f64,
}

/// Signal to cancel order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderSignal {
    pub reason: String,
    pub cancelled_by: String,
}

/// Order state maintained by workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderState {
    pub order_id: String,
    pub customer_id: String,
    pub items: Vec<OrderItem>,
    pub status: OrderStatus,
    pub total: f64,
    pub created_at: String,
    pub updated_at: String,
}

/// Order item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub item_id: String,
    pub quantity: u32,
    pub unit_price: f64,
    pub subtotal: f64,
}

/// Result from order workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderWorkflowResult {
    pub order_id: String,
    pub final_status: OrderStatus,
    pub items_count: usize,
    pub total: f64,
    pub was_cancelled: bool,
}

/// Workflow that handles multiple signal types for order management
///
/// This workflow demonstrates:
/// - Creating signal channels for different signal types
/// - Handling signals in a loop
/// - Updating state based on signals
/// - Sending notifications when state changes
pub async fn order_management_workflow(
    ctx: &mut WorkflowContext,
    order_id: String,
) -> Result<OrderWorkflowResult, WorkflowError> {
    info!("Starting order management workflow for order: {}", order_id);

    // Initialize order state
    let mut state = OrderState {
        order_id: order_id.clone(),
        customer_id: "customer_123".to_string(),
        items: vec![],
        status: OrderStatus::Pending,
        total: 0.0,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    // Create signal channels for different signal types
    let mut status_channel: SignalChannel = ctx.get_signal_channel("update_status");
    let mut add_item_channel: SignalChannel = ctx.get_signal_channel("add_item");
    let mut cancel_channel: SignalChannel = ctx.get_signal_channel("cancel_order");

    let mut was_cancelled = false;
    let workflow_timeout = Duration::from_secs(60);
    let start_time = ctx.now();

    // Main loop: process signals until order is complete or cancelled
    loop {
        // Check if we've exceeded workflow timeout
        let elapsed = ctx.now() - start_time;
        if elapsed > chrono::Duration::from_std(workflow_timeout).unwrap_or_default() {
            info!("Order workflow timed out");
            break;
        }

        // Check if order is in final state
        if matches!(
            state.status,
            OrderStatus::Delivered | OrderStatus::Cancelled
        ) {
            info!("Order reached final state: {:?}", state.status);
            break;
        }

        // Try to receive signals (non-blocking with timeout)
        tokio::select! {
            // Handle status update signal
            signal_data = async { status_channel.recv().await } => {
                if let Some(data) = signal_data {
                    if let Ok(signal) = serde_json::from_slice::<UpdateStatusSignal>(&data) {
                        info!("Received status update signal: {:?}", signal);

                        let old_status = state.status.clone();
                        state.status = signal.new_status;
                        state.updated_at = chrono::Utc::now().to_rfc3339();

                        // Send notification about status change
                        let notif_input = NotificationInput {
                            recipient: state.customer_id.clone(),
                            message: format!("Order {} status changed from {:?} to {:?}",
                                state.order_id, old_status, state.status),
                            notification_type: NotificationType::Email,
                        };

                        let _ = ctx.execute_activity(
                            "send_notification",
                            Some(serde_json::to_vec(&notif_input).unwrap()),
                            ActivityOptions::default(),
                        ).await;

                        // Update status in database
                        let update_input = StatusUpdateInput {
                            entity_id: state.order_id.clone(),
                            new_status: format!("{:?}", state.status),
                            updated_by: signal.updated_by,
                        };

                        let _ = ctx.execute_activity(
                            "update_status",
                            Some(serde_json::to_vec(&update_input).unwrap()),
                            ActivityOptions::default(),
                        ).await;
                    }
                }
            }

            // Handle add item signal
            signal_data = async { add_item_channel.recv().await } => {
                if let Some(data) = signal_data {
                    if let Ok(signal) = serde_json::from_slice::<AddItemSignal>(&data) {
                        info!("Received add item signal: {:?}", signal);

                        // Only allow adding items while pending
                        if state.status == OrderStatus::Pending {
                            let item = OrderItem {
                                item_id: signal.item_id.clone(),
                                quantity: signal.quantity,
                                unit_price: signal.unit_price,
                                subtotal: signal.quantity as f64 * signal.unit_price,
                            };

                            state.items.push(item);
                            state.total = state.items.iter().map(|i| i.subtotal).sum();
                            state.updated_at = chrono::Utc::now().to_rfc3339();

                            info!("Added item {} to order, new total: ${:.2}", signal.item_id, state.total);
                        } else {
                            warn!("Cannot add items to order in {:?} state", state.status);
                        }
                    }
                }
            }

            // Handle cancel signal
            signal_data = async { cancel_channel.recv().await } => {
                if let Some(data) = signal_data {
                    if let Ok(signal) = serde_json::from_slice::<CancelOrderSignal>(&data) {
                        info!("Received cancel order signal: {:?}", signal);

                        // Can only cancel if not already delivered
                        if state.status != OrderStatus::Delivered {
                            state.status = OrderStatus::Cancelled;
                            state.updated_at = chrono::Utc::now().to_rfc3339();
                            was_cancelled = true;

                            // Send cancellation notification
                            let notif_input = NotificationInput {
                                recipient: state.customer_id.clone(),
                                message: format!("Order {} has been cancelled. Reason: {}",
                                    state.order_id, signal.reason),
                                notification_type: NotificationType::Email,
                            };

                            let _ = ctx.execute_activity(
                                "send_notification",
                                Some(serde_json::to_vec(&notif_input).unwrap()),
                                ActivityOptions::default(),
                            ).await;

                            // Update status
                            let update_input = StatusUpdateInput {
                                entity_id: state.order_id.clone(),
                                new_status: "Cancelled".to_string(),
                                updated_by: signal.cancelled_by,
                            };

                            let _ = ctx.execute_activity(
                                "update_status",
                                Some(serde_json::to_vec(&update_input).unwrap()),
                                ActivityOptions::default(),
                            ).await;

                            info!("Order {} cancelled", state.order_id);
                            break;
                        } else {
                            warn!("Cannot cancel already delivered order");
                        }
                    }
                }
            }

            // Timeout between checks
            _ = ctx.sleep(Duration::from_millis(100)) => {
                // Continue loop
            }
        }
    }

    info!(
        "Order management workflow completed for order: {}",
        order_id
    );

    Ok(OrderWorkflowResult {
        order_id: state.order_id,
        final_status: state.status,
        items_count: state.items.len(),
        total: state.total,
        was_cancelled,
    })
}

/// Signal for approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalSignal {
    pub approver: String,
    pub approved: bool,
    pub comments: Option<String>,
}

/// Result from approval workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResult {
    pub request_id: String,
    pub approved: bool,
    pub approver: String,
    pub comments: Option<String>,
}

/// Workflow that waits for an approval signal
///
/// This demonstrates a simple signal-receive pattern for human-in-the-loop workflows.
pub async fn approval_workflow(
    ctx: &mut WorkflowContext,
    request_id: String,
) -> Result<ApprovalResult, WorkflowError> {
    info!("Starting approval workflow for request: {}", request_id);

    // Create signal channel for approval
    let mut approval_channel: SignalChannel = ctx.get_signal_channel("approval");

    // Wait for approval signal (blocking)
    info!("Waiting for approval signal...");

    let signal_data = approval_channel.recv().await;

    if let Some(data) = signal_data {
        if let Ok(signal) = serde_json::from_slice::<ApprovalSignal>(&data) {
            info!(
                "Received approval from {}: approved={}",
                signal.approver, signal.approved
            );

            // Send notification about approval decision
            let notif_input = NotificationInput {
                recipient: "requester@example.com".to_string(),
                message: format!(
                    "Request {} has been {} by {}",
                    request_id,
                    if signal.approved {
                        "approved"
                    } else {
                        "rejected"
                    },
                    signal.approver
                ),
                notification_type: NotificationType::Email,
            };

            let _ = ctx
                .execute_activity(
                    "send_notification",
                    Some(serde_json::to_vec(&notif_input).unwrap()),
                    ActivityOptions::default(),
                )
                .await;

            return Ok(ApprovalResult {
                request_id,
                approved: signal.approved,
                approver: signal.approver,
                comments: signal.comments,
            });
        }
    }

    Err(WorkflowError::Generic(
        "No approval signal received".to_string(),
    ))
}

/// Signal for configuration update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateSignal {
    pub config_key: String,
    pub config_value: String,
}

/// Result from config workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigWorkflowResult {
    pub config_count: usize,
    pub configs: Vec<(String, String)>,
}

/// Workflow that accumulates configuration updates via signals
///
/// This demonstrates collecting multiple signals and aggregating results.
pub async fn config_update_workflow(
    ctx: &mut WorkflowContext,
    max_updates: usize,
) -> Result<ConfigWorkflowResult, WorkflowError> {
    info!(
        "Starting config update workflow (max {} updates)",
        max_updates
    );

    let mut config_channel: SignalChannel = ctx.get_signal_channel("config_update");
    let mut configs: Vec<(String, String)> = vec![];

    // Collect up to max_updates config signals
    while configs.len() < max_updates {
        match config_channel.recv().await {
            Some(data) => {
                if let Ok(signal) = serde_json::from_slice::<ConfigUpdateSignal>(&data) {
                    info!(
                        "Received config update: {} = {}",
                        signal.config_key, signal.config_value
                    );
                    configs.push((signal.config_key, signal.config_value));
                }
            }
            None => {
                // No more signals or timeout
                break;
            }
        }

        // Small delay to not busy-wait
        ctx.sleep(Duration::from_millis(10)).await;
    }

    info!("Config workflow completed with {} configs", configs.len());

    Ok(ConfigWorkflowResult {
        config_count: configs.len(),
        configs,
    })
}
