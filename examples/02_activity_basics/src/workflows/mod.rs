//! Workflow implementations for activity basics example.

use crate::activities::*;
use cadence_core::ActivityOptions;
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use tracing::{info, error};

/// Process order workflow that chains multiple activities
pub async fn process_order_workflow(
    ctx: &mut WorkflowContext,
    input: OrderInput,
) -> Result<OrderOutput, WorkflowError> {
    info!("Starting process_order_workflow for customer: {}", input.customer_id);

    // Step 1: Validate the order
    let validation = ctx
        .execute_activity(
            "validate_order",
            Some(serde_json::to_vec(&input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let validation_result: ValidationResult = serde_json::from_slice(&validation)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation result: {}", e)))?;

    if !validation_result.is_valid {
        error!("Order validation failed: {:?}", validation_result.errors);
        return Err(WorkflowError::Generic(format!(
            "Order validation failed: {:?}",
            validation_result.errors
        )));
    }

    // Step 2: Calculate total
    let calculation_input = (validation_result.order_id.clone(), input.items.clone());
    let calculation = ctx
        .execute_activity(
            "calculate_total",
            Some(serde_json::to_vec(&calculation_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let calculation_result: CalculationResult = serde_json::from_slice(&calculation)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse calculation result: {}", e)))?;

    // Step 3: Process payment
    let payment_input = (validation_result.order_id.clone(), calculation_result.total);
    let payment = ctx
        .execute_activity(
            "process_payment",
            Some(serde_json::to_vec(&payment_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let payment_result: PaymentResult = serde_json::from_slice(&payment)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse payment result: {}", e)))?;

    // Step 4: Send confirmation
    let confirmation_input = (validation_result.order_id.clone(), payment_result.payment_id.clone());
    let confirmation = ctx
        .execute_activity(
            "send_confirmation",
            Some(serde_json::to_vec(&confirmation_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let confirmation_result: ConfirmationResult = serde_json::from_slice(&confirmation)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse confirmation result: {}", e)))?;

    info!(
        "Order {} processed successfully. Payment: {}, Confirmation: {}",
        validation_result.order_id,
        payment_result.payment_id,
        confirmation_result.confirmation_id
    );

    Ok(OrderOutput {
        order_id: validation_result.order_id,
        payment_id: payment_result.payment_id,
        confirmation_id: confirmation_result.confirmation_id,
        total: calculation_result.total,
    })
}

/// Output from the order processing workflow
#[derive(Debug, Clone)]
pub struct OrderOutput {
    pub order_id: String,
    pub payment_id: String,
    pub confirmation_id: String,
    pub total: f64,
}
