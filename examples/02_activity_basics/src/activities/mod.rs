//! Activity implementations for activity basics example.
//!
//! This example demonstrates a simple order processing pipeline:
//! 1. Validate order
//! 2. Calculate total
//! 3. Process payment
//! 4. Send confirmation

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};
use uber_cadence_activity::ActivityContext;
use uber_cadence_worker::ActivityError;
use uuid::Uuid;

/// Order item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub name: String,
    pub quantity: u32,
    pub unit_price: f64,
}

/// Order input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInput {
    pub customer_id: String,
    pub items: Vec<OrderItem>,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub order_id: String,
    pub errors: Vec<String>,
}

/// Calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationResult {
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
}

/// Payment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResult {
    pub payment_id: String,
    pub status: PaymentStatus,
    pub amount: f64,
}

/// Payment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentStatus {
    Success,
    Failed,
    Pending,
}

/// Confirmation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationResult {
    pub confirmation_id: String,
    pub sent: bool,
}

/// Validate the order
pub async fn validate_order_activity(
    _ctx: &ActivityContext,
    input: OrderInput,
) -> Result<ValidationResult, ActivityError> {
    info!("Validating order for customer: {}", input.customer_id);

    let mut errors = Vec::new();

    // Check if order has items
    if input.items.is_empty() {
        errors.push("Order must have at least one item".to_string());
    }

    // Validate each item
    for item in &input.items {
        if item.quantity == 0 {
            errors.push(format!("Item {} has zero quantity", item.product_id));
        }
        if item.unit_price < 0.0 {
            errors.push(format!("Item {} has negative price", item.product_id));
        }
    }

    // Simulate validation delay
    tokio::time::sleep(Duration::from_millis(50)).await;

    let is_valid = errors.is_empty();
    let order_id = Uuid::new_v4().to_string();

    if is_valid {
        info!("Order {} validated successfully", order_id);
    } else {
        warn!("Order validation failed: {:?}", errors);
    }

    Ok(ValidationResult {
        is_valid,
        order_id,
        errors,
    })
}

/// Calculate order totals
pub async fn calculate_total_activity(
    _ctx: &ActivityContext,
    (order_id, items): (String, Vec<OrderItem>),
) -> Result<CalculationResult, ActivityError> {
    info!("Calculating total for order: {}", order_id);

    let subtotal: f64 = items
        .iter()
        .map(|item| item.quantity as f64 * item.unit_price)
        .sum();

    // Calculate tax (8%)
    let tax = subtotal * 0.08;
    let total = subtotal + tax;

    info!(
        "Order {}: subtotal=${:.2}, tax=${:.2}, total=${:.2}",
        order_id, subtotal, tax, total
    );

    Ok(CalculationResult {
        subtotal,
        tax,
        total,
    })
}

/// Process payment
pub async fn process_payment_activity(
    ctx: &ActivityContext,
    (order_id, amount): (String, f64),
) -> Result<PaymentResult, ActivityError> {
    info!("Processing payment for order {}: ${:.2}", order_id, amount);

    // Get activity info for retry tracking
    let info = ctx.get_info();
    if info.attempt > 1 {
        info!("Payment attempt {} for order {}", info.attempt, order_id);
    }

    // Simulate payment processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    let payment_id = Uuid::new_v4().to_string();

    info!(
        "Payment {} processed successfully for order {}",
        payment_id, order_id
    );

    Ok(PaymentResult {
        payment_id,
        status: PaymentStatus::Success,
        amount,
    })
}

/// Send confirmation notification
pub async fn send_confirmation_activity(
    _ctx: &ActivityContext,
    (order_id, payment_id): (String, String),
) -> Result<ConfirmationResult, ActivityError> {
    info!(
        "Sending confirmation for order {} (payment: {})",
        order_id, payment_id
    );

    // Simulate sending email/notification
    tokio::time::sleep(Duration::from_millis(50)).await;

    let confirmation_id = Uuid::new_v4().to_string();

    info!(
        "Confirmation {} sent for order {}",
        confirmation_id, order_id
    );

    Ok(ConfirmationResult {
        confirmation_id,
        sent: true,
    })
}
