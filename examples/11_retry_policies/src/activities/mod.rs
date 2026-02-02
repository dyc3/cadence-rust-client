//! Activity implementations for retry policies example.
//!
//! This example demonstrates activities with various retry behaviors:
//! - Activities that fail intermittently (should be retried)
//! - Activities with custom retry policies
//! - Non-retryable error handling

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tracing::{info, warn};

/// Request to process a payment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub payment_id: String,
    pub amount: f64,
    pub currency: String,
    pub customer_id: String,
}

/// Result of payment processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResult {
    pub payment_id: String,
    pub status: PaymentStatus,
    pub transaction_id: String,
    pub processed_at: i64,
}

/// Payment processing status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentStatus {
    Success,
    Failed,
    Pending,
}

/// Result from a potentially failing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryableOperationResult {
    pub success: bool,
    pub attempt_count: usize,
    pub message: String,
}

/// Simulates an external service call counter
static SERVICE_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Activity that simulates intermittent failures
/// This demonstrates how retry policies help recover from transient failures
pub async fn unreliable_payment_gateway_activity(
    ctx: &ActivityContext,
    request: PaymentRequest,
) -> Result<PaymentResult, ActivityError> {
    let info = ctx.get_info();
    let attempt = info.attempt as usize;
    
    info!(
        "Processing payment {} (attempt {})",
        request.payment_id, attempt
    );
    
    // Simulate work
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Simulate intermittent failures - succeeds on 3rd attempt
    let call_count = SERVICE_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    if call_count % 3 != 2 {
        warn!("Payment gateway temporarily unavailable (attempt {})", attempt);
        return Err(ActivityError::ExecutionFailed(
            "Service temporarily unavailable - will retry".to_string()
        ));
    }
    
    info!("Payment {} processed successfully", request.payment_id);
    
    Ok(PaymentResult {
        payment_id: request.payment_id.clone(),
        status: PaymentStatus::Success,
        transaction_id: format!("txn_{}", request.payment_id),
        processed_at: chrono::Utc::now().timestamp(),
    })
}

/// Activity that fails with permanent errors (should not be retried)
pub async fn validate_payment_activity(
    _ctx: &ActivityContext,
    request: PaymentRequest,
) -> Result<RetryableOperationResult, ActivityError> {
    info!("Validating payment {}", request.payment_id);
    
    // Simulate validation
    tokio::time::sleep(Duration::from_millis(30)).await;
    
    // Validate amount
    if request.amount <= 0.0 {
        // This is a permanent error - invalid input, don't retry
        return Err(ActivityError::ExecutionFailed(
            "Permanent error: Payment amount must be greater than zero".to_string()
        ));
    }
    
    // Validate currency
    let valid_currencies = ["USD", "EUR", "GBP", "JPY"];
    if !valid_currencies.contains(&request.currency.as_str()) {
        // Permanent error - invalid currency
        return Err(ActivityError::ExecutionFailed(
            format!("Permanent error: Unsupported currency: {}", request.currency)
        ));
    }
    
    Ok(RetryableOperationResult {
        success: true,
        attempt_count: 1,
        message: "Payment validation passed".to_string(),
    })
}

/// Activity that demonstrates custom retry behavior
/// Fails a configurable number of times before succeeding
pub async fn configurable_retry_activity(
    ctx: &ActivityContext,
    (operation_id, fail_count): (String, usize),
) -> Result<RetryableOperationResult, ActivityError> {
    let info = ctx.get_info();
    let attempt = info.attempt as usize;
    
    info!(
        "Executing operation {} (attempt {}, configured to fail {} times)",
        operation_id, attempt, fail_count
    );
    
    // Simulate work
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Fail until we reach the configured fail count
    if attempt <= fail_count {
        warn!(
            "Operation {} failed on attempt {} (configured to fail {} times)",
            operation_id, attempt, fail_count
        );
        return Err(ActivityError::ExecutionFailed(
            format!("Simulated failure on attempt {}", attempt)
        ));
    }
    
    info!("Operation {} succeeded on attempt {}", operation_id, attempt);
    
    Ok(RetryableOperationResult {
        success: true,
        attempt_count: attempt,
        message: format!("Operation succeeded after {} attempts", attempt),
    })
}

/// Activity that simulates timeout scenarios
pub async fn timeout_prone_activity(
    ctx: &ActivityContext,
    duration_ms: u64,
) -> Result<RetryableOperationResult, ActivityError> {
    let info = ctx.get_info();
    let attempt = info.attempt;
    
    info!(
        "Running timeout-prone activity (attempt {}, duration {}ms)",
        attempt, duration_ms
    );
    
    // Record heartbeat before potentially long operation
    ctx.record_heartbeat(None);
    
    // Simulate variable duration work
    tokio::time::sleep(Duration::from_millis(duration_ms)).await;
    
    // Record completion
    ctx.record_heartbeat(None);
    
    Ok(RetryableOperationResult {
        success: true,
        attempt_count: attempt as usize,
        message: format!("Completed in {}ms on attempt {}", duration_ms, attempt),
    })
}

/// Reset the service call counter (for testing)
pub fn reset_service_counter() {
    SERVICE_CALL_COUNT.store(0, Ordering::SeqCst);
}
