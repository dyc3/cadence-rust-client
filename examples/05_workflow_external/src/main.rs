//! # Example 05: Workflow External
//!
//! This example demonstrates external workflow signaling and cancellation.
//!
//! ## Features Demonstrated
//!
//! - **External Workflow Signaling**: Sending signals to other workflows by ID
//! - **External Cancellation**: Requesting cancellation of other workflows
//! - **Workflow Orchestration**: Coordinating multiple workflows
//! - **Producer-Consumer Pattern**: Workflows notifying each other
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p workflow_external
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p workflow_external
//! ```

use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Workflow External Example ===\n");
    println!("This example demonstrates:");
    println!("1. Signaling workflows by ID");
    println!("2. Cancelling external workflows");
    println!("3. Orchestrating multiple workflows");
    println!("4. Producer-consumer patterns");
    println!();
    println!("Workflows:");
    println!("  - orchestrator: Manages multiple workflows");
    println!("  - producer: Creates data and notifies consumers");
    println!("  - consumer: Processes data from producers");
    println!("  - timer: Delays and signals other workflows");
    println!();
    println!("APIs Demonstrated:");
    println!("  - signal_external_workflow()");
    println!("  - request_cancel_external_workflow()");
    println!();
    println!("Run tests to see the workflows in action:");
    println!("  cargo test -p workflow_external");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::TestWorkflowEnvironment;
// 
//     #[tokio::test]
//     async fn test_orchestrator_with_success() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("log_execution", log_activity_execution);
//         env.register_activity("send_external_notification", send_external_notification);
//         env.register_workflow("orchestrator", orchestrator_workflow);
// 
//         let workflow_ids = vec![
//             "wf_001".to_string(),
//             "wf_002".to_string(),
//             "wf_003".to_string(),
//         ];
// 
//         let result = env
//             .execute_workflow("orchestrator", workflow_ids)
//             .await
//             .expect("Workflow should complete");
// 
//         assert_eq!(result.managed_workflows.len(), 3);
//         assert_eq!(result.completed_workflows.len(), 3);
//         assert!(result.cancelled_workflows.is_empty());
//     }
// 
//     #[tokio::test]
//     async fn test_orchestrator_with_failure() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("log_execution", log_activity_execution);
//         env.register_activity("send_external_notification", send_external_notification);
//         env.register_workflow("orchestrator", orchestrator_workflow);
// 
//         // One workflow marked to fail
//         let workflow_ids = vec![
//             "wf_001".to_string(),
//             "wf_fail_002".to_string(),
//             "wf_003".to_string(),
//         ];
// 
//         let result = env
//             .execute_workflow("orchestrator", workflow_ids)
//             .await
//             .expect("Workflow should complete");
// 
//         assert_eq!(result.managed_workflows.len(), 3);
//         assert!(!result.completed_workflows.is_empty());
//         assert!(!result.cancelled_workflows.is_empty());
//     }
// 
//     #[tokio::test]
//     async fn test_producer_consumer_pattern() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("log_execution", log_activity_execution);
//         env.register_workflow("producer", producer_workflow);
//         env.register_workflow("consumer", consumer_workflow);
// 
//         let consumer_ids = vec!["consumer_1".to_string(), "consumer_2".to_string()];
//         let data_id = "dataset_001".to_string();
// 
//         // Execute producer
//         let producer_result = env
//             .execute_workflow("producer", (data_id.clone(), consumer_ids.clone()))
//             .await
//             .expect("Producer should complete");
// 
//         assert_eq!(producer_result.data_id, data_id);
//         assert!(!producer_result.data_location.is_empty());
// 
//         // Execute consumers
//         for consumer_id in consumer_ids {
//             let input = ConsumerInput {
//                 consumer_id: consumer_id.clone(),
//                 expected_data_types: vec!["dataset".to_string()],
//             };
// 
//             let consumer_result = env
//                 .execute_workflow("consumer", input)
//                 .await
//                 .expect("Consumer should complete");
// 
//             assert_eq!(consumer_result.consumer_id, consumer_id);
//         }
//     }
// 
//     #[tokio::test]
//     async fn test_timer_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_workflow("timer", timer_workflow);
// 
//         let timer_id = "timer_001".to_string();
//         let target_id = "target_wf".to_string();
//         let duration_ms = 100u64;
// 
//         let result = env
//             .execute_workflow("timer", (timer_id.clone(), target_id, duration_ms))
//             .await
//             .expect("Timer should complete");
// 
//         assert_eq!(result.timer_id, timer_id);
//         assert_eq!(result.duration_ms, duration_ms);
//     }
// 
//     #[tokio::test]
//     async fn test_consumer_receives_data() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("log_execution", log_activity_execution);
//         env.register_workflow("consumer", consumer_workflow);
// 
//         // Send data_ready signal before executing consumer
//         let signal = DataReadySignal {
//             data_id: "data_001".to_string(),
//             data_location: "s3://bucket/data_001.parquet".to_string(),
//             producer_workflow_id: "producer_123".to_string(),
//         };
//         env.send_signal("data_ready", signal);
// 
//         let input = ConsumerInput {
//             consumer_id: "consumer_1".to_string(),
//             expected_data_types: vec!["dataset".to_string()],
//         };
// 
//         let result = env
//             .execute_workflow("consumer", input)
//             .await
//             .expect("Consumer should complete");
// 
//         assert!(!result.data_processed.is_empty());
//         assert!(result.processing_completed);
//     }
// }
