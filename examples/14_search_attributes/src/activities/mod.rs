//! Activity implementations for search attributes example.
//!
//! This example demonstrates activities that work with searchable workflows:
//! - Order processing activities
//! - Status updates that can be indexed

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

/// Order information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    pub order_id: String,
    pub customer_id: String,
    pub items: Vec<OrderItem>,
    pub priority: OrderPriority,
    pub region: String,
}

/// Order item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub quantity: u32,
    pub unit_price: f64,
}

/// Order priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Order processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderProcessingResult {
    pub order_id: String,
    pub status: OrderStatus,
    pub processed_at: i64,
    pub processing_time_ms: u64,
}

/// Order status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    Received,
    Validated,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
}

/// Inventory check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryResult {
    pub all_available: bool,
    pub unavailable_items: Vec<String>,
}

/// Activity that validates an order
pub async fn validate_order_activity(
    _ctx: &ActivityContext,
    order: OrderInfo,
) -> Result<InventoryResult, ActivityError> {
    info!("Validating order {} for customer {}", order.order_id, order.customer_id);
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Simulate inventory check
    let mut unavailable = Vec::new();
    
    for item in &order.items {
        // Simulate some items being unavailable
        if item.product_id.starts_with("OUT") {
            unavailable.push(item.product_id.clone());
        }
    }
    
    info!(
        "Order {} validation: {} items unavailable",
        order.order_id,
        unavailable.len()
    );
    
    Ok(InventoryResult {
        all_available: unavailable.is_empty(),
        unavailable_items: unavailable,
    })
}

/// Activity that processes an order
pub async fn process_order_activity(
    ctx: &ActivityContext,
    order: OrderInfo,
) -> Result<OrderProcessingResult, ActivityError> {
    let start_time = std::time::Instant::now();
    
    info!(
        "Processing order {} with {} items",
        order.order_id,
        order.items.len()
    );
    
    // Record heartbeat
    ctx.record_heartbeat(None);
    
    // Simulate processing time based on priority
    let processing_time = match order.priority {
        OrderPriority::Critical => 100,
        OrderPriority::High => 200,
        OrderPriority::Normal => 500,
        OrderPriority::Low => 1000,
    };
    
    tokio::time::sleep(Duration::from_millis(processing_time)).await;
    
    ctx.record_heartbeat(None);
    
    let processing_time_ms = start_time.elapsed().as_millis() as u64;
    
    info!("Order {} processed in {}ms", order.order_id, processing_time_ms);
    
    Ok(OrderProcessingResult {
        order_id: order.order_id,
        status: OrderStatus::Processing,
        processed_at: chrono::Utc::now().timestamp(),
        processing_time_ms,
    })
}

/// Activity that ships an order
pub async fn ship_order_activity(
    _ctx: &ActivityContext,
    order_id: String,
) -> Result<OrderProcessingResult, ActivityError> {
    info!("Shipping order {}", order_id);
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    info!("Order {} shipped", order_id);
    
    Ok(OrderProcessingResult {
        order_id,
        status: OrderStatus::Shipped,
        processed_at: chrono::Utc::now().timestamp(),
        processing_time_ms: 300,
    })
}

/// Activity that cancels an order
pub async fn cancel_order_activity(
    _ctx: &ActivityContext,
    (order_id, reason): (String, String),
) -> Result<OrderProcessingResult, ActivityError> {
    info!("Cancelling order {}: {}", order_id, reason);
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    info!("Order {} cancelled", order_id);
    
    Ok(OrderProcessingResult {
        order_id,
        status: OrderStatus::Cancelled,
        processed_at: chrono::Utc::now().timestamp(),
        processing_time_ms: 100,
    })
}

/// Activity that updates order status
pub async fn update_order_status_activity(
    _ctx: &ActivityContext,
    (order_id, status): (String, OrderStatus),
) -> Result<bool, ActivityError> {
    info!("Updating order {} status to {:?}", order_id, status);
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    info!("Order {} status updated", order_id);
    
    Ok(true)
}
