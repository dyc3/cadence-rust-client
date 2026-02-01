//! Workflow implementations for the complete application example.
//!
//! This module implements the main business process workflows:
//! - User registration workflow
//! - Order processing saga
//! - Order fulfillment workflow
//! - Cancellation and refund workflow

use crate::models::*;
use cadence_core::ActivityOptions;
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use tracing::{info, error, warn};
use std::time::Duration;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;

/// User registration workflow
pub async fn user_registration_workflow(
    ctx: &mut WorkflowContext,
    input: UserRegistrationInput,
) -> Result<User, WorkflowError> {
    info!("Starting user registration workflow for: {}", input.email);
    
    // Step 1: Register user
    let user_result = ctx
        .execute_activity(
            "register_user",
            Some(serde_json::to_vec(&input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let user: User = serde_json::from_slice(&user_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse user: {}", e)))?;
    
    // Step 2: Send welcome email
    let notification_result = ctx
        .execute_activity(
            "send_welcome_email",
            Some(serde_json::to_vec(&user).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let _: NotificationResult = serde_json::from_slice(&notification_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse notification: {}", e)))?;
    
    info!("User registration workflow complete: {}", user.id);
    Ok(user)
}

/// Order processing saga with compensation
pub async fn order_processing_saga(
    ctx: &mut WorkflowContext,
    input: OrderInput,
) -> Result<OrderOutput, WorkflowError> {
    let order_id = Uuid::new_v4().to_string();
    info!("Starting order processing saga: {} for user {}", order_id, input.user_id);
    
    let mut saga_state = OrderSagaState::new(order_id.clone(), input.user_id.clone());
    
    // Step 1: Calculate order total
    let total_result = ctx
        .execute_activity(
            "calculate_order_total",
            Some(serde_json::to_vec(&input.items).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let (_subtotal, _tax, _shipping, total): (f64, f64, f64, f64) = 
        serde_json::from_slice(&total_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse totals: {}", e)))?;
    
    saga_state.add_step_result("calculate_total", true, None);
    
    // Step 2: Reserve inventory
    let reservation_result = ctx
        .execute_activity(
            "reserve_inventory",
            Some(serde_json::to_vec(&(order_id.clone(), input.items.clone())).unwrap()),
            ActivityOptions {
                start_to_close_timeout: Duration::from_secs(60),
                ..Default::default()
            },
        )
        .await?;
    
    let reservation: InventoryReservation = serde_json::from_slice(&reservation_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse reservation: {}", e)))?;
    
    saga_state.reservation_id = Some(reservation.reservation_id.clone());
    saga_state.add_step_result("reserve_inventory", true, None);
    
    // Step 3: Process payment
    let payment_info = PaymentInfo {
        order_id: order_id.clone(),
        amount: total,
        currency: "USD".to_string(),
        method: input.payment_method.clone(),
    };
    
    let payment_result = ctx
        .execute_activity(
            "process_payment",
            Some(serde_json::to_vec(&payment_info).unwrap()),
            ActivityOptions {
                start_to_close_timeout: Duration::from_secs(120),
                retry_policy: Some(cadence_core::RetryPolicy {
                    initial_interval: Duration::from_secs(1),
                    backoff_coefficient: 2.0,
                    maximum_interval: Duration::from_secs(30),
                    maximum_attempts: 5,
                    non_retryable_error_types: vec!["InvalidPaymentMethod".to_string()],
                    expiration_interval: Duration::from_secs(300),
                }),
                ..Default::default()
            },
        )
        .await;
    
    match payment_result {
        Ok(result) => {
            let payment: PaymentResult = serde_json::from_slice(&result)
                .map_err(|e| WorkflowError::Generic(format!("Failed to parse payment: {}", e)))?;
            
            saga_state.payment_id = Some(payment.payment_id.clone());
            saga_state.add_step_result("process_payment", true, None);
            
            // Step 4: Send order confirmation notification
            let mut notification_data = HashMap::new();
            notification_data.insert("order_id".to_string(), order_id.clone());
            notification_data.insert("total".to_string(), format!("{:.2}", total));
            
            let _notification_request = NotificationRequest {
                user_id: input.user_id.clone(),
                notification_type: NotificationType::OrderConfirmation,
                channels: vec![NotificationChannel::Email { address: "user@example.com".to_string() }],
                data: notification_data,
            };
            
            let notification_result = ctx
                .execute_activity(
                    "send_notification",
                    Some(serde_json::to_vec(&_notification_request).unwrap()),
                    ActivityOptions::default(),
                )
                .await?;
            
            let notification: NotificationResult = serde_json::from_slice(&notification_result)
                .map_err(|e| WorkflowError::Generic(format!("Failed to parse notification: {}", e)))?;
            
            saga_state.notifications_sent.push(notification.notification_id);
            saga_state.add_step_result("send_notification", true, None);
            
            info!("Order processing saga completed successfully: {}", order_id);
            
            Ok(OrderOutput {
                order_id,
                user_id: input.user_id,
                items: input.items,
                total,
                status: OrderStatus::Paid,
                created_at: Utc::now(),
            })
        }
        Err(e) => {
            error!("Payment failed for order {}: {:?}", order_id, e);
            
            // Compensation: Release inventory reservation
            if let Some(ref reservation_id) = saga_state.reservation_id {
                warn!("Compensating: releasing inventory reservation {}", reservation_id);
                
                let _ = ctx
                    .execute_activity(
                        "release_inventory",
                        Some(serde_json::to_vec(&reservation_id).unwrap()),
                        ActivityOptions::default(),
                    )
                    .await;
                
                saga_state.add_step_result("release_inventory", true, None);
            }
            
            // Send failure notification
            let mut notification_data = HashMap::new();
            notification_data.insert("order_id".to_string(), order_id.clone());
            notification_data.insert("error".to_string(), format!("{:?}", e));
            
            let notification_request = NotificationRequest {
                user_id: input.user_id,
                notification_type: NotificationType::OrderConfirmation,
                channels: vec![NotificationChannel::Email { address: "user@example.com".to_string() }],
                data: notification_data,
            };
            
            let _ = ctx
                .execute_activity(
                    "send_notification",
                    Some(serde_json::to_vec(&notification_request).unwrap()),
                    ActivityOptions::default(),
                )
                .await;
            
            Err(WorkflowError::Generic(format!("Order processing failed: {:?}", e)))
        }
    }
}

/// Order fulfillment workflow
pub async fn order_fulfillment_workflow(
    ctx: &mut WorkflowContext,
    order: OrderOutput,
) -> Result<ShippingResult, WorkflowError> {
    info!("Starting order fulfillment for order: {}", order.order_id);
    
    // Create shipment
    let _shipping_address = Address {
        street: "123 Main St".to_string(),
        city: "Anytown".to_string(),
        state: "CA".to_string(),
        postal_code: "12345".to_string(),
        country: "USA".to_string(),
    };
    
    let shipment_result = ctx
        .execute_activity(
            "create_shipment",
            Some(serde_json::to_vec(&(order.order_id.clone(), _shipping_address)).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let shipment: ShippingResult = serde_json::from_slice(&shipment_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse shipment: {}", e)))?;
    
    // Send shipping notification
    let mut notification_data = HashMap::new();
    notification_data.insert("order_id".to_string(), order.order_id.clone());
    notification_data.insert("tracking_number".to_string(), shipment.tracking_number.clone());
    
    let notification_request = NotificationRequest {
        user_id: order.user_id.clone(),
        notification_type: NotificationType::OrderShipped,
        channels: vec![NotificationChannel::Email { address: "user@example.com".to_string() }],
        data: notification_data,
    };
    
    let _ = ctx
        .execute_activity(
            "send_notification",
            Some(serde_json::to_vec(&notification_request).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    info!("Order fulfillment complete: {} with tracking {}", 
        order.order_id, shipment.tracking_number);
    
    Ok(shipment)
}

/// Order cancellation workflow with refund
pub async fn order_cancellation_workflow(
    ctx: &mut WorkflowContext,
    order_id: String,
    payment_id: String,
    reservation_id: String,
    user_id: String,
) -> Result<(), WorkflowError> {
    info!("Starting order cancellation: {}", order_id);
    
    // Step 1: Release inventory
    let _ = ctx
        .execute_activity(
            "release_inventory",
            Some(serde_json::to_vec(&reservation_id).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    // Step 2: Process refund
    let refund_result = ctx
        .execute_activity(
            "refund_payment",
            Some(serde_json::to_vec(&payment_id).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let _: PaymentResult = serde_json::from_slice(&refund_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse refund: {}", e)))?;
    
    // Step 3: Send cancellation notification
    let mut notification_data = HashMap::new();
    notification_data.insert("order_id".to_string(), order_id.clone());
    
    let notification_request = NotificationRequest {
        user_id,
        notification_type: NotificationType::OrderConfirmation,
        channels: vec![NotificationChannel::Email { address: "user@example.com".to_string() }],
        data: notification_data,
    };
    
    let _ = ctx
        .execute_activity(
            "send_notification",
            Some(serde_json::to_vec(&notification_request).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    info!("Order cancellation complete: {}", order_id);
    Ok(())
}
