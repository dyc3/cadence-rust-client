//! Workflow implementations for advanced patterns.

use crate::activities::*;
use cadence_core::ActivityOptions;
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use std::time::Duration;
use tracing::{error, info, warn};

/// Saga step with compensation
#[allow(dead_code)]
struct SagaStep<T, C> {
    name: String,
    action: T,
    compensate: C,
}

impl<T, C> SagaStep<T, C> {
    #[allow(dead_code)]
    fn new(name: &str, action: T, compensate: C) -> Self {
        Self {
            name: name.to_string(),
            action,
            compensate,
        }
    }
}

/// Order saga workflow demonstrating distributed transaction pattern
pub async fn order_saga_workflow(
    ctx: &mut WorkflowContext,
    order_input: OrderSagaInput,
) -> Result<OrderSagaResult, WorkflowError> {
    info!("Starting order saga workflow for order: {}", order_input.order_id);

    let workflow_info = ctx.workflow_info();
    let activity_options = ActivityOptions {
        task_list: workflow_info.task_list.clone(),
        start_to_close_timeout: Duration::from_secs(30),
        ..Default::default()
    };

    // Track completed steps for compensation
    let mut completed_steps: Vec<String> = Vec::new();

    // Step 1: Process Payment
    let payment_input = PaymentInput {
        order_id: order_input.order_id.clone(),
        amount: order_input.total_amount,
        currency: order_input.currency.clone(),
        customer_id: order_input.customer_id.clone(),
    };

    let payment_result = match ctx
        .execute_activity(
            "process_payment",
            Some(serde_json::to_vec(&payment_input).unwrap()),
            activity_options.clone(),
        )
        .await
    {
        Ok(result) => {
            let payment: PaymentResult = serde_json::from_slice(&result)
                .map_err(|e| WorkflowError::Generic(format!("Failed to parse payment result: {}", e)))?;
            completed_steps.push("payment".to_string());
            payment
        }
        Err(e) => {
            error!("Payment processing failed: {}", e);
            return Err(WorkflowError::ActivityFailed(format!("Payment failed: {}", e)));
        }
    };

    // Step 2: Reserve Inventory
    let inventory_input = InventoryInput {
        order_id: order_input.order_id.clone(),
        items: order_input.items.clone(),
    };

    let inventory_result = match ctx
        .execute_activity(
            "reserve_inventory",
            Some(serde_json::to_vec(&inventory_input).unwrap()),
            activity_options.clone(),
        )
        .await
    {
        Ok(result) => {
            let inventory: InventoryResult = serde_json::from_slice(&result)
                .map_err(|e| WorkflowError::Generic(format!("Failed to parse inventory result: {}", e)))?;
            completed_steps.push("inventory".to_string());
            inventory
        }
        Err(e) => {
            error!("Inventory reservation failed: {}", e);
            // Compensate: Refund payment
            compensate_payment(ctx, &payment_input, &activity_options).await?;
            return Err(WorkflowError::ActivityFailed(format!("Inventory reservation failed: {}", e)));
        }
    };

    // Step 3: Create Shipment
    let shipping_items: Vec<ShippingItem> = order_input
        .items
        .iter()
        .map(|item| ShippingItem {
            product_id: item.product_id.clone(),
            quantity: item.quantity,
        })
        .collect();

    let shipping_input = ShippingInput {
        order_id: order_input.order_id.clone(),
        items: shipping_items,
        address: order_input.shipping_address.clone(),
    };

    let shipping_result = match ctx
        .execute_activity(
            "create_shipment",
            Some(serde_json::to_vec(&shipping_input).unwrap()),
            activity_options.clone(),
        )
        .await
    {
        Ok(result) => {
            let shipping: ShippingResult = serde_json::from_slice(&result)
                .map_err(|e| WorkflowError::Generic(format!("Failed to parse shipping result: {}", e)))?;
            completed_steps.push("shipping".to_string());
            shipping
        }
        Err(e) => {
            error!("Shipment creation failed: {}", e);
            // Compensate: Release inventory and refund payment
            compensate_inventory(ctx, &inventory_result.reservation_id, &order_input.order_id, &activity_options).await?;
            compensate_payment(ctx, &payment_input, &activity_options).await?;
            return Err(WorkflowError::ActivityFailed(format!("Shipment creation failed: {}", e)));
        }
    };

    // Step 4: Send confirmation notification
    let notification_input = NotificationInput {
        recipient_id: order_input.customer_id.clone(),
        notification_type: NotificationType::Email,
        message: format!(
            "Order {} confirmed! Tracking: {}",
            order_input.order_id, shipping_result.tracking_number
        ),
        metadata: serde_json::json!({
            "order_id": order_input.order_id,
            "tracking_number": shipping_result.tracking_number,
        }),
    };

    if let Err(e) = ctx
        .execute_activity(
            "send_notification",
            Some(serde_json::to_vec(&notification_input).unwrap()),
            activity_options,
        )
        .await
    {
        warn!("Failed to send notification: {}. Continuing anyway.", e);
        // Notifications are not critical - don't fail the workflow
    }

    info!("Order saga workflow completed successfully");

    Ok(OrderSagaResult {
        order_id: order_input.order_id,
        status: OrderStatus::Confirmed,
        payment_transaction_id: payment_result.transaction_id,
        reservation_id: inventory_result.reservation_id,
        shipment_id: shipping_result.shipment_id,
        tracking_number: shipping_result.tracking_number,
    })
}

/// Compensate payment (refund)
async fn compensate_payment(
    ctx: &WorkflowContext,
    payment_input: &PaymentInput,
    options: &ActivityOptions,
) -> Result<(), WorkflowError> {
    warn!("Compensating payment for order: {}", payment_input.order_id);
    
    ctx.execute_activity(
        "refund_payment",
        Some(serde_json::to_vec(payment_input).unwrap()),
        options.clone(),
    )
    .await
    .map_err(|e| WorkflowError::Generic(format!("Compensation failed: {}", e)))?;
    
    Ok(())
}

/// Compensate inventory (release reservation)
async fn compensate_inventory(
    ctx: &WorkflowContext,
    reservation_id: &str,
    order_id: &str,
    options: &ActivityOptions,
) -> Result<(), WorkflowError> {
    warn!("Compensating inventory for order: {}", order_id);
    
    let compensation_data = serde_json::json!({
        "reservation_id": reservation_id,
        "order_id": order_id,
    });
    
    ctx.execute_activity(
        "release_inventory",
        Some(serde_json::to_vec(&compensation_data).unwrap()),
        options.clone(),
    )
    .await
    .map_err(|e| WorkflowError::Generic(format!("Inventory compensation failed: {}", e)))?;
    
    Ok(())
}

/// Order saga input
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderSagaInput {
    pub order_id: String,
    pub customer_id: String,
    pub items: Vec<InventoryItem>,
    pub total_amount: f64,
    pub currency: String,
    pub shipping_address: Address,
}

/// Order saga result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderSagaResult {
    pub order_id: String,
    pub status: OrderStatus,
    pub payment_transaction_id: String,
    pub reservation_id: String,
    pub shipment_id: String,
    pub tracking_number: String,
}

/// Order status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum OrderStatus {
    Pending,
    Confirmed,
    Failed,
    Cancelled,
}

/// Fan-out/fan-in workflow for parallel data processing
pub async fn parallel_processing_workflow(
    ctx: &mut WorkflowContext,
    batch_input: DataProcessingInput,
) -> Result<BatchProcessingResult, WorkflowError> {
    info!(
        "Starting parallel processing workflow for batch: {} with {} records",
        batch_input.batch_id,
        batch_input.records.len()
    );

    let workflow_info = ctx.workflow_info();
    let activity_options = ActivityOptions {
        task_list: workflow_info.task_list.clone(),
        start_to_close_timeout: Duration::from_secs(60),
        ..Default::default()
    };

    // Process records sequentially
    // Note: In a real implementation with true parallel processing,
    // this would use the Cadence Promise/Selector pattern
    let mut successful_count = 0;
    let mut failed_count = 0;
    let mut total_processing_time = 0u64;

    // Sequential processing for demonstration
    for record in &batch_input.records {
        match ctx
            .execute_activity(
                "process_record",
                Some(serde_json::to_vec(record).unwrap()),
                activity_options.clone(),
            )
            .await
        {
            Ok(result_data) => {
                if let Ok(processed) = serde_json::from_slice::<ProcessedRecord>(&result_data) {
                    successful_count += 1;
                    total_processing_time += processed.processing_time_ms;
                }
            }
            Err(_) => {
                failed_count += 1;
            }
        }
    }

    info!(
        "Parallel processing completed. Success: {}, Failed: {}",
        successful_count, failed_count
    );

    Ok(BatchProcessingResult {
        batch_id: batch_input.batch_id,
        total_records: batch_input.records.len() as u64,
        successful_count,
        failed_count,
        average_processing_time_ms: if successful_count > 0 {
            total_processing_time / successful_count
        } else {
            0
        },
    })
}

/// Batch processing result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchProcessingResult {
    pub batch_id: String,
    pub total_records: u64,
    pub successful_count: u64,
    pub failed_count: u64,
    pub average_processing_time_ms: u64,
}

/// Circuit breaker workflow pattern
pub async fn circuit_breaker_workflow(
    ctx: &mut WorkflowContext,
    operation: CircuitBreakerOperation,
) -> Result<CircuitBreakerResult, WorkflowError> {
    info!("Executing circuit breaker workflow for operation: {}", operation.operation_type);

    // Check circuit state from previous runs using mutable side effect
    let failure_count: u32 = ctx
        .mutable_side_effect("circuit_failure_count", || 0u32)
        .await;

    let circuit_open = failure_count >= 5; // Circuit opens after 5 failures

    if circuit_open {
        warn!("Circuit breaker is OPEN - failing fast");
        return Ok(CircuitBreakerResult {
            success: false,
            result: None,
            fallback_triggered: true,
            error: Some("Circuit breaker open".to_string()),
        });
    }

    let workflow_info = ctx.workflow_info();
    let activity_options = ActivityOptions {
        task_list: workflow_info.task_list.clone(),
        start_to_close_timeout: Duration::from_secs(10),
        ..Default::default()
    };

    // Try to execute the operation
    match ctx
        .execute_activity(
            &operation.operation_type,
            operation.input.clone(),
            activity_options,
        )
        .await
    {
        Ok(result) => {
            // Success - reset failure count
            info!("Operation succeeded - resetting circuit");
            Ok(CircuitBreakerResult {
                success: true,
                result: Some(result),
                fallback_triggered: false,
                error: None,
            })
        }
        Err(e) => {
            // Failure - increment failure count
            error!("Operation failed: {}", e);
            let new_failure_count = failure_count + 1;
            
            Ok(CircuitBreakerResult {
                success: false,
                result: None,
                fallback_triggered: new_failure_count >= 5,
                error: Some(format!("Operation failed: {}", e)),
            })
        }
    }
}

/// Circuit breaker operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CircuitBreakerOperation {
    pub operation_type: String,
    pub input: Option<Vec<u8>>,
    pub fallback: Option<String>,
}

/// Circuit breaker result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CircuitBreakerResult {
    pub success: bool,
    pub result: Option<Vec<u8>>,
    pub fallback_triggered: bool,
    pub error: Option<String>,
}
