//! Domain models for the complete application example.
//!
//! This module contains all data structures used throughout the application.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub verified: bool,
}

/// User registration input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistrationInput {
    pub email: String,
    pub name: String,
    pub password_hash: String,
}

/// Product information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub stock_quantity: u32,
}

/// Order line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub sku: String,
    pub quantity: u32,
    pub unit_price: f64,
}

/// Order input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInput {
    pub user_id: String,
    pub items: Vec<OrderItem>,
    pub shipping_address: Address,
    pub payment_method: PaymentMethod,
}

/// Order output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderOutput {
    pub order_id: String,
    pub user_id: String,
    pub items: Vec<OrderItem>,
    pub total: f64,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
}

/// Order status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Reserved,
    Paid,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
}

/// Shipping address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
}

/// Payment method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentMethod {
    CreditCard { last_four: String, token: String },
    PayPal { email: String },
    BankTransfer { account_number: String },
}

/// Payment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInfo {
    pub order_id: String,
    pub amount: f64,
    pub currency: String,
    pub method: PaymentMethod,
}

/// Payment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResult {
    pub payment_id: String,
    pub status: PaymentStatus,
    pub transaction_id: String,
    pub processed_at: DateTime<Utc>,
}

/// Payment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentStatus {
    Pending,
    Authorized,
    Captured,
    Failed { reason: String },
    Refunded,
}

/// Inventory reservation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReservation {
    pub reservation_id: String,
    pub order_id: String,
    pub items: Vec<ReservedItem>,
    pub expires_at: DateTime<Utc>,
}

/// Reserved item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservedItem {
    pub product_id: String,
    pub quantity: u32,
    pub reserved_at: DateTime<Utc>,
}

/// Notification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub user_id: String,
    pub notification_type: NotificationType,
    pub channels: Vec<NotificationChannel>,
    pub data: HashMap<String, String>,
}

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    OrderConfirmation,
    PaymentReceived,
    OrderShipped,
    OrderDelivered,
    WelcomeEmail,
    PasswordReset,
}

/// Notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email { address: String },
    Sms { phone: String },
    Push { device_token: String },
}

/// Notification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResult {
    pub notification_id: String,
    pub sent_at: DateTime<Utc>,
    pub channels_delivered: Vec<NotificationChannel>,
}

/// Shipping information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingInfo {
    pub order_id: String,
    pub carrier: String,
    pub tracking_number: String,
    pub estimated_delivery: DateTime<Utc>,
}

/// Shipping result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingResult {
    pub shipment_id: String,
    pub status: ShipmentStatus,
    pub tracking_number: String,
    pub shipped_at: DateTime<Utc>,
}

/// Shipment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShipmentStatus {
    LabelCreated,
    PickedUp,
    InTransit,
    OutForDelivery,
    Delivered,
    Exception { reason: String },
}

/// Saga state for order processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSagaState {
    pub order_id: String,
    pub user_id: String,
    pub reservation_id: Option<String>,
    pub payment_id: Option<String>,
    pub shipment_id: Option<String>,
    pub notifications_sent: Vec<String>,
    pub step_results: Vec<StepResult>,
}

/// Individual saga step result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_name: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl OrderSagaState {
    pub fn new(order_id: String, user_id: String) -> Self {
        Self {
            order_id,
            user_id,
            reservation_id: None,
            payment_id: None,
            shipment_id: None,
            notifications_sent: Vec::new(),
            step_results: Vec::new(),
        }
    }

    pub fn add_step_result(&mut self, step_name: &str, success: bool, error: Option<String>) {
        self.step_results.push(StepResult {
            step_name: step_name.to_string(),
            success,
            error_message: error,
            timestamp: Utc::now(),
        });
    }
}
