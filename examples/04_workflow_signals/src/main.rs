//! # Example 04: Workflow Signals
//!
//! This example demonstrates workflow signal handling patterns.
//!
//! ## Features Demonstrated
//!
//! - **Signal Reception**: Receiving and processing signals
//! - **Multiple Signal Types**: Handling different signals in one workflow
//! - **Signal Channels**: Using async channels for signal handling
//! - **State Management**: Updating workflow state based on signals
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p workflow_signals
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p workflow_signals
//! ```

use workflow_signals::*;
use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Workflow Signals Example ===\n");
    println!("This example demonstrates:");
    println!("1. Receiving and handling workflow signals");
    println!("2. Multiple signal types in a single workflow");
    println!("3. Signal-driven state updates");
    println!("4. Signal channels for async handling");
    println!();
    println!("Workflows:");
    println!("  - order_management: Multi-signal order workflow");
    println!("  - approval: Single signal wait pattern");
    println!("  - config_update: Signal aggregation pattern");
    println!();
    println!("Signals:");
    println!("  - update_status: Change order status");
    println!("  - add_item: Add item to order");
    println!("  - cancel_order: Cancel the order");
    println!("  - approval: Approve/reject a request");
    println!("  - config_update: Update configuration");
    println!();
    println!("Run tests to see the workflows in action:");
    println!("  cargo test -p workflow_signals");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::TestWorkflowEnvironment;
//
//     #[tokio::test]
//     async fn test_approval_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("send_notification", send_notification_activity);
//         env.register_workflow("approval", approval_workflow);
//
//         // Send approval signal before starting workflow
//         let signal = ApprovalSignal {
//             approver: "manager@example.com".to_string(),
//             approved: true,
//             comments: Some("Looks good".to_string()),
//         };
//         
//         env.send_signal("approval", signal);
//
//         let result = env
//             .execute_workflow("approval", "req_123".to_string())
//             .await
//             .expect("Workflow should complete");
//
//         assert!(result.approved);
//         assert_eq!(result.approver, "manager@example.com");
//         assert_eq!(result.request_id, "req_123");
//     }
//
//     #[tokio::test]
//     async fn test_rejection_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("send_notification", send_notification_activity);
//         env.register_workflow("approval", approval_workflow);
//
//         let signal = ApprovalSignal {
//             approver: "manager@example.com".to_string(),
//             approved: false,
//             comments: Some("Budget exceeded".to_string()),
//         };
//         
//         env.send_signal("approval", signal);
//
//         let result = env
//             .execute_workflow("approval", "req_456".to_string())
//             .await
//             .expect("Workflow should complete");
//
//         assert!(!result.approved);
//         assert_eq!(result.comments, Some("Budget exceeded".to_string()));
//     }
//
//     #[tokio::test]
//     async fn test_config_update_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_workflow("config_update", config_update_workflow);
//
//         // Send multiple config signals
//         for i in 0..5 {
//             let signal = ConfigUpdateSignal {
//                 config_key: format!("setting_{}", i),
//                 config_value: format!("value_{}", i),
//             };
//             env.send_signal("config_update", signal);
//         }
//
//         let result = env
//             .execute_workflow("config_update", 5usize)
//             .await
//             .expect("Workflow should complete");
//
//         assert_eq!(result.config_count, 5);
//         assert_eq!(result.configs.len(), 5);
//     }
//
//     #[tokio::test]
//     async fn test_order_management_add_items() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("send_notification", send_notification_activity);
//         env.register_activity("update_status", update_status_activity);
//         env.register_workflow("order_management", order_management_workflow);
//
//         // Send add item signals
//         for i in 0..3 {
//             let signal = AddItemSignal {
//                 item_id: format!("item_{}", i),
//                 quantity: 2,
//                 unit_price: 10.0 + i as f64,
//             };
//             env.send_signal("add_item", signal);
//         }
//
//         // Then complete the order
//         let status_signal = UpdateStatusSignal {
//             new_status: OrderStatus::Delivered,
//             updated_by: "admin".to_string(),
//             reason: None,
//         };
//         env.send_signal("update_status", status_signal);
//
//         let result = env
//             .execute_workflow("order_management", "order_123".to_string())
//             .await
//             .expect("Workflow should complete");
//
//         assert_eq!(result.items_count, 3);
//         assert!(!result.was_cancelled);
//     }
//
//     #[tokio::test]
//     async fn test_order_management_cancel() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("send_notification", send_notification_activity);
//         env.register_activity("update_status", update_status_activity);
//         env.register_workflow("order_management", order_management_workflow);
//
//         let cancel_signal = CancelOrderSignal {
//             reason: "Customer request".to_string(),
//             cancelled_by: "support".to_string(),
//         };
//         env.send_signal("cancel_order", cancel_signal);
//
//         let result = env
//             .execute_workflow("order_management", "order_456".to_string())
//             .await
//             .expect("Workflow should complete");
//
//         assert!(result.was_cancelled);
//         assert_eq!(result.final_status, OrderStatus::Cancelled);
//     }
// }
