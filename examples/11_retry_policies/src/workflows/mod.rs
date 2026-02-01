//! Workflow implementations for retry policies example.

use crate::activities::{
    PaymentRequest, PaymentResult, RetryableOperationResult,
};
use cadence_core::{ActivityOptions, RetryPolicy};
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use std::time::Duration;
use tracing::{info, warn};

/// Workflow that demonstrates exponential backoff retry
pub async fn payment_with_retry_workflow(
    ctx: &mut WorkflowContext,
    request: PaymentRequest,
) -> Result<PaymentResult, WorkflowError> {
    info!("Starting payment workflow for {}", request.payment_id);
    
    // Configure retry policy with exponential backoff
    let retry_policy = RetryPolicy {
        initial_interval: Duration::from_millis(500),
        backoff_coefficient: 2.0,
        maximum_interval: Duration::from_secs(30),
        maximum_attempts: 5,
        non_retryable_error_types: vec!["Permanent".to_string()],
        expiration_interval: Duration::from_secs(0),
    };
    
    let activity_options = ActivityOptions {
        task_list: ctx.workflow_info().task_list.clone(),
        start_to_close_timeout: Duration::from_secs(10),
        schedule_to_close_timeout: Duration::from_secs(60),
        retry_policy: Some(retry_policy),
        ..Default::default()
    };
    
    // Execute payment activity with retry policy
    let result = ctx
        .execute_activity(
            "unreliable_payment_gateway",
            Some(serde_json::to_vec(&request).unwrap()),
            activity_options,
        )
        .await?;
    
    let payment_result: PaymentResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    info!("Payment workflow completed: {:?}", payment_result.status);
    
    Ok(payment_result)
}

/// Workflow that demonstrates custom retry configuration
pub async fn configurable_retry_workflow(
    ctx: &mut WorkflowContext,
    (operation_id, configured_failures): (String, usize),
) -> Result<RetryableOperationResult, WorkflowError> {
    info!(
        "Starting configurable retry workflow for {} (configured to fail {} times)",
        operation_id, configured_failures
    );
    
    // Custom retry policy with more aggressive retry
    let retry_policy = RetryPolicy {
        initial_interval: Duration::from_millis(100),
        backoff_coefficient: 1.5,
        maximum_interval: Duration::from_secs(5),
        maximum_attempts: (configured_failures + 2) as i32,
        non_retryable_error_types: vec![],
        expiration_interval: Duration::from_secs(0),
    };
    
    let activity_options = ActivityOptions {
        task_list: ctx.workflow_info().task_list.clone(),
        start_to_close_timeout: Duration::from_secs(5),
        retry_policy: Some(retry_policy),
        ..Default::default()
    };
    
    let result = ctx
        .execute_activity(
            "configurable_retry",
            Some(serde_json::to_vec(&(operation_id.clone(), configured_failures)).unwrap()),
            activity_options,
        )
        .await?;
    
    let operation_result: RetryableOperationResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    info!(
        "Configurable retry workflow completed after {} attempts",
        operation_result.attempt_count
    );
    
    Ok(operation_result)
}

/// Workflow that demonstrates non-retryable error handling
pub async fn validation_before_processing_workflow(
    ctx: &mut WorkflowContext,
    request: PaymentRequest,
) -> Result<PaymentResult, WorkflowError> {
    info!("Starting validation workflow for {}", request.payment_id);
    
    // Step 1: Validate (no retries - validation failures are permanent)
    let validation_result = ctx
        .execute_activity(
            "validate_payment",
            Some(serde_json::to_vec(&request).unwrap()),
            ActivityOptions {
                task_list: ctx.workflow_info().task_list.clone(),
                start_to_close_timeout: Duration::from_secs(5),
                retry_policy: Some(RetryPolicy {
                    maximum_attempts: 1,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await;
    
    match validation_result {
        Ok(result) => {
            let validation: RetryableOperationResult = serde_json::from_slice(&result)
                .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation: {}", e)))?;
            
            if !validation.success {
                warn!("Validation failed: {}", validation.message);
                return Err(WorkflowError::Generic(format!("Validation failed: {}", validation.message)));
            }
            
            info!("Validation passed: {}", validation.message);
        }
        Err(e) => {
            warn!("Validation error (no retries): {:?}", e);
            return Err(e);
        }
    }
    
    // Step 2: Process payment (with retries)
    let payment_result = payment_with_retry_workflow(ctx, request).await?;
    
    Ok(payment_result)
}

/// Workflow that demonstrates timeout handling
pub async fn timeout_handling_workflow(
    ctx: &mut WorkflowContext,
    duration_ms: u64,
) -> Result<RetryableOperationResult, WorkflowError> {
    info!("Starting timeout handling workflow (expected duration: {}ms)", duration_ms);
    
    // Configure retry policy for timeout scenarios
    let retry_policy = RetryPolicy {
        initial_interval: Duration::from_millis(200),
        backoff_coefficient: 1.2,
        maximum_interval: Duration::from_secs(10),
        maximum_attempts: 3,
        non_retryable_error_types: vec!["Permanent".to_string()],
        expiration_interval: Duration::from_secs(0),
    };
    
    // Activity with short timeout - may timeout and retry
    let activity_options = ActivityOptions {
        task_list: ctx.workflow_info().task_list.clone(),
        start_to_close_timeout: Duration::from_millis(duration_ms + 50),
        schedule_to_start_timeout: Duration::from_secs(5),
        retry_policy: Some(retry_policy),
        ..Default::default()
    };
    
    let result = ctx
        .execute_activity(
            "timeout_prone",
            Some(serde_json::to_vec(&duration_ms).unwrap()),
            activity_options,
        )
        .await?;
    
    let operation_result: RetryableOperationResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    info!(
        "Timeout handling workflow completed: {}",
        operation_result.message
    );
    
    Ok(operation_result)
}
