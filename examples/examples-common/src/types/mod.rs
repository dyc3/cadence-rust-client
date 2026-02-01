//! Common types used across examples.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Order ID type for order processing examples.
pub type OrderId = Uuid;

/// Customer ID type.
pub type CustomerId = Uuid;

/// Payment ID type.
pub type PaymentId = Uuid;

/// Order status enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

/// Order structure for order processing workflow examples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub customer_id: CustomerId,
    pub items: Vec<OrderItem>,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub total_amount: f64,
}

/// Order item structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub quantity: u32,
    pub unit_price: f64,
}

/// Payment information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: PaymentId,
    pub order_id: OrderId,
    pub amount: f64,
    pub method: PaymentMethod,
    pub status: PaymentStatus,
}

/// Payment method options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMethod {
    CreditCard,
    DebitCard,
    PayPal,
    BankTransfer,
}

/// Payment status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Pending,
    Authorized,
    Captured,
    Failed,
    Refunded,
}

/// Inventory item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub product_id: String,
    pub quantity_available: u32,
    pub reserved_quantity: u32,
}

/// Inventory reservation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryReservation {
    Success,
    InsufficientStock {
        product_id: String,
        requested: u32,
        available: u32,
    },
    ProductNotFound(String),
}

/// Notification types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Notification {
    OrderConfirmation {
        order_id: OrderId,
        customer_email: String,
    },
    OrderShipped {
        order_id: OrderId,
        tracking_number: String,
    },
    PaymentFailed {
        order_id: OrderId,
        reason: String,
    },
}

/// Generic result type for activities.
pub type ActivityResult<T> = Result<T, ActivityError>;

/// Common error type for activities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityError {
    NotFound(String),
    InvalidInput(String),
    ProcessingFailed(String),
    Timeout,
    Cancelled,
    ExternalServiceError { service: String, message: String },
}

/// Workflow result type.
pub type WorkflowResult<T> = Result<T, WorkflowError>;

/// Common error type for workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowError {
    ActivityFailed(String),
    ChildWorkflowFailed(String),
    Timeout,
    Cancelled,
    InvalidState(String),
    CompensationFailed {
        original_error: String,
        compensation_error: String,
    },
}

/// Retryable trait for marking operations as retryable.
pub trait Retryable {
    fn is_retryable(&self) -> bool;
}

impl Retryable for ActivityError {
    fn is_retryable(&self) -> bool {
        matches!(
            self,
            ActivityError::Timeout | ActivityError::ExternalServiceError { .. }
        )
    }
}
