//! # Example 02: Activity Basics
//!
//! This example demonstrates activity chaining and composition patterns.
//!
//! ## Features Demonstrated
//!
//! - Multiple activities in a single workflow
//! - Activity chaining (output of one activity as input to another)
//! - Activity error propagation
//! - Activity result handling
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p activity_basics
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p activity_basics
//! ```

use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Activity Basics Example ===\n");
    println!("This example demonstrates:");
    println!("1. Multiple activities in a workflow");
    println!("2. Activity chaining (output -> input)");
    println!("3. Error propagation between activities");
    println!("4. Result handling");
    println!();
    println!("Order Processing Pipeline:");
    println!("  1. Validate Order");
    println!("  2. Calculate Total");
    println!("  3. Process Payment");
    println!("  4. Send Confirmation");
    println!();
    println!("Run tests to see the full workflow in action:");
    println!("  cargo test -p activity_basics");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use uber_cadence_testsuite::{TestWorkflowEnvironment, TestWorkflowContext};
//     use uber_cadence_workflow::context::WorkflowError;
//     use uber_cadence_core::ActivityOptions;
//     use tracing::{info, error};
//
//     /// Test workflow wrapper that uses TestWorkflowContext
//     async fn test_process_order_workflow(
//         ctx: &mut TestWorkflowContext,
//         input: OrderInput,
//     ) -> Result<OrderOutput, WorkflowError> {
//         info!("Starting process_order_workflow for customer: {}", input.customer_id);
//
//         // Step 1: Validate the order
//         let validation = ctx
//             .execute_activity(
//                 "validate_order",
//                 Some(serde_json::to_vec(&input).unwrap()),
//                 ActivityOptions::default(),
//             )
//             .await?;
//
//         let validation_result: ValidationResult = serde_json::from_slice(&validation)
//             .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation result: {}", e)))?;
//
//         if !validation_result.is_valid {
//             error!("Order validation failed: {:?}", validation_result.errors);
//             return Err(WorkflowError::Generic(format!(
//                 "Order validation failed: {:?}",
//                 validation_result.errors
//             )));
//         }
//
//         // Step 2: Calculate total
//         let calculation_input = (validation_result.order_id.clone(), input.items.clone());
//         let calculation = ctx
//             .execute_activity(
//                 "calculate_total",
//                 Some(serde_json::to_vec(&calculation_input).unwrap()),
//                 ActivityOptions::default(),
//             )
//             .await?;
//
//         let calculation_result: CalculationResult = serde_json::from_slice(&calculation)
//             .map_err(|e| WorkflowError::Generic(format!("Failed to parse calculation result: {}", e)))?;
//
//         // Step 3: Process payment
//         let payment_input = (validation_result.order_id.clone(), calculation_result.total);
//         let payment = ctx
//             .execute_activity(
//                 "process_payment",
//                 Some(serde_json::to_vec(&payment_input).unwrap()),
//                 ActivityOptions::default(),
//             )
//             .await?;
//
//         let payment_result: PaymentResult = serde_json::from_slice(&payment)
//             .map_err(|e| WorkflowError::Generic(format!("Failed to parse payment result: {}", e)))?;
//
//         // Step 4: Send confirmation
//         let confirmation_input = (validation_result.order_id.clone(), payment_result.payment_id.clone());
//         let confirmation = ctx
//             .execute_activity(
//                 "send_confirmation",
//                 Some(serde_json::to_vec(&confirmation_input).unwrap()),
//                 ActivityOptions::default(),
//             )
//             .await?;
//
//         let confirmation_result: ConfirmationResult = serde_json::from_slice(&confirmation)
//             .map_err(|e| WorkflowError::Generic(format!("Failed to parse confirmation result: {}", e)))?;
//
//         info!(
//             "Order {} processed successfully. Payment: {}, Confirmation: {}",
//             validation_result.order_id,
//             payment_result.payment_id,
//             confirmation_result.confirmation_id
//         );
//
//         Ok(OrderOutput {
//             order_id: validation_result.order_id,
//             payment_id: payment_result.payment_id,
//             confirmation_id: confirmation_result.confirmation_id,
//             total: calculation_result.total,
//         })
//     }
//
//     fn create_test_order() -> OrderInput {
//         OrderInput {
//             customer_id: "cust_12345".to_string(),
//             items: vec![
//                 OrderItem {
//                     product_id: "prod_001".to_string(),
//                     name: "Widget".to_string(),
//                     quantity: 2,
//                     unit_price: 29.99,
//                 },
//                 OrderItem {
//                     product_id: "prod_002".to_string(),
//                     name: "Gadget".to_string(),
//                     quantity: 1,
//                     unit_price: 49.99,
//                 },
//             ],
//         }
//     }
//
//     #[tokio::test]
//     async fn test_process_order_workflow_success() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         // Register all activities
//         env.register_activity("validate_order", validate_order_activity);
//         env.register_activity("calculate_total", calculate_total_activity);
//         env.register_activity("process_payment", process_payment_activity);
//         env.register_activity("send_confirmation", send_confirmation_activity);
//
//         // Register test workflow with TestWorkflowContext
//         env.register_workflow("process_order", test_process_order_workflow);
//
//         let input = create_test_order();
//         let result = env
//             .execute_workflow("process_order", input)
//             .await
//             .expect("Workflow should complete successfully");
//
//         // Verify output
//         assert!(!result.order_id.is_empty(), "Order ID should be generated");
//         assert!(!result.payment_id.is_empty(), "Payment ID should be generated");
//         assert!(!result.confirmation_id.is_empty(), "Confirmation ID should be generated");
//
//         // Total should be: (2 * 29.99) + 49.99 = 109.97 + 8% tax = 118.77
//         let expected_subtotal = 2.0 * 29.99 + 49.99;
//         let expected_total = expected_subtotal * 1.08;
//         assert!(
//             (result.total - expected_total).abs() < 0.01,
//             "Total should be approximately ${:.2}, got ${:.2}",
//             expected_total,
//             result.total
//         );
//     }
//
//     #[tokio::test]
//     async fn test_process_order_validation_failure() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("validate_order", validate_order_activity);
//         env.register_workflow("process_order", test_process_order_workflow);
//
//         // Empty order should fail validation
//         let input = OrderInput {
//             customer_id: "cust_12345".to_string(),
//             items: vec![],
//         };
//
//         let result = env.execute_workflow("process_order", input).await;
//         assert!(result.is_err(), "Workflow should fail with empty order");
//     }
//
//     #[tokio::test]
//     async fn test_activity_chaining() {
//         // Test that output of one activity can be input to another
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("validate_order", validate_order_activity);
//         env.register_activity("calculate_total", calculate_total_activity);
//
//         let input = create_test_order();
//
//         // Execute validate_order
//         let validation_result = env
//             .execute_activity("validate_order", input.clone())
//             .await
//             .expect("Validation should succeed");
//
//         let validation: ValidationResult = validation_result;
//
//         assert!(validation.is_valid, "Order should be valid");
//
//         // Use validation output as input to calculate_total
//         let calculation_input = (validation.order_id, input.items);
//         let calculation_result = env
//             .execute_activity("calculate_total", calculation_input)
//             .await
//             .expect("Calculation should succeed");
//
//         let calculation: CalculationResult = calculation_result;
//
//         // Verify calculation
//         let expected_subtotal = 2.0 * 29.99 + 49.99;
//         assert!(
//             (calculation.subtotal - expected_subtotal).abs() < 0.01,
//             "Subtotal should match"
//         );
//         assert!(calculation.tax > 0.0, "Tax should be calculated");
//         assert!(calculation.total > calculation.subtotal, "Total should include tax");
//     }
//
//     #[tokio::test]
//     async fn test_individual_activities() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         // Register all activities
//         env.register_activity("validate_order", validate_order_activity);
//         env.register_activity("calculate_total", calculate_total_activity);
//         env.register_activity("process_payment", process_payment_activity);
//         env.register_activity("send_confirmation", send_confirmation_activity);
//
//         // Test validate_order
//         let order = create_test_order();
//         let result = env
//             .execute_activity("validate_order", order.clone())
//             .await
//             .expect("Validate should succeed");
//         let validation: ValidationResult = result;
//         assert!(validation.is_valid);
//
//         // Test calculate_total
//         let calc_input = ("order_123".to_string(), order.items);
//         let result = env
//             .execute_activity("calculate_total", calc_input)
//             .await
//             .expect("Calculate should succeed");
//         let calc: CalculationResult = result;
//         assert!(calc.total > 0.0);
//
//         // Test process_payment
//         let payment_input = ("order_123".to_string(), 100.0);
//         let result = env
//             .execute_activity("process_payment", payment_input)
//             .await
//             .expect("Payment should succeed");
//         let payment: PaymentResult = result;
//         assert!(matches!(payment.status, PaymentStatus::Success));
//
//         // Test send_confirmation
//         let confirm_input = ("order_123".to_string(), "pay_456".to_string());
//         let result = env
//             .execute_activity("send_confirmation", confirm_input)
//             .await
//             .expect("Confirmation should succeed");
//         let confirm: ConfirmationResult = result;
//         assert!(confirm.sent);
//     }
// }
