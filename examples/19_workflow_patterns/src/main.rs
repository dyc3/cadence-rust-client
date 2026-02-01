//! # Example 19: Advanced Workflow Patterns
//!
//! This example demonstrates advanced workflow patterns including Saga pattern
//! for distributed transactions and fan-out/fan-in for parallel processing.
//!
//! ## Features Demonstrated
//!
//! - **Saga Pattern**: Compensating transactions for distributed operations
//! - **Fan-out/Fan-in**: Parallel activity execution with result aggregation
//! - **Circuit Breaker**: Fail-fast pattern for fault tolerance
//! - **Compensation**: Automatic rollback on failure
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p workflow_patterns
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p workflow_patterns
//! ```

use workflow_patterns::{
    activities::*,
    workflows::*,
};
use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Workflow Patterns Example ===\n");

    // Demonstrate Saga Pattern
    demonstrate_saga_pattern().await?;

    // Demonstrate Fan-out/Fan-in Pattern
    demonstrate_fanout_pattern().await?;

    // Demonstrate Circuit Breaker Pattern
    demonstrate_circuit_breaker().await?;

    println!("\nExample completed successfully!");
    println!("\nRun tests with: cargo test -p workflow_patterns");

    Ok(())
}

async fn demonstrate_saga_pattern() -> anyhow::Result<()> {
    println!("\n--- Saga Pattern Demonstration ---\n");
    println!("The Saga pattern manages distributed transactions with compensation.");
    println!("If any step fails, all previous steps are compensated (rolled back).\n");

    // Create a sample order
    let order_input = OrderSagaInput {
        order_id: "ORD-2024-001".to_string(),
        customer_id: "CUST-12345".to_string(),
        items: vec![
            InventoryItem {
                product_id: "PROD-001".to_string(),
                quantity: 2,
                unit_price: 29.99,
            },
            InventoryItem {
                product_id: "PROD-002".to_string(),
                quantity: 1,
                unit_price: 49.99,
            },
        ],
        total_amount: 109.97,
        currency: "USD".to_string(),
        shipping_address: Address {
            street: "123 Main St".to_string(),
            city: "San Francisco".to_string(),
            country: "USA".to_string(),
            postal_code: "94102".to_string(),
        },
    };

    println!("Order ID: {}", order_input.order_id);
    println!("Customer: {}", order_input.customer_id);
    println!("Items: {:?}", order_input.items);
    println!("Total: {} {}", order_input.total_amount, order_input.currency);
    println!("Shipping to: {}, {}\n", order_input.shipping_address.city, order_input.shipping_address.country);

    // Simulate successful order processing
    println!("Order saga workflow steps:");
    println!("  1. Process payment");
    println!("  2. Reserve inventory");
    println!("  3. Create shipment");
    println!("  4. Send notification\n");

    println!("If any step fails:");
    println!("  - Shipment creation failure → Cancel shipment + Release inventory + Refund payment");
    println!("  - Inventory reservation failure → Release inventory + Refund payment");
    println!("  - Payment failure → Just fail (nothing to compensate)\n");

    Ok(())
}

async fn demonstrate_fanout_pattern() -> anyhow::Result<()> {
    println!("\n--- Fan-out/Fan-in Pattern Demonstration ---\n");
    println!("The Fan-out/Fan-in pattern enables parallel processing of multiple items.");
    println!("All items are processed concurrently, and results are aggregated.\n");

    // Create a sample batch
    let batch_input = DataProcessingInput {
        batch_id: "BATCH-2024-001".to_string(),
        records: vec![
            DataRecord {
                record_id: "REC-001".to_string(),
                data: "raw_data_1".to_string(),
            },
            DataRecord {
                record_id: "REC-002".to_string(),
                data: "raw_data_2".to_string(),
            },
            DataRecord {
                record_id: "REC-003".to_string(),
                data: "raw_data_3".to_string(),
            },
        ],
    };

    println!("Batch ID: {}", batch_input.batch_id);
    println!("Records to process: {}", batch_input.records.len());
    println!("Records:");
    for record in &batch_input.records {
        println!("  - {}: {}", record.record_id, record.data);
    }
    println!();

    println!("Parallel processing workflow:");
    println!("  FAN-OUT: Create {} parallel processing tasks", batch_input.records.len());
    println!("  FAN-IN:  Wait for all tasks to complete and aggregate results\n");

    println!("Benefits:");
    println!("  - Reduced total processing time");
    println!("  - Independent failure handling per record");
    println!("  - Scalable with number of records\n");

    Ok(())
}

async fn demonstrate_circuit_breaker() -> anyhow::Result<()> {
    println!("\n--- Circuit Breaker Pattern Demonstration ---\n");
    println!("The Circuit Breaker pattern prevents cascading failures in distributed systems.");
    println!("After a threshold of failures, the circuit opens and requests fail fast.\n");

    let operation = CircuitBreakerOperation {
        operation_type: "external_api_call".to_string(),
        input: Some(serde_json::to_vec(&serde_json::json!({
            "endpoint": "https://api.example.com/data",
            "method": "GET",
        }))?),
        fallback: Some("cached_data".to_string()),
    };

    println!("Circuit Breaker Configuration:");
    println!("  Failure threshold: 5 failures");
    println!("  Timeout: 10 seconds");
    println!("  Operation: {}\n", operation.operation_type);

    println!("Circuit States:");
    println!("  CLOSED  → Normal operation, requests pass through");
    println!("  OPEN    → Fail fast, return error immediately");
    println!("  HALF-OPEN → Test if service has recovered\n");

    println!("State transitions:");
    println!("  1. Start in CLOSED state");
    println!("  2. After 5 failures → OPEN");
    println!("  3. After timeout → HALF-OPEN (test request)");
    println!("  4. If test succeeds → CLOSED");
    println!("  5. If test fails → OPEN again\n");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::TestWorkflowEnvironment;
// 
//     #[tokio::test]
//     async fn test_saga_pattern() {
//         println!("\nTesting Saga Pattern...\n");
// 
//         // In a real implementation, this would use TestWorkflowEnvironment
//         // to test the full saga workflow with mocked activities
//         
//         let order_input = OrderSagaInput {
//             order_id: "TEST-ORDER-001".to_string(),
//             customer_id: "TEST-CUST-001".to_string(),
//             items: vec![InventoryItem {
//                 product_id: "PROD-001".to_string(),
//                 quantity: 1,
//                 unit_price: 99.99,
//             }],
//             total_amount: 99.99,
//             currency: "USD".to_string(),
//             shipping_address: Address {
//                 street: "123 Test St".to_string(),
//                 city: "Test City".to_string(),
//                 country: "USA".to_string(),
//                 postal_code: "12345".to_string(),
//             },
//         };
// 
//         // Verify the input structure
//         assert_eq!(order_input.order_id, "TEST-ORDER-001");
//         assert_eq!(order_input.items.len(), 1);
//         assert!(order_input.total_amount > 0.0);
//     }
// 
//     #[tokio::test]
//     async fn test_fanout_pattern() {
//         println!("\nTesting Fan-out/Fan-in Pattern...\n");
// 
//         let batch_input = DataProcessingInput {
//             batch_id: "TEST-BATCH-001".to_string(),
//             records: (1..=10)
//                 .map(|i| DataRecord {
//                     record_id: format!("REC-{:03}", i),
//                     data: format!("data_{}", i),
//                 })
//                 .collect(),
//         };
// 
//         assert_eq!(batch_input.records.len(), 10);
//         assert_eq!(batch_input.batch_id, "TEST-BATCH-001");
//     }
// 
//     #[test]
//     fn test_circuit_breaker_states() {
//         println!("\nTesting Circuit Breaker States...\n");
// 
//         // Test circuit breaker operation structure
//         let operation = CircuitBreakerOperation {
//             operation_type: "test_operation".to_string(),
//             input: None,
//             fallback: Some("fallback".to_string()),
//         };
// 
//         assert_eq!(operation.operation_type, "test_operation");
//         assert!(operation.fallback.is_some());
//     }
// 
//     #[test]
//     fn test_order_status_enum() {
//         use workflow_patterns::workflows::OrderStatus;
// 
//         let statuses = vec![
//             OrderStatus::Pending,
//             OrderStatus::Confirmed,
//             OrderStatus::Failed,
//             OrderStatus::Cancelled,
//         ];
// 
//         assert_eq!(statuses.len(), 4);
//     }
// 
//     #[test]
//     fn test_payment_status() {
//         use workflow_patterns::activities::PaymentStatus;
// 
//         let statuses = vec![
//             PaymentStatus::Pending,
//             PaymentStatus::Completed,
//             PaymentStatus::Failed,
//             PaymentStatus::Refunded,
//         ];
// 
//         assert_eq!(statuses.len(), 4);
//     }
// }
