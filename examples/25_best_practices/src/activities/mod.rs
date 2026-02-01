//! Activity implementations demonstrating best practices.
//!
//! This module demonstrates:
//! - Idempotent activities
//! - Structured error handling
//! - Context propagation
//! - Input validation
//! - Comprehensive logging

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use crate::types::*;
use crate::utils::*;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, instrument};
use std::time::Duration;

/// Input for idempotent processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotentProcessInput {
    pub idempotency_key: IdempotencyKey,
    pub user_id: UserId,
    pub operation: String,
    pub amount: Amount,
}

/// Result of idempotent processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotentProcessResult {
    pub processed: bool,
    pub previous_result: Option<String>,
    pub processing_time_ms: u64,
}

/// Validated activity input example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedProcessInput {
    pub user_id: UserId,
    pub email: Email,
    pub amount: Amount,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Idempotent activity with deduplication
#[instrument(skip(ctx, input))]
pub async fn idempotent_process_activity(
    ctx: &ActivityContext,
    input: IdempotentProcessInput,
) -> Result<IdempotentProcessResult, ActivityError> {
    let start = std::time::Instant::now();
    
    info!(
        idempotency_key = %input.idempotency_key,
        user_id = %input.user_id,
        operation = %input.operation,
        "Starting idempotent process"
    );
    
    // In production: check if already processed using idempotency_key
    // For this example, we simulate the check
    let already_processed = false; // Would check cache/db in real impl
    
    if already_processed {
        warn!(
            idempotency_key = %input.idempotency_key,
            "Request already processed, returning cached result"
        );
        
        return Ok(IdempotentProcessResult {
            processed: false,
            previous_result: Some("cached_result".to_string()),
            processing_time_ms: 0,
        });
    }
    
    // Record heartbeat
    ctx.record_heartbeat(None);
    
    // Simulate processing
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    let duration = start.elapsed();
    
    info!(
        idempotency_key = %input.idempotency_key,
        duration_ms = %duration.as_millis(),
        "Idempotent process completed"
    );
    
    Ok(IdempotentProcessResult {
        processed: true,
        previous_result: None,
        processing_time_ms: duration.as_millis() as u64,
    })
}

/// Validated activity with strong typing
#[instrument(skip(ctx, input))]
pub async fn validated_process_activity(
    ctx: &ActivityContext,
    input: ValidatedProcessInput,
) -> Result<TypedActivityResult<String>, ActivityError> {
    info!(
        user_id = %input.user_id,
        email = %input.email.as_str(),
        amount = %input.amount.value(),
        "Starting validated process"
    );
    
    // Record heartbeat
    ctx.record_heartbeat(None);
    
    // Validate business logic
    if input.amount.value() == 0.0 {
        return Err(ActivityError::Application("Amount cannot be zero".into()));
    }
    
    // Simulate processing
    tokio::time::sleep(Duration::from_millis(30)).await;
    
    let result = format!(
        "Processed ${:.2} for user {} ({})",
        input.amount.value(),
        input.user_id.as_str(),
        input.email.as_str()
    );
    
    info!(result = %result, "Validated process completed");
    
    Ok(TypedActivityResult::new(result))
}

/// Activity with comprehensive error handling
#[instrument(skip(ctx))]
pub async fn robust_external_call_activity(
    ctx: &ActivityContext,
    request_context: RequestContext,
) -> Result<String, ActivityError> {
    info!(
        trace_id = %request_context.trace_id,
        span_id = %request_context.span_id,
        "Starting external service call"
    );
    
    ctx.record_heartbeat(None);
    
    // Simulate external service call with retry
    let result = retry_with_backoff(
        || async {
            // Simulate sometimes failing external call
            if rand::random::<f32>() < 0.3 {
                return Err::<String, String>("External service temporarily unavailable".into());
            }
            Ok("external_call_result".to_string())
        },
        3,
        Duration::from_millis(100),
        2.0,
    )
    .await;
    
    match result {
        Ok(data) => {
            info!("External service call succeeded");
            Ok(data)
        }
        Err(e) => {
            error!(error = %e, "External service call failed after retries");
            Err(ActivityError::Retryable(e.to_string()))
        }
    }
}

/// Activity demonstrating structured logging
#[instrument(skip(ctx, operation_name, metadata), fields(operation = %operation_name))]
pub async fn structured_logging_activity(
    ctx: &ActivityContext,
    operation_name: String,
    metadata: std::collections::HashMap<String, String>,
) -> Result<(), ActivityError> {
    let correlation_id = generate_correlation_id();
    
    LogEntry::new()
        .field("correlation_id", &correlation_id)
        .field("operation", &operation_name)
        .field("metadata_count", &metadata.len().to_string())
        .info("Starting structured operation");
    
    ctx.record_heartbeat(None);
    
    // Simulate work
    tokio::time::sleep(Duration::from_millis(20)).await;
    
    LogEntry::new()
        .field("correlation_id", &correlation_id)
        .field("operation", &operation_name)
        .info("Structured operation completed");
    
    Ok(())
}

/// Circuit breaker protected activity
pub async fn circuit_breaker_protected_activity(
    ctx: &ActivityContext,
    request: String,
) -> Result<String, ActivityError> {
    // In production, circuit breaker state would be shared
    let mut circuit_breaker = CircuitBreaker::new(3, 2, Duration::from_secs(30));
    
    match circuit_breaker.call(|| async {
        ctx.record_heartbeat(None);
        
        // Simulate potentially failing operation
        if request.contains("fail") {
            return Err::<String, String>("Simulated failure".into());
        }
        
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(format!("Processed: {}", request))
    }).await {
        Ok(result) => Ok(result),
        Err(CircuitError::Open) => {
            warn!("Circuit breaker is open, rejecting request");
            Err(ActivityError::Application("Circuit breaker open".into()))
        }
        Err(CircuitError::OperationFailed(e)) => {
            error!(error = %e, "Operation failed");
            Err(ActivityError::Retryable(e))
        }
    }
}
