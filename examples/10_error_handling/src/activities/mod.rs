//! Activity implementations for error handling example.
//!
//! This example demonstrates various error scenarios and how to handle them
//! in activities and workflows.

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tracing::{error, info, warn};

/// Custom error types for the application
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingError {
    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("External service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Rate limit exceeded, retry after {retry_after}s")]
    RateLimitExceeded { retry_after: u64 },

    #[error("Data not found: {0}")]
    NotFound(String),

    #[error("Processing failed after {attempts} attempts")]
    MaxRetriesExceeded { attempts: u32 },
}

/// Processing input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInput {
    pub item_id: String,
    pub should_fail: bool,
    pub failure_type: Option<FailureType>,
}

/// Types of failures to simulate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureType {
    Retryable,    // Should be retried
    NonRetryable, // Should not be retried
    RateLimited,  // Special handling needed
    Validation,   // Input validation failed
}

/// Processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    pub item_id: String,
    pub success: bool,
    pub attempt: u32,
    pub processing_time_ms: u64,
}

/// External service call input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServiceInput {
    pub endpoint: String,
    pub payload: serde_json::Value,
    pub simulate_failure: bool,
}

/// An activity that may fail with different error types
pub async fn unreliable_process_activity(
    ctx: &ActivityContext,
    input: ProcessInput,
) -> Result<ProcessResult, ActivityError> {
    let info = ctx.get_info();
    info!(
        "Processing item {} (attempt {})",
        input.item_id, info.attempt
    );

    let start = std::time::Instant::now();

    // Simulate processing time
    tokio::time::sleep(Duration::from_millis(25)).await;

    // Record heartbeat
    ctx.record_heartbeat(Some(
        &serde_json::to_vec(&serde_json::json!({
            "item_id": &input.item_id,
            "attempt": info.attempt,
            "progress": 50,
        }))
        .unwrap(),
    ));

    // Simulate failure based on configuration
    if input.should_fail {
        match input.failure_type {
            Some(FailureType::Retryable) => {
                warn!("Simulating retryable error for item {}", input.item_id);
                return Err(ActivityError::Retryable(
                    "Temporary processing error".to_string(),
                ));
            }
            Some(FailureType::NonRetryable) => {
                error!("Simulating non-retryable error for item {}", input.item_id);
                return Err(ActivityError::NonRetryable(
                    "Permanent processing error".to_string(),
                ));
            }
            Some(FailureType::RateLimited) => {
                warn!("Simulating rate limit for item {}", input.item_id);
                return Err(ActivityError::RetryableWithDelay(
                    "Rate limit exceeded".to_string(),
                    5000, // 5 seconds in milliseconds
                ));
            }
            Some(FailureType::Validation) => {
                error!("Simulating validation error for item {}", input.item_id);
                return Err(ActivityError::NonRetryable(format!(
                    "Validation failed for item {}",
                    input.item_id
                )));
            }
            None => {
                return Err(ActivityError::Retryable("Unknown error".to_string()));
            }
        }
    }

    tokio::time::sleep(Duration::from_millis(25)).await;

    let processing_time = start.elapsed().as_millis() as u64;

    info!(
        "Successfully processed item {} in {}ms",
        input.item_id, processing_time
    );

    Ok(ProcessResult {
        item_id: input.item_id,
        success: true,
        attempt: info.attempt as u32,
        processing_time_ms: processing_time,
    })
}

/// An activity that calls an external service with error handling
pub async fn external_service_call_activity(
    ctx: &ActivityContext,
    input: ExternalServiceInput,
) -> Result<serde_json::Value, ActivityError> {
    let info = ctx.get_info();
    info!(
        "Calling external service {} (attempt {})",
        input.endpoint, info.attempt
    );

    // Simulate network delay
    tokio::time::sleep(Duration::from_millis(50)).await;

    ctx.record_heartbeat(None);

    if input.simulate_failure {
        // Simulate different failure scenarios based on endpoint
        if input.endpoint.contains("unavailable") {
            return Err(ActivityError::Retryable(
                "Service temporarily unavailable".to_string(),
            ));
        } else if input.endpoint.contains("timeout") {
            return Err(ActivityError::Retryable("Request timeout".to_string()));
        } else {
            return Err(ActivityError::NonRetryable(
                "Service returned error".to_string(),
            ));
        }
    }

    // Simulate successful response
    Ok(serde_json::json!({
        "status": "success",
        "endpoint": input.endpoint,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "attempt": info.attempt,
    }))
}

/// An activity that validates input data
pub async fn validate_data_activity(
    _ctx: &ActivityContext,
    data: serde_json::Value,
) -> Result<ValidationResult, ActivityError> {
    info!("Validating data: {:?}", data);

    let mut errors = Vec::new();

    // Check for required fields
    if data.get("id").is_none() {
        errors.push("Missing required field: id".to_string());
    }

    if data.get("value").is_none() {
        errors.push("Missing required field: value".to_string());
    } else if let Some(value) = data.get("value") {
        if let Some(num) = value.as_i64() {
            if num < 0 {
                errors.push("Value must be non-negative".to_string());
            }
        }
    }

    tokio::time::sleep(Duration::from_millis(10)).await;

    if errors.is_empty() {
        Ok(ValidationResult {
            valid: true,
            errors: Vec::new(),
            normalized_data: data,
        })
    } else {
        // Validation errors are non-retryable
        Err(ActivityError::NonRetryable(format!(
            "Validation failed: {}",
            errors.join(", ")
        )))
    }
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub normalized_data: serde_json::Value,
}
