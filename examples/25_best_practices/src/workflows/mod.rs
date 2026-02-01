//! Workflow implementations demonstrating best practices.
//!
//! This module demonstrates:
//! - Type-safe workflow orchestration
//! - Saga pattern implementation
//! - Idempotency handling
//! - Context propagation through workflow
//! - Version-aware workflows

use cadence_core::ActivityOptions;
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use crate::activities::*;
use crate::types::*;
use crate::utils::*;
use tracing::{info, warn, error, instrument};
use std::time::Duration;
use std::collections::HashMap;

/// Type-safe idempotent workflow
#[instrument(skip(ctx, input))]
pub async fn idempotent_workflow(
    ctx: &mut WorkflowContext,
    input: IdempotentProcessInput,
) -> Result<IdempotentProcessResult, WorkflowError> {
    info!(
        workflow_id = %ctx.workflow_info().workflow_execution.workflow_id,
        idempotency_key = %input.idempotency_key,
        "Starting idempotent workflow"
    );
    
    let result = ctx
        .execute_activity(
            "idempotent_process",
            Some(serde_json::to_vec(&input).unwrap()),
            ActivityOptions {
                start_to_close_timeout: Duration::from_secs(30),
                retry_policy: Some(cadence_core::RetryPolicy {
                    initial_interval: Duration::from_secs(1),
                    backoff_coefficient: 2.0,
                    maximum_interval: Duration::from_secs(30),
                    maximum_attempts: 3,
                    non_retryable_error_types: vec![],
                    expiration_interval: Duration::from_secs(300),
                }),
                ..Default::default()
            },
        )
        .await?;
    
    let process_result: IdempotentProcessResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    if process_result.processed {
        info!("Workflow processed new request");
    } else {
        warn!("Workflow returned cached result for duplicate request");
    }
    
    Ok(process_result)
}

/// Validated data processing workflow
#[instrument(skip(ctx, user_id, email, amount))]
pub async fn validated_processing_workflow(
    ctx: &mut WorkflowContext,
    user_id: UserId,
    email: Email,
    amount: Amount,
) -> Result<TypedActivityResult<String>, WorkflowError> {
    info!(
        user_id = %user_id,
        email = %email.as_str(),
        amount = %amount.value(),
        "Starting validated processing workflow"
    );
    
    let input = ValidatedProcessInput {
        user_id,
        email,
        amount,
        metadata: HashMap::new(),
    };
    
    let result = ctx
        .execute_activity(
            "validated_process",
            Some(serde_json::to_vec(&input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let process_result: TypedActivityResult<String> = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    info!(
        idempotency_key = %process_result.idempotency_key,
        "Validated processing workflow completed"
    );
    
    Ok(process_result)
}

/// Saga pattern workflow with compensation
#[instrument(skip(ctx, user_id, amount))]
pub async fn saga_pattern_workflow(
    ctx: &mut WorkflowContext,
    user_id: UserId,
    amount: Amount,
) -> Result<String, WorkflowError> {
    info!(
        user_id = %user_id,
        amount = %amount.value(),
        "Starting saga pattern workflow"
    );
    
    let mut compensation_actions: Vec<Box<dyn FnOnce() -> Result<(), WorkflowError>>> = Vec::new();
    
    // Step 1: Validate input
    let validated_input = ValidatedProcessInput {
        user_id: user_id.clone(),
        email: Email::new("user@example.com").unwrap(),
        amount,
        metadata: HashMap::new(),
    };
    
    let step1_result = ctx
        .execute_activity(
            "validated_process",
            Some(serde_json::to_vec(&validated_input).unwrap()),
            ActivityOptions::default(),
        )
        .await;
    
    match step1_result {
        Ok(result) => {
            let _: TypedActivityResult<String> = serde_json::from_slice(&result)
                .map_err(|e| WorkflowError::Generic(format!("Parse error: {}", e)))?;
            
            info!("Step 1 completed successfully");
            
            // Add compensation action (in real implementation, this would undo step 1)
            compensation_actions.push(Box::new(|| {
                info!("Compensating step 1");
                Ok(())
            }));
        }
        Err(e) => {
            error!(error = ?e, "Step 1 failed");
            return Err(e);
        }
    }
    
    // Step 2: Process with idempotency
    let idempotent_input = IdempotentProcessInput {
        idempotency_key: IdempotencyKey::generate(),
        user_id: user_id.clone(),
        operation: "saga_step_2".to_string(),
        amount,
    };
    
    let step2_result = ctx
        .execute_activity(
            "idempotent_process",
            Some(serde_json::to_vec(&idempotent_input).unwrap()),
            ActivityOptions::default(),
        )
        .await;
    
    match step2_result {
        Ok(result) => {
            let _: IdempotentProcessResult = serde_json::from_slice(&result)
                .map_err(|e| WorkflowError::Generic(format!("Parse error: {}", e)))?;
            
            info!("Step 2 completed successfully");
        }
        Err(e) => {
            error!(error = ?e, "Step 2 failed, running compensation");
            
            // Run compensation actions in reverse order
            for action in compensation_actions.into_iter().rev() {
                if let Err(comp_err) = action() {
                    error!(error = ?comp_err, "Compensation action failed");
                }
            }
            
            return Err(e);
        }
    }
    
    info!("Saga pattern workflow completed successfully");
    Ok("saga_completed".to_string())
}

/// Context propagation workflow
#[instrument(skip(ctx, request_context, operation))]
pub async fn context_propagation_workflow(
    ctx: &mut WorkflowContext,
    request_context: RequestContext,
    operation: String,
) -> Result<String, WorkflowError> {
    info!(
        trace_id = %request_context.trace_id,
        span_id = %request_context.span_id,
        operation = %operation,
        "Starting context propagation workflow"
    );
    
    // Pass context through to activity
    let result = ctx
        .execute_activity(
            "robust_external_call",
            Some(serde_json::to_vec(&request_context).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let external_result: String = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    info!(
        trace_id = %request_context.trace_id,
        "Context propagation workflow completed"
    );
    
    Ok(external_result)
}

/// Structured logging workflow
#[instrument(skip(ctx, operation_name))]
pub async fn structured_logging_workflow(
    ctx: &mut WorkflowContext,
    operation_name: String,
) -> Result<(), WorkflowError> {
    let correlation_id = generate_correlation_id();
    
    info!(
        correlation_id = %correlation_id,
        operation = %operation_name,
        "Starting structured logging workflow"
    );
    
    let mut metadata = HashMap::new();
    metadata.insert("correlation_id".to_string(), correlation_id.clone());
    metadata.insert("workflow_id".to_string(), ctx.workflow_info().workflow_execution.workflow_id.clone());
    
    let _ = ctx
        .execute_activity(
            "structured_logging",
            Some(serde_json::to_vec(&(operation_name.clone(), metadata)).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    info!(
        correlation_id = %correlation_id,
        operation = %operation_name,
        "Structured logging workflow completed"
    );
    
    Ok(())
}

/// Version-aware workflow
#[instrument(skip(ctx, input))]
pub async fn version_aware_workflow(
    ctx: &mut WorkflowContext,
    input: String,
) -> Result<String, WorkflowError> {
    let workflow_version = WorkflowVersion::new(1, 0, 0);
    
    info!(
        version = %workflow_version.as_string(),
        "Starting version-aware workflow"
    );
    
    // In a real implementation, version would be stored and checked
    // to handle workflow updates gracefully
    
    let result = format!("Processed with version {}: {}", workflow_version.as_string(), input);
    
    info!(
        version = %workflow_version.as_string(),
        "Version-aware workflow completed"
    );
    
    Ok(result)
}
