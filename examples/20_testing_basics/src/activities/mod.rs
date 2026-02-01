//! Activity implementations for testing example.

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

/// Payment processing input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPaymentInput {
    pub order_id: String,
    pub amount: f64,
    pub currency: String,
}

/// Payment processing output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPaymentOutput {
    pub transaction_id: String,
    pub status: PaymentStatus,
}

/// Payment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Success,
    Failed,
    Pending,
}

/// Inventory check input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckInventoryInput {
    pub product_id: String,
    pub quantity: i32,
}

/// Inventory check output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckInventoryOutput {
    pub available: bool,
    pub warehouse_id: String,
    pub reserved_quantity: i32,
}

/// Email notification input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailInput {
    pub recipient: String,
    pub subject: String,
    pub body: String,
}

/// Process payment activity
pub async fn process_payment(
    ctx: &ActivityContext,
    input: ProcessPaymentInput,
) -> Result<ProcessPaymentOutput, ActivityError> {
    info!(
        "Processing payment for order {}: {} {}",
        input.order_id, input.amount, input.currency
    );

    // Simulate processing time
    tokio::time::sleep(Duration::from_millis(100)).await;
    ctx.record_heartbeat(None);

    // Simulate occasional failures
    if input.amount < 0.0 {
        warn!("Invalid payment amount: {}", input.amount);
        return Err(ActivityError::ExecutionFailed(
            "Invalid payment amount".to_string()
        ));
    }

    Ok(ProcessPaymentOutput {
        transaction_id: format!("txn_{}", uuid::Uuid::new_v4()),
        status: PaymentStatus::Success,
    })
}

/// Check inventory activity
pub async fn check_inventory(
    ctx: &ActivityContext,
    input: CheckInventoryInput,
) -> Result<CheckInventoryOutput, ActivityError> {
    info!(
        "Checking inventory for product {} quantity {}",
        input.product_id, input.quantity
    );

    tokio::time::sleep(Duration::from_millis(50)).await;
    ctx.record_heartbeat(None);

    // Simple logic: product IDs ending in even numbers have inventory
    let available = input.product_id.chars().last()
        .and_then(|c| c.to_digit(10))
        .map(|d| d % 2 == 0)
        .unwrap_or(false);

    Ok(CheckInventoryOutput {
        available,
        warehouse_id: if available { "WH-001".to_string() } else { String::new() },
        reserved_quantity: if available { input.quantity } else { 0 },
    })
}

/// Send email activity
pub async fn send_email(
    ctx: &ActivityContext,
    input: SendEmailInput,
) -> Result<(), ActivityError> {
    info!("Sending email to {}: {}", input.recipient, input.subject);

    tokio::time::sleep(Duration::from_millis(25)).await;
    ctx.record_heartbeat(None);

    Ok(())
}

/// Risk assessment input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessmentInput {
    pub transaction_id: String,
    pub customer_id: String,
    pub amount: f64,
}

/// Risk assessment output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessmentOutput {
    pub risk_score: f64,
    pub requires_manual_review: bool,
}

/// Risk assessment activity
pub async fn assess_risk(
    ctx: &ActivityContext,
    input: RiskAssessmentInput,
) -> Result<RiskAssessmentOutput, ActivityError> {
    info!(
        "Assessing risk for transaction {} customer {}",
        input.transaction_id, input.customer_id
    );

    tokio::time::sleep(Duration::from_millis(75)).await;
    ctx.record_heartbeat(None);

    // Simple risk scoring: higher amounts = higher risk
    let risk_score = (input.amount / 1000.0).min(1.0);
    let requires_manual_review = risk_score > 0.7;

    Ok(RiskAssessmentOutput {
        risk_score,
        requires_manual_review,
    })
}
