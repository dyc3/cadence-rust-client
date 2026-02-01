//! Activity implementations for workflow patterns example.

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Payment transaction input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInput {
    pub order_id: String,
    pub amount: f64,
    pub currency: String,
    pub customer_id: String,
}

/// Payment transaction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResult {
    pub transaction_id: String,
    pub order_id: String,
    pub status: PaymentStatus,
    pub processed_at: i64,
}

/// Payment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
    Refunded,
}

/// Inventory reservation input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryInput {
    pub order_id: String,
    pub items: Vec<InventoryItem>,
}

/// Inventory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub product_id: String,
    pub quantity: i32,
    pub unit_price: f64,
}

/// Inventory reservation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryResult {
    pub reservation_id: String,
    pub order_id: String,
    pub reserved_items: Vec<ReservedItem>,
    pub status: ReservationStatus,
}

/// Reserved item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservedItem {
    pub product_id: String,
    pub quantity: i32,
    pub warehouse_id: String,
}

/// Reservation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReservationStatus {
    Reserved,
    Partial,
    Failed,
    Released,
}

/// Shipping input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingInput {
    pub order_id: String,
    pub items: Vec<ShippingItem>,
    pub address: Address,
}

/// Shipping item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingItem {
    pub product_id: String,
    pub quantity: i32,
}

/// Address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub country: String,
    pub postal_code: String,
}

/// Shipping result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingResult {
    pub shipment_id: String,
    pub order_id: String,
    pub carrier: String,
    pub tracking_number: String,
    pub estimated_delivery: i64,
}

/// Notification input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationInput {
    pub recipient_id: String,
    pub notification_type: NotificationType,
    pub message: String,
    pub metadata: serde_json::Value,
}

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Email,
    Sms,
    Push,
}

/// Process payment activity
pub async fn process_payment_activity(
    ctx: &ActivityContext,
    input: PaymentInput,
) -> Result<PaymentResult, ActivityError> {
    info!(
        "Processing payment for order: {} amount: {} {}",
        input.order_id, input.amount, input.currency
    );

    // Simulate payment processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    ctx.record_heartbeat(None);

    // Simulate occasional failures for demonstration
    if input.amount < 0.0 {
        return Err(ActivityError::Application("Invalid payment amount".into()));
    }

    let transaction_id = format!("txn_{}", uuid::Uuid::new_v4());
    
    Ok(PaymentResult {
        transaction_id,
        order_id: input.order_id.clone(),
        status: PaymentStatus::Completed,
        processed_at: chrono::Utc::now().timestamp(),
    })
}

/// Refund payment activity (compensation)
pub async fn refund_payment_activity(
    ctx: &ActivityContext,
    input: PaymentInput,
) -> Result<PaymentResult, ActivityError> {
    info!(
        "Refunding payment for order: {} amount: {} {}",
        input.order_id, input.amount, input.currency
    );

    // Simulate refund processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    ctx.record_heartbeat(None);

    let transaction_id = format!("ref_{}", uuid::Uuid::new_v4());
    
    Ok(PaymentResult {
        transaction_id,
        order_id: input.order_id.clone(),
        status: PaymentStatus::Refunded,
        processed_at: chrono::Utc::now().timestamp(),
    })
}

/// Reserve inventory activity
pub async fn reserve_inventory_activity(
    ctx: &ActivityContext,
    input: InventoryInput,
) -> Result<InventoryResult, ActivityError> {
    info!("Reserving inventory for order: {}", input.order_id);

    let mut reserved_items = Vec::new();
    
    for item in &input.items {
        // Simulate inventory check and reservation
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        reserved_items.push(ReservedItem {
            product_id: item.product_id.clone(),
            quantity: item.quantity,
            warehouse_id: format!("wh_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        });
        
        ctx.record_heartbeat(None);
    }

    let reservation_id = format!("res_{}", uuid::Uuid::new_v4());
    
    Ok(InventoryResult {
        reservation_id,
        order_id: input.order_id.clone(),
        reserved_items,
        status: ReservationStatus::Reserved,
    })
}

/// Release inventory activity (compensation)
pub async fn release_inventory_activity(
    ctx: &ActivityContext,
    reservation_id: String,
    order_id: String,
) -> Result<InventoryResult, ActivityError> {
    info!("Releasing inventory reservation: {} for order: {}", reservation_id, order_id);

    tokio::time::sleep(Duration::from_millis(50)).await;
    ctx.record_heartbeat(None);

    Ok(InventoryResult {
        reservation_id,
        order_id,
        reserved_items: vec![],
        status: ReservationStatus::Released,
    })
}

/// Create shipment activity
pub async fn create_shipment_activity(
    ctx: &ActivityContext,
    input: ShippingInput,
) -> Result<ShippingResult, ActivityError> {
    info!("Creating shipment for order: {}", input.order_id);

    tokio::time::sleep(Duration::from_millis(150)).await;
    ctx.record_heartbeat(None);

    let shipment_id = format!("shp_{}", uuid::Uuid::new_v4());
    let tracking_number = format!("TRK{}", uuid::Uuid::new_v4().to_string().replace("-", "").to_uppercase()[..12].to_string());

    Ok(ShippingResult {
        shipment_id,
        order_id: input.order_id.clone(),
        carrier: "UPS".to_string(),
        tracking_number,
        estimated_delivery: (chrono::Utc::now() + chrono::Duration::days(3)).timestamp(),
    })
}

/// Cancel shipment activity (compensation)
pub async fn cancel_shipment_activity(
    ctx: &ActivityContext,
    shipment_id: String,
    order_id: String,
) -> Result<(), ActivityError> {
    info!("Cancelling shipment: {} for order: {}", shipment_id, order_id);

    tokio::time::sleep(Duration::from_millis(50)).await;
    ctx.record_heartbeat(None);

    Ok(())
}

/// Send notification activity
pub async fn send_notification_activity(
    ctx: &ActivityContext,
    input: NotificationInput,
) -> Result<(), ActivityError> {
    info!(
        "Sending {:?} notification to: {}",
        input.notification_type, input.recipient_id
    );

    tokio::time::sleep(Duration::from_millis(50)).await;
    ctx.record_heartbeat(None);

    Ok(())
}

/// Data processing input for fan-out/fan-in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProcessingInput {
    pub batch_id: String,
    pub records: Vec<DataRecord>,
}

/// Data record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRecord {
    pub record_id: String,
    pub data: String,
}

/// Processed record result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedRecord {
    pub record_id: String,
    pub processed_data: String,
    pub processing_time_ms: u64,
}

/// Process single record activity
pub async fn process_record_activity(
    ctx: &ActivityContext,
    record: DataRecord,
) -> Result<ProcessedRecord, ActivityError> {
    info!("Processing record: {}", record.record_id);

    let start = std::time::Instant::now();
    
    // Simulate data processing
    tokio::time::sleep(Duration::from_millis(50)).await;
    ctx.record_heartbeat(None);

    // Transform the data (just an example transformation)
    let processed_data = format!("PROCESSED_{}", record.data.to_uppercase());

    Ok(ProcessedRecord {
        record_id: record.record_id.clone(),
        processed_data,
        processing_time_ms: start.elapsed().as_millis() as u64,
    })
}
