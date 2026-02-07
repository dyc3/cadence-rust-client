//! Common activity implementations for reuse across examples.

use crate::types::{
    ActivityError, ActivityResult, InventoryReservation, Notification, Order, Payment,
};
use uber_cadence_activity::ActivityContext;

/// Mock payment processing activity.
pub async fn process_payment_activity(
    _ctx: &ActivityContext,
    payment: Payment,
) -> ActivityResult<Payment> {
    // In real implementation, this would call a payment gateway
    tracing::info!(
        "Processing payment {} for order {}",
        payment.id,
        payment.order_id
    );

    // Simulate processing
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Ok(payment)
}

/// Mock inventory reservation activity.
pub async fn reserve_inventory_activity(
    _ctx: &ActivityContext,
    order: &Order,
) -> ActivityResult<Vec<InventoryReservation>> {
    tracing::info!("Reserving inventory for order {}", order.id);

    // Mock implementation - always succeeds
    let results = order
        .items
        .iter()
        .map(|_item| InventoryReservation::Success)
        .collect();

    Ok(results)
}

/// Mock notification sending activity.
pub async fn send_notification_activity(
    _ctx: &ActivityContext,
    notification: Notification,
) -> ActivityResult<()> {
    match &notification {
        Notification::OrderConfirmation {
            order_id,
            customer_email,
        } => {
            tracing::info!(
                "Sending order confirmation for {} to {}",
                order_id,
                customer_email
            );
        }
        Notification::OrderShipped {
            order_id,
            tracking_number,
        } => {
            tracing::info!("Order {} shipped. Tracking: {}", order_id, tracking_number);
        }
        Notification::PaymentFailed { order_id, reason } => {
            tracing::warn!("Payment failed for order {}: {}", order_id, reason);
        }
    }

    Ok(())
}

/// Mock activity that can be configured to fail for testing.
pub async fn flaky_activity(ctx: &ActivityContext, should_fail: bool) -> ActivityResult<String> {
    if should_fail {
        // Check if we have heartbeat details from previous attempt
        if ctx.has_heartbeat_details() {
            let details = ctx.get_heartbeat_details();
            tracing::info!("Resuming from previous attempt with details: {:?}", details);
        }

        return Err(ActivityError::ExternalServiceError {
            service: "mock-service".to_string(),
            message: "Simulated failure".to_string(),
        });
    }

    Ok("Success".to_string())
}
