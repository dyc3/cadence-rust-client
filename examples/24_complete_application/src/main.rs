//! # Example 24: Complete Application
//!
//! This example demonstrates a production-ready complete e-commerce application using Cadence.
//!
//! ## Features Demonstrated
//!
//! - User registration workflow
//! - Order processing saga with compensation
//! - Inventory management with reservations
//! - Payment processing with retry logic
//! - Multi-channel notifications
//! - Order fulfillment and shipping
//! - Cancellation and refund handling
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p complete_application
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p complete_application
//! ```

use complete_application::*;
use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Complete Application Example ===\n");
    println!("This example demonstrates a production-ready e-commerce application:");
    println!();
    println!("Features:");
    println!("  - User registration with welcome emails");
    println!("  - Order processing saga with compensation");
    println!("  - Inventory reservation and management");
    println!("  - Payment processing with retry logic");
    println!("  - Multi-channel notifications (Email, SMS, Push)");
    println!("  - Order fulfillment and shipping");
    println!("  - Order cancellation and refunds");
    println!();
    println!("Architecture Patterns:");
    println!("  - Saga pattern for distributed transactions");
    println!("  - Compensation for failure recovery");
    println!("  - Idempotent operations");
    println!("  - Comprehensive error handling");
    println!();
    println!("Run tests to see all workflows in action:");
    println!("  cargo test -p complete_application -- --nocapture");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::TestWorkflowEnvironment;
//     use chrono::Utc;
// 
//     fn create_test_user_input() -> UserRegistrationInput {
//         UserRegistrationInput {
//             email: "test@example.com".to_string(),
//             name: "Test User".to_string(),
//             password_hash: "hashed_password_123".to_string(),
//         }
//     }
// 
//     fn create_test_order_input() -> OrderInput {
//         OrderInput {
//             user_id: "user_123".to_string(),
//             items: vec![
//                 OrderItem {
//                     product_id: "prod_001".to_string(),
//                     sku: "WIDGET-001".to_string(),
//                     quantity: 2,
//                     unit_price: 29.99,
//                 },
//                 OrderItem {
//                     product_id: "prod_002".to_string(),
//                     sku: "GADGET-001".to_string(),
//                     quantity: 1,
//                     unit_price: 49.99,
//                 },
//             ],
//             shipping_address: Address {
//                 street: "123 Main St".to_string(),
//                 city: "Anytown".to_string(),
//                 state: "CA".to_string(),
//                 postal_code: "12345".to_string(),
//                 country: "USA".to_string(),
//             },
//             payment_method: PaymentMethod::CreditCard {
//                 last_four: "4242".to_string(),
//                 token: "tok_visa".to_string(),
//             },
//         }
//     }
// 
//     #[tokio::test]
//     async fn test_user_registration_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         
//         env.register_activity("register_user", register_user_activity);
//         env.register_activity("send_welcome_email", send_welcome_email_activity);
//         env.register_workflow("user_registration", user_registration_workflow);
// 
//         let input = create_test_user_input();
//         
//         let user = env
//             .execute_workflow("user_registration", input)
//             .await
//             .expect("User registration should succeed");
// 
//         assert_eq!(user.email, "test@example.com");
//         assert_eq!(user.name, "Test User");
//         assert!(!user.id.is_empty());
//         assert!(user.created_at <= Utc::now());
//         
//         println!("\nUser Registration Workflow:");
//         println!("  User ID: {}", user.id);
//         println!("  Email: {}", user.email);
//         println!("  Name: {}", user.name);
//     }
// 
//     #[tokio::test]
//     async fn test_order_processing_saga_success() {
//         let mut env = TestWorkflowEnvironment::new();
//         
//         // Register all activities
//         env.register_activity("calculate_order_total", calculate_order_total_activity);
//         env.register_activity("reserve_inventory", reserve_inventory_activity);
//         env.register_activity("process_payment", process_payment_activity);
//         env.register_activity("send_notification", send_notification_activity);
//         env.register_activity("release_inventory", release_inventory_activity);
//         env.register_activity("refund_payment", refund_payment_activity);
//         
//         env.register_workflow("order_processing", order_processing_saga);
// 
//         let input = create_test_order_input();
//         
//         let order = env
//             .execute_workflow("order_processing", input)
//             .await
//             .expect("Order processing should succeed");
// 
//         assert!(!order.order_id.is_empty());
//         assert_eq!(order.user_id, "user_123");
//         assert_eq!(order.items.len(), 2);
//         
//         // Verify total: (2 * 29.99) + 49.99 = 109.97 + 8% tax = 118.77 + 10 shipping = 128.77
//         let expected_subtotal = 2.0 * 29.99 + 49.99;
//         let expected_total = expected_subtotal * 1.08 + 10.0;
//         assert!((order.total - expected_total).abs() < 0.01, 
//             "Expected {:.2}, got {:.2}", expected_total, order.total);
//         
//         assert!(matches!(order.status, OrderStatus::Paid));
//         
//         println!("\nOrder Processing Saga (Success):");
//         println!("  Order ID: {}", order.order_id);
//         println!("  Total: ${:.2}", order.total);
//         println!("  Status: {:?}", order.status);
//         println!("  Items: {}", order.items.len());
//     }
// 
//     #[tokio::test]
//     async fn test_order_processing_saga_compensation() {
//         let mut env = TestWorkflowEnvironment::new();
//         
//         env.register_activity("calculate_order_total", calculate_order_total_activity);
//         env.register_activity("reserve_inventory", reserve_inventory_activity);
//         env.register_activity("process_payment", process_payment_activity);
//         env.register_activity("send_notification", send_notification_activity);
//         env.register_activity("release_inventory", release_inventory_activity);
//         
//         env.register_workflow("order_processing", order_processing_saga);
// 
//         // Create order with high amount to trigger simulated payment failure
//         let mut input = create_test_order_input();
//         input.items.push(OrderItem {
//             product_id: "prod_expensive".to_string(),
//             sku: "EXPENSIVE-001".to_string(),
//             quantity: 1000,
//             unit_price: 999.99,
//         });
//         
//         let result = env.execute_workflow("order_processing", input).await;
//         
//         // Should fail due to simulated payment failure, triggering compensation
//         assert!(result.is_err(), "Order should fail with high-value payment");
//         
//         println!("\nOrder Processing Saga (Compensation):");
//         println!("  Order failed as expected with high-value payment");
//         println!("  Inventory reservation was released (compensation executed)");
//     }
// 
//     #[tokio::test]
//     async fn test_order_fulfillment_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         
//         env.register_activity("create_shipment", create_shipment_activity);
//         env.register_activity("send_notification", send_notification_activity);
//         env.register_workflow("order_fulfillment", order_fulfillment_workflow);
// 
//         let order = OrderOutput {
//             order_id: "order_123".to_string(),
//             user_id: "user_123".to_string(),
//             items: create_test_order_input().items,
//             total: 128.77,
//             status: OrderStatus::Paid,
//             created_at: Utc::now(),
//         };
//         
//         let shipment = env
//             .execute_workflow("order_fulfillment", order)
//             .await
//             .expect("Fulfillment should succeed");
// 
//         assert!(!shipment.shipment_id.is_empty());
//         assert!(!shipment.tracking_number.is_empty());
//         assert!(matches!(shipment.status, ShipmentStatus::LabelCreated));
//         
//         println!("\nOrder Fulfillment Workflow:");
//         println!("  Shipment ID: {}", shipment.shipment_id);
//         println!("  Tracking: {}", shipment.tracking_number);
//         println!("  Status: {:?}", shipment.status);
//     }
// 
//     #[tokio::test]
//     async fn test_order_cancellation_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         
//         env.register_activity("release_inventory", release_inventory_activity);
//         env.register_activity("refund_payment", refund_payment_activity);
//         env.register_activity("send_notification", send_notification_activity);
//         env.register_workflow("order_cancellation", order_cancellation_workflow);
// 
//         let result = env
//             .execute_workflow(
//                 "order_cancellation",
//                 ("order_123".to_string(), "payment_456".to_string(), "reservation_789".to_string(), "user_123".to_string())
//             )
//             .await;
// 
//         assert!(result.is_ok(), "Cancellation should succeed");
//         
//         println!("\nOrder Cancellation Workflow:");
//         println!("  Inventory released: reservation_789");
//         println!("  Refund processed: payment_456");
//         println!("  Cancellation notification sent");
//     }
// 
//     #[tokio::test]
//     async fn test_individual_activities() {
//         let mut env = TestWorkflowEnvironment::new();
//         
//         // Register all activities
//         env.register_activity("register_user", register_user_activity);
//         env.register_activity("send_welcome_email", send_welcome_email_activity);
//         env.register_activity("calculate_order_total", calculate_order_total_activity);
//         env.register_activity("reserve_inventory", reserve_inventory_activity);
//         env.register_activity("release_inventory", release_inventory_activity);
//         env.register_activity("process_payment", process_payment_activity);
//         env.register_activity("refund_payment", refund_payment_activity);
//         env.register_activity("send_notification", send_notification_activity);
//         env.register_activity("create_shipment", create_shipment_activity);
// 
//         println!("\n=== Testing Individual Activities ===");
// 
//         // Test register_user
//         let user_input = create_test_user_input();
//         let result = env
//             .execute_activity("register_user", user_input.clone())
//             .await
//             .expect("Register should succeed");
//         let user: User = serde_json::from_slice(&result).expect("Should parse");
//         assert_eq!(user.email, user_input.email);
//         println!("  register_user: OK ({})", user.id);
// 
//         // Test calculate_order_total
//         let items = create_test_order_input().items;
//         let result = env
//             .execute_activity("calculate_order_total", items.clone())
//             .await
//             .expect("Calculate should succeed");
//         let (subtotal, tax, shipping, total): (f64, f64, f64, f64) = 
//             serde_json::from_slice(&result).expect("Should parse");
//         assert!(total > 0.0);
//         println!("  calculate_order_total: OK (${:.2})", total);
// 
//         // Test reserve_inventory
//         let order_id = "order_test".to_string();
//         let result = env
//             .execute_activity("reserve_inventory", (order_id.clone(), items.clone()))
//             .await
//             .expect("Reserve should succeed");
//         let reservation: InventoryReservation = serde_json::from_slice(&result).expect("Should parse");
//         assert!(!reservation.reservation_id.is_empty());
//         println!("  reserve_inventory: OK ({})", reservation.reservation_id);
// 
//         // Test process_payment
//         let payment_info = PaymentInfo {
//             order_id: order_id.clone(),
//             amount: 100.0,
//             currency: "USD".to_string(),
//             method: PaymentMethod::CreditCard { last_four: "4242".to_string(), token: "tok".to_string() },
//         };
//         let result = env
//             .execute_activity("process_payment", payment_info)
//             .await
//             .expect("Payment should succeed");
//         let payment: PaymentResult = serde_json::from_slice(&result).expect("Should parse");
//         assert!(!payment.payment_id.is_empty());
//         println!("  process_payment: OK ({})", payment.payment_id);
// 
//         // Test create_shipment
//         let address = create_test_order_input().shipping_address;
//         let result = env
//             .execute_activity("create_shipment", (order_id.clone(), address))
//             .await
//             .expect("Shipment should succeed");
//         let shipment: ShippingResult = serde_json::from_slice(&result).expect("Should parse");
//         assert!(!shipment.tracking_number.is_empty());
//         println!("  create_shipment: OK ({})", shipment.tracking_number);
// 
//         println!("\nAll activities passed!");
//     }
// 
//     #[tokio::test]
//     async fn test_complete_order_lifecycle() {
//         let mut env = TestWorkflowEnvironment::new();
//         
//         // Register all activities
//         env.register_activity("register_user", register_user_activity);
//         env.register_activity("send_welcome_email", send_welcome_email_activity);
//         env.register_activity("calculate_order_total", calculate_order_total_activity);
//         env.register_activity("reserve_inventory", reserve_inventory_activity);
//         env.register_activity("process_payment", process_payment_activity);
//         env.register_activity("send_notification", send_notification_activity);
//         env.register_activity("create_shipment", create_shipment_activity);
//         env.register_activity("release_inventory", release_inventory_activity);
//         env.register_activity("refund_payment", refund_payment_activity);
//         
//         // Register all workflows
//         env.register_workflow("user_registration", user_registration_workflow);
//         env.register_workflow("order_processing", order_processing_saga);
//         env.register_workflow("order_fulfillment", order_fulfillment_workflow);
//         env.register_workflow("order_cancellation", order_cancellation_workflow);
// 
//         println!("\n=== Complete Order Lifecycle Test ===\n");
// 
//         // Step 1: User Registration
//         println!("Step 1: User Registration");
//         let user_input = create_test_user_input();
//         let user = env
//             .execute_workflow("user_registration", user_input)
//             .await
//             .expect("Registration should succeed");
//         println!("  Created user: {} ({})", user.id, user.email);
// 
//         // Step 2: Place Order
//         println!("\nStep 2: Order Processing");
//         let order_input = create_test_order_input();
//         let order = env
//             .execute_workflow("order_processing", order_input)
//             .await
//             .expect("Order processing should succeed");
//         println!("  Order created: {}", order.order_id);
//         println!("  Total: ${:.2}", order.total);
//         println!("  Status: {:?}", order.status);
// 
//         // Step 3: Fulfillment
//         println!("\nStep 3: Order Fulfillment");
//         let shipment = env
//             .execute_workflow("order_fulfillment", order.clone())
//             .await
//             .expect("Fulfillment should succeed");
//         println!("  Shipment created: {}", shipment.shipment_id);
//         println!("  Tracking: {}", shipment.tracking_number);
// 
//         println!("\n✅ Complete order lifecycle successful!");
//     }
// }
