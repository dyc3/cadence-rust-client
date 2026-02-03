//! Workflow implementations for testing example.

use crate::activities::*;
use cadence_core::ActivityOptions;
use cadence_workflow::context::WorkflowError;
use cadence_workflow::WorkflowContext;
use std::time::Duration;
use tracing::{info, warn};

/// Order processing workflow input
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderWorkflowInput {
    pub order_id: String,
    pub customer_id: String,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
    pub currency: String,
}

/// Order item
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub quantity: i32,
    pub unit_price: f64,
}

/// Order processing workflow output
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderWorkflowOutput {
    pub order_id: String,
    pub status: OrderStatus,
    pub transaction_id: String,
    pub processed_at: i64,
}

/// Order status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum OrderStatus {
    Pending,
    PaymentProcessing,
    InventoryChecking,
    Confirmed,
    Failed,
    Cancelled,
}

/// Order processing workflow - demonstrates basic workflow testing
pub async fn order_processing_workflow(
    ctx: &mut WorkflowContext,
    input: OrderWorkflowInput,
) -> Result<OrderWorkflowOutput, WorkflowError> {
    info!("Starting order processing workflow for {}", input.order_id);

    let workflow_info = ctx.workflow_info();
    let activity_options = ActivityOptions {
        task_list: workflow_info.task_list.clone(),
        start_to_close_timeout: Duration::from_secs(30),
        ..Default::default()
    };

    // Step 1: Process payment
    let payment_input = ProcessPaymentInput {
        order_id: input.order_id.clone(),
        amount: input.total_amount,
        currency: input.currency.clone(),
    };

    let payment_result = ctx
        .execute_activity(
            "process_payment",
            Some(serde_json::to_vec(&payment_input).unwrap()),
            activity_options.clone(),
        )
        .await?;

    let payment_output: ProcessPaymentOutput = serde_json::from_slice(&payment_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse payment result: {}", e)))?;

    if payment_output.status != PaymentStatus::Success {
        return Ok(OrderWorkflowOutput {
            order_id: input.order_id,
            status: OrderStatus::Failed,
            transaction_id: payment_output.transaction_id,
            processed_at: chrono::Utc::now().timestamp(),
        });
    }

    // Step 2: Check inventory for all items
    let mut all_available = true;
    for item in &input.items {
        let inventory_input = CheckInventoryInput {
            product_id: item.product_id.clone(),
            quantity: item.quantity,
        };

        let inventory_result = ctx
            .execute_activity(
                "check_inventory",
                Some(serde_json::to_vec(&inventory_input).unwrap()),
                activity_options.clone(),
            )
            .await?;

        let inventory_output: CheckInventoryOutput = serde_json::from_slice(&inventory_result)
            .map_err(|e| {
                WorkflowError::Generic(format!("Failed to parse inventory result: {}", e))
            })?;

        if !inventory_output.available {
            all_available = false;
            warn!("Product {} not available", item.product_id);
            break;
        }
    }

    if !all_available {
        return Ok(OrderWorkflowOutput {
            order_id: input.order_id,
            status: OrderStatus::Failed,
            transaction_id: payment_output.transaction_id,
            processed_at: chrono::Utc::now().timestamp(),
        });
    }

    // Step 3: Send confirmation email
    let email_input = SendEmailInput {
        recipient: format!("customer_{}@example.com", input.customer_id),
        subject: format!("Order {} Confirmed", input.order_id),
        body: format!(
            "Your order {} has been confirmed. Transaction ID: {}",
            input.order_id, payment_output.transaction_id
        ),
    };

    if let Err(e) = ctx
        .execute_activity(
            "send_email",
            Some(serde_json::to_vec(&email_input).unwrap()),
            activity_options,
        )
        .await
    {
        warn!("Failed to send confirmation email: {}", e);
        // Don't fail the workflow for email failures
    }

    Ok(OrderWorkflowOutput {
        order_id: input.order_id,
        status: OrderStatus::Confirmed,
        transaction_id: payment_output.transaction_id,
        processed_at: chrono::Utc::now().timestamp(),
    })
}

/// Workflow with timer - demonstrates time testing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimerWorkflowInput {
    pub task_id: String,
    pub delay_seconds: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimerWorkflowOutput {
    pub task_id: String,
    pub completed_at: i64,
}

pub async fn timer_workflow(
    ctx: &mut WorkflowContext,
    input: TimerWorkflowInput,
) -> Result<TimerWorkflowOutput, WorkflowError> {
    info!("Starting timer workflow for task {}", input.task_id);

    // Sleep for the specified duration
    ctx.sleep(Duration::from_secs(input.delay_seconds)).await;

    Ok(TimerWorkflowOutput {
        task_id: input.task_id,
        completed_at: chrono::Utc::now().timestamp(),
    })
}

/// Workflow with signal - demonstrates signal testing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignalWorkflowInput {
    pub workflow_id: String,
    pub default_value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignalWorkflowOutput {
    pub workflow_id: String,
    pub received_value: String,
    pub signal_received: bool,
}

pub async fn signal_handling_workflow(
    ctx: &mut WorkflowContext,
    input: SignalWorkflowInput,
) -> Result<SignalWorkflowOutput, WorkflowError> {
    info!("Starting signal handling workflow {}", input.workflow_id);

    // Set up signal handler
    let mut signal_channel = ctx.get_signal_channel("update_value");

    // Wait for signal with timeout
    let received_value = tokio::select! {
        signal_data = signal_channel.recv() => {
            if let Some(data) = signal_data {
                serde_json::from_slice(&data).unwrap_or(input.default_value.clone())
            } else {
                input.default_value.clone()
            }
        }
        _ = ctx.sleep(Duration::from_secs(30)) => {
            warn!("No signal received within 30 seconds");
            input.default_value.clone()
        }
    };

    let signal_received = received_value != input.default_value;

    Ok(SignalWorkflowOutput {
        workflow_id: input.workflow_id,
        received_value,
        signal_received,
    })
}

/// Workflow with side effects - demonstrates non-deterministic testing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SideEffectWorkflowInput {
    pub request_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SideEffectWorkflowOutput {
    pub request_id: String,
    pub generated_id: String,
    pub timestamp: i64,
}

pub async fn side_effect_workflow(
    ctx: &mut WorkflowContext,
    input: SideEffectWorkflowInput,
) -> Result<SideEffectWorkflowOutput, WorkflowError> {
    info!("Starting side effect workflow for {}", input.request_id);

    // Generate UUID using side effect (non-deterministic operation)
    let generated_id = ctx.side_effect(|| uuid::Uuid::new_v4().to_string()).await;

    // Get current time using side effect
    let timestamp = ctx.side_effect(|| chrono::Utc::now().timestamp()).await;

    Ok(SideEffectWorkflowOutput {
        request_id: input.request_id,
        generated_id,
        timestamp,
    })
}

/// Workflow with child workflow - demonstrates child workflow testing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParentWorkflowInput {
    pub parent_id: String,
    pub child_count: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParentWorkflowOutput {
    pub parent_id: String,
    pub child_results: Vec<ChildWorkflowOutput>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChildWorkflowInput {
    pub child_id: String,
    pub parent_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChildWorkflowOutput {
    pub child_id: String,
    pub processed: bool,
}

pub async fn parent_workflow(
    ctx: &mut WorkflowContext,
    input: ParentWorkflowInput,
) -> Result<ParentWorkflowOutput, WorkflowError> {
    info!(
        "Starting parent workflow {} with {} children",
        input.parent_id, input.child_count
    );

    let _workflow_info = ctx.workflow_info();
    let child_options = cadence_core::ChildWorkflowOptions {
        workflow_id: format!("{}-child-{}", input.parent_id, uuid::Uuid::new_v4()),
        execution_start_to_close_timeout: Duration::from_secs(60),
        ..Default::default()
    };

    let mut child_results = Vec::new();

    for i in 0..input.child_count {
        let child_input = ChildWorkflowInput {
            child_id: format!("{}-{}", input.parent_id, i),
            parent_id: input.parent_id.clone(),
        };

        let child_result = ctx
            .execute_child_workflow(
                "child_workflow",
                Some(serde_json::to_vec(&child_input).unwrap()),
                child_options.clone(),
            )
            .await?;

        let child_output: ChildWorkflowOutput = serde_json::from_slice(&child_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse child result: {}", e)))?;

        child_results.push(child_output);
    }

    Ok(ParentWorkflowOutput {
        parent_id: input.parent_id,
        child_results,
    })
}

pub async fn child_workflow(
    _ctx: &mut WorkflowContext,
    input: ChildWorkflowInput,
) -> Result<ChildWorkflowOutput, WorkflowError> {
    info!("Child workflow {} processing", input.child_id);

    // Simulate some work
    tokio::time::sleep(Duration::from_millis(10)).await;

    Ok(ChildWorkflowOutput {
        child_id: input.child_id,
        processed: true,
    })
}
