//! Activity implementations for the complete application example.
//!
//! This module implements all business logic activities for the e-commerce system:
//! - User management
//! - Inventory management
//! - Payment processing
//! - Notification delivery
//! - Shipping coordination

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use crate::models::*;
use serde::{Deserialize, Serialize};
use tracing::{info, error, debug, warn};
use std::time::Duration;
use uuid::Uuid;
use chrono::Utc;

/// Register a new user
pub async fn register_user_activity(
    ctx: &ActivityContext,
    input: UserRegistrationInput,
) -> Result<User, ActivityError> {
    info!("Registering user: {}", input.email);
    
    let user_id = Uuid::new_v4().to_string();
    
    // Simulate database operation
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    let user = User {
        id: user_id,
        email: input.email.clone(),
        name: input.name.clone(),
        created_at: Utc::now(),
        verified: false,
    };
    
    ctx.record_heartbeat(None);
    
    info!("User registered: {}", user.id);
    Ok(user)
}

/// Send welcome email to new user
pub async fn send_welcome_email_activity(
    ctx: &ActivityContext,
    user: User,
) -> Result<NotificationResult, ActivityError> {
    info!("Sending welcome email to: {}", user.email);
    
    let notification_request = NotificationRequest {
        user_id: user.id.clone(),
        notification_type: NotificationType::WelcomeEmail,
        channels: vec![NotificationChannel::Email { address: user.email.clone() }],
        data: [
            ("user_name".to_string(), user.name.clone()),
            ("user_email".to_string(), user.email.clone()),
        ].into_iter().collect(),
    };
    
    // Simulate email sending
    tokio::time::sleep(Duration::from_millis(30)).await;
    
    ctx.record_heartbeat(None);
    
    let result = NotificationResult {
        notification_id: Uuid::new_v4().to_string(),
        sent_at: Utc::now(),
        channels_delivered: vec![NotificationChannel::Email { address: user.email }],
    };
    
    info!("Welcome email sent: {}", result.notification_id);
    Ok(result)
}

/// Reserve inventory for an order
pub async fn reserve_inventory_activity(
    ctx: &ActivityContext,
    order_id: String,
    items: Vec<OrderItem>,
) -> Result<InventoryReservation, ActivityError> {
    info!("Reserving inventory for order: {}", order_id);
    
    let reservation_id = Uuid::new_v4().to_string();
    
    // Validate stock availability
    for item in &items {
        debug!("Checking stock for product {}: {} units", item.product_id, item.quantity);
        // In real implementation: check actual inventory
    }
    
    // Simulate reservation
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let reserved_items: Vec<ReservedItem> = items.iter().map(|item| {
        ReservedItem {
            product_id: item.product_id.clone(),
            quantity: item.quantity,
            reserved_at: Utc::now(),
        }
    }).collect();
    
    ctx.record_heartbeat(None);
    
    let reservation = InventoryReservation {
        reservation_id: reservation_id.clone(),
        order_id: order_id.clone(),
        items: reserved_items,
        expires_at: Utc::now() + chrono::Duration::minutes(30),
    };
    
    info!("Inventory reserved: {} for order {}", reservation_id, order_id);
    Ok(reservation)
}

/// Release inventory reservation (compensation)
pub async fn release_inventory_activity(
    _ctx: &ActivityContext,
    reservation_id: String,
) -> Result<(), ActivityError> {
    info!("Releasing inventory reservation: {}", reservation_id);
    
    // Simulate inventory release
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    info!("Inventory reservation released: {}", reservation_id);
    Ok(())
}

/// Process payment for an order
pub async fn process_payment_activity(
    ctx: &ActivityContext,
    payment_info: PaymentInfo,
) -> Result<PaymentResult, ActivityError> {
    info!(
        "Processing payment for order {}: {} {}",
        payment_info.order_id, payment_info.currency, payment_info.amount
    );
    
    let activity_info = ctx.get_info();
    if activity_info.attempt > 1 {
        info!("Payment attempt {} for order {}", activity_info.attempt, payment_info.order_id);
    }
    
    // Simulate payment processing with external provider
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    ctx.record_heartbeat(None);
    
    let payment_id = Uuid::new_v4().to_string();
    let transaction_id = format!("txn_{}", Uuid::new_v4().to_string().replace("-", ""));
    
    // Simulate occasional failures for retry demonstration
    if payment_info.amount > 10000.0 && activity_info.attempt < 3 {
        warn!("Simulating payment failure for retry demonstration");
        return Err(ActivityError::Retryable("High value payment requires verification".to_string()));
    }
    
    let result = PaymentResult {
        payment_id: payment_id.clone(),
        status: PaymentStatus::Captured,
        transaction_id,
        processed_at: Utc::now(),
    };
    
    info!("Payment processed: {} for order {}", payment_id, payment_info.order_id);
    Ok(result)
}

/// Refund payment (compensation)
pub async fn refund_payment_activity(
    _ctx: &ActivityContext,
    payment_id: String,
) -> Result<PaymentResult, ActivityError> {
    info!("Processing refund for payment: {}", payment_id);
    
    // Simulate refund processing
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    let result = PaymentResult {
        payment_id: payment_id.clone(),
        status: PaymentStatus::Refunded,
        transaction_id: format!("ref_{}", Uuid::new_v4().to_string()),
        processed_at: Utc::now(),
    };
    
    info!("Refund processed: {}", payment_id);
    Ok(result)
}

/// Send order notification
pub async fn send_notification_activity(
    ctx: &ActivityContext,
    request: NotificationRequest,
) -> Result<NotificationResult, ActivityError> {
    info!(
        "Sending {:?} notification to user {}",
        request.notification_type, request.user_id
    );
    
    let mut delivered = Vec::new();
    
    for channel in &request.channels {
        match channel {
            NotificationChannel::Email { address } => {
                debug!("Sending email to: {}", address);
                tokio::time::sleep(Duration::from_millis(20)).await;
                delivered.push(channel.clone());
            }
            NotificationChannel::Sms { phone } => {
                debug!("Sending SMS to: {}", phone);
                tokio::time::sleep(Duration::from_millis(30)).await;
                delivered.push(channel.clone());
            }
            NotificationChannel::Push { device_token } => {
                debug!("Sending push notification to: {}", device_token);
                tokio::time::sleep(Duration::from_millis(10)).await;
                delivered.push(channel.clone());
            }
        }
    }
    
    ctx.record_heartbeat(None);
    
    let result = NotificationResult {
        notification_id: Uuid::new_v4().to_string(),
        sent_at: Utc::now(),
        channels_delivered: delivered,
    };
    
    info!("Notification sent: {} via {} channels", 
        result.notification_id, 
        result.channels_delivered.len()
    );
    Ok(result)
}

/// Create shipping label and dispatch order
pub async fn create_shipment_activity(
    ctx: &ActivityContext,
    order_id: String,
    shipping_address: Address,
) -> Result<ShippingResult, ActivityError> {
    info!("Creating shipment for order: {}", order_id);
    
    // Simulate carrier integration
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    ctx.record_heartbeat(None);
    
    let shipment_id = Uuid::new_v4().to_string();
    let tracking_number = format!("TRK{}", Uuid::new_v4().to_string().replace("-", "").to_uppercase());
    
    let result = ShippingResult {
        shipment_id: shipment_id.clone(),
        status: ShipmentStatus::LabelCreated,
        tracking_number: tracking_number.clone(),
        shipped_at: Utc::now(),
    };
    
    info!("Shipment created: {} with tracking {}", shipment_id, tracking_number);
    Ok(result)
}

/// Calculate order totals including tax and shipping
pub async fn calculate_order_total_activity(
    _ctx: &ActivityContext,
    items: Vec<OrderItem>,
) -> Result<(f64, f64, f64, f64), ActivityError> {
    debug!("Calculating order totals for {} items", items.len());
    
    let subtotal: f64 = items.iter()
        .map(|item| item.quantity as f64 * item.unit_price)
        .sum();
    
    // Calculate tax (8%)
    let tax = subtotal * 0.08;
    
    // Calculate shipping (flat $10 or free over $100)
    let shipping = if subtotal >= 100.0 { 0.0 } else { 10.0 };
    
    let total = subtotal + tax + shipping;
    
    debug!("Order total: subtotal={}, tax={}, shipping={}, total={}", 
        subtotal, tax, shipping, total);
    
    Ok((subtotal, tax, shipping, total))
}
