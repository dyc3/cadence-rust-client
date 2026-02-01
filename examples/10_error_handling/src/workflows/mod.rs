//! Workflow implementations for error handling example.
//!
//! This example demonstrates error handling patterns in Cadence workflows,
//! including retry logic, error classification, and recovery strategies.

use crate::activities::*;
use cadence_core::ActivityOptions;
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use tracing::{info, warn, error};
use std::time::Duration;

/// A workflow that demonstrates error handling with retry logic
pub async fn error_handling_workflow(
    ctx: &mut WorkflowContext,
    items: Vec<ProcessInput>,
) -> Result<BatchProcessingResult, WorkflowError> {
    info!("Starting error handling workflow with {} items", items.len());
    
    let mut successful = Vec::new();
    let mut failed = Vec::new();
    let mut retried = Vec::new();
    
    for item in &items {
        info!("Processing item: {}", item.item_id);
        
        let activity_options = ActivityOptions {
            start_to_close_timeout: Duration::from_secs(30),
            schedule_to_close_timeout: Duration::from_secs(60),
            retry_policy: Some(cadence_core::RetryPolicy {
                initial_interval: Duration::from_secs(1),
                backoff_coefficient: 2.0,
                maximum_interval: Duration::from_secs(30),
                maximum_attempts: 3,
                non_retryable_error_types: vec!["NonRetryable".to_string()],
                expiration_interval: Duration::from_secs(0),
            }),
            ..Default::default()
        };
        
        match ctx
            .execute_activity(
                "unreliable_process",
                Some(serde_json::to_vec(item).unwrap()),
                activity_options,
            )
            .await
        {
            Ok(result) => {
                let process_result: ProcessResult = serde_json::from_slice(&result)
                    .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
                
                if process_result.attempt > 1 {
                    retried.push(item.item_id.clone());
                }
                
                successful.push(process_result);
                info!("Item {} processed successfully", item.item_id);
            }
            Err(e) => {
                error!("Failed to process item {}: {:?}", item.item_id, e);
                failed.push((item.item_id.clone(), format!("{:?}", e)));
            }
        }
    }
    
    info!(
        "Workflow completed: {} successful, {} failed, {} retried",
        successful.len(),
        failed.len(),
        retried.len()
    );
    
    Ok(BatchProcessingResult {
        successful,
        failed,
        retried,
        total: items.len(),
    })
}

/// Result of batch processing
#[derive(Debug, Clone)]
pub struct BatchProcessingResult {
    pub successful: Vec<ProcessResult>,
    pub failed: Vec<(String, String)>,
    pub retried: Vec<String>,
    pub total: usize,
}

/// A workflow that demonstrates fallback patterns
pub async fn fallback_workflow(
    ctx: &mut WorkflowContext,
    item: ProcessInput,
) -> Result<FallbackResult, WorkflowError> {
    info!("Starting fallback workflow for item: {}", item.item_id);
    
    // Try primary processing
    let primary_options = ActivityOptions {
        start_to_close_timeout: Duration::from_secs(10),
        retry_policy: Some(cadence_core::RetryPolicy {
            initial_interval: Duration::from_millis(100),
            backoff_coefficient: 1.5,
            maximum_interval: Duration::from_secs(5),
            maximum_attempts: 2,
            ..Default::default()
        }),
        ..Default::default()
    };
    
    match ctx
        .execute_activity(
            "unreliable_process",
            Some(serde_json::to_vec(&item).unwrap()),
            primary_options,
        )
        .await
    {
        Ok(result) => {
            let process_result: ProcessResult = serde_json::from_slice(&result)
                .map_err(|e| WorkflowError::Generic(format!("Failed to parse: {}", e)))?;
            
            Ok(FallbackResult {
                item_id: item.item_id,
                success: true,
                method: "primary".to_string(),
                attempts: process_result.attempt,
            })
        }
        Err(_) => {
            warn!("Primary processing failed, trying fallback");
            
            // Fallback: Try with different parameters
            let mut fallback_item = item.clone();
            fallback_item.should_fail = false; // Force success in fallback
            
            let fallback_options = ActivityOptions {
                start_to_close_timeout: Duration::from_secs(20),
                retry_policy: Some(cadence_core::RetryPolicy {
                    initial_interval: Duration::from_millis(500),
                    backoff_coefficient: 2.0,
                    maximum_interval: Duration::from_secs(10),
                    maximum_attempts: 5,
                    ..Default::default()
                }),
                ..Default::default()
            };
            
            match ctx
                .execute_activity(
                    "unreliable_process",
                    Some(serde_json::to_vec(&fallback_item).unwrap()),
                    fallback_options,
                )
                .await
            {
                Ok(result) => {
                    let process_result: ProcessResult = serde_json::from_slice(&result)
                        .map_err(|e| WorkflowError::Generic(format!("Failed to parse: {}", e)))?;
                    
                    Ok(FallbackResult {
                        item_id: item.item_id,
                        success: true,
                        method: "fallback".to_string(),
                        attempts: process_result.attempt + 2, // +2 for primary attempts
                    })
                }
                Err(e) => {
                    error!("Fallback also failed: {:?}", e);
                    Ok(FallbackResult {
                        item_id: item.item_id,
                        success: false,
                        method: "failed".to_string(),
                        attempts: 0,
                    })
                }
            }
        }
    }
}

/// Result with fallback information
#[derive(Debug, Clone)]
pub struct FallbackResult {
    pub item_id: String,
    pub success: bool,
    pub method: String,
    pub attempts: u32,
}

/// A workflow that demonstrates validation and early failure
pub async fn validation_workflow(
    ctx: &mut WorkflowContext,
    data: serde_json::Value,
) -> Result<ValidationWorkflowResult, WorkflowError> {
    info!("Starting validation workflow");
    
    // First, validate the data
    let validation_result = ctx
        .execute_activity(
            "validate_data",
            Some(serde_json::to_vec(&data).unwrap()),
            ActivityOptions::default(),
        )
        .await;
    
    match validation_result {
        Ok(result) => {
            let validation: ValidationResult = serde_json::from_slice(&result)
                .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation: {}", e)))?;
            
            if validation.valid {
                info!("Data validation passed, proceeding with processing");
                
                // Process the validated data
                let process_input = ProcessInput {
                    item_id: validation.normalized_data.get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    should_fail: false,
                    failure_type: None,
                };
                
                let process_result = ctx
                    .execute_activity(
                        "unreliable_process",
                        Some(serde_json::to_vec(&process_input).unwrap()),
                        ActivityOptions::default(),
                    )
                    .await?;
                
                let result: ProcessResult = serde_json::from_slice(&process_result)
                    .map_err(|e| WorkflowError::Generic(format!("Failed to parse: {}", e)))?;
                
                Ok(ValidationWorkflowResult {
                    validated: true,
                    processed: true,
                    result: Some(result),
                    error: None,
                })
            } else {
                warn!("Data validation failed: {:?}", validation.errors);
                Ok(ValidationWorkflowResult {
                    validated: false,
                    processed: false,
                    result: None,
                    error: Some(format!("Validation failed: {}", validation.errors.join(", "))),
                })
            }
        }
        Err(e) => {
            error!("Validation activity failed: {:?}", e);
            Ok(ValidationWorkflowResult {
                validated: false,
                processed: false,
                result: None,
                error: Some(format!("Validation error: {:?}", e)),
            })
        }
    }
}

/// Result from validation workflow
#[derive(Debug, Clone)]
pub struct ValidationWorkflowResult {
    pub validated: bool,
    pub processed: bool,
    pub result: Option<ProcessResult>,
    pub error: Option<String>,
}
