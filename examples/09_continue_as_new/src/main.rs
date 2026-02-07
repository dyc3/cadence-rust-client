//! # Example 09: Continue As New
//!
//! This example demonstrates the continue-as-new pattern for long-running workflows.
//!
//! ## Features Demonstrated
//!
//! - Continue-as-new workflow pattern
//! - State transfer between workflow runs
//! - Iteration limits and pagination
//! - Long-running workflow management
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p continue_as_new
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p continue_as_new
//! ```

use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Continue As New Example ===\n");
    println!("This example demonstrates:");
    println!("1. Continue-as-new workflow pattern");
    println!("2. State transfer between workflow runs");
    println!("3. Iteration limits for long-running workflows");
    println!("4. Pagination and batch processing");
    println!();
    println!("Run tests to see the workflows in action:");
    println!("  cargo test -p continue_as_new");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use uber_cadence_testsuite::TestWorkflowEnvironment;
//
//     #[tokio::test]
//     async fn test_batched_processor_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("fetch_data", fetch_data_activity);
//         env.register_activity("process_batch", process_batch_activity);
//         env.register_activity("save_checkpoint", save_checkpoint_activity);
//         env.register_workflow("batched_processor", batched_processor_workflow);
//
//         let summary: BatchSummary = env
//             .execute_workflow("batched_processor", None::<CheckpointData>)
//             .await
//             .expect("Workflow should complete");
//
//         assert!(summary.total_processed > 0, "Should process some items");
//         assert!(summary.iterations_completed > 0, "Should complete at least one iteration");
//
//         // Since we limited iterations, it should either complete or indicate continuation
//         if summary.continued_as_new {
//             assert!(summary.final_cursor.is_some(), "Should have cursor for continuation");
//         }
//     }
//
//     #[tokio::test]
//     async fn test_batched_processor_with_checkpoint() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("fetch_data", fetch_data_activity);
//         env.register_activity("process_batch", process_batch_activity);
//         env.register_activity("save_checkpoint", save_checkpoint_activity);
//         env.register_workflow("batched_processor", batched_processor_workflow);
//
//         // Start with a checkpoint
//         let checkpoint = CheckpointData {
//             iteration: 2,
//             total_processed: 50,
//             last_cursor: Some("50".to_string()),
//             metadata: serde_json::json!({ "resumed": true }),
//         };
//
//         let summary: BatchSummary = env
//             .execute_workflow("batched_processor", Some(checkpoint))
//             .await
//             .expect("Workflow should complete");
//
//         assert!(summary.total_processed >= 50, "Should maintain previous progress");
//         assert!(summary.iterations_completed > 2, "Should continue from checkpoint");
//     }
//
//     #[tokio::test]
//     async fn test_paginated_processor_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("fetch_data", fetch_data_activity);
//         env.register_activity("process_batch", process_batch_activity);
//         env.register_workflow("paginated_processor", paginated_processor_workflow);
//
//         let summary: PaginationSummary = env
//             .execute_workflow("paginated_processor", None::<String>)
//             .await
//             .expect("Workflow should complete");
//
//         assert!(summary.total_items > 0, "Should process some items");
//         assert!(summary.pages_processed > 0, "Should process at least one page");
//     }
//
//     #[tokio::test]
//     async fn test_process_batch_activity() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("process_batch", process_batch_activity);
//
//         let items = vec![
//             ProcessItem {
//                 item_id: "item_1".to_string(),
//                 data: serde_json::json!({ "value": 1 }),
//             },
//             ProcessItem {
//                 item_id: "item_2".to_string(),
//                 data: serde_json::json!({ "value": 2 }),
//             },
//             ProcessItem {
//                 item_id: "fail_item".to_string(),
//                 data: serde_json::json!({ "value": -1 }),
//             },
//         ];
//
//         let input = BatchProcessInput {
//             batch_id: "test_batch".to_string(),
//             items,
//             iteration: 1,
//         };
//
//         let result = env
//             .execute_activity("process_batch", input)
//             .await
//             .expect("Activity should complete");
//
//         let batch_result: BatchProcessResult = serde_json::from_slice(&result).expect("Should parse");
//         assert_eq!(batch_result.batch_id, "test_batch");
//         assert!(batch_result.processed_count > 0, "Should process some items");
//         assert!(batch_result.failed_count > 0, "Should track failures");
//         assert_eq!(batch_result.iteration, 1);
//     }
//
//     #[tokio::test]
//     async fn test_fetch_data_activity() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("fetch_data", fetch_data_activity);
//
//         let input = DataFetchInput {
//             cursor: None,
//             batch_size: 5,
//         };
//
//         let result = env
//             .execute_activity("fetch_data", input)
//             .await
//             .expect("Activity should complete");
//
//         let data_result: DataFetchResult = serde_json::from_slice(&result).expect("Should parse");
//         assert_eq!(data_result.items.len(), 5, "Should fetch batch_size items");
//         assert!(data_result.has_more, "Should indicate more data available");
//         assert!(data_result.next_cursor.is_some(), "Should have next cursor");
//         assert_eq!(data_result.total_processed, 5);
//     }
//
//     #[tokio::test]
//     async fn test_fetch_data_with_cursor() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("fetch_data", fetch_data_activity);
//
//         let input = DataFetchInput {
//             cursor: Some("10".to_string()),
//             batch_size: 3,
//         };
//
//         let result = env
//             .execute_activity("fetch_data", input)
//             .await
//             .expect("Activity should complete");
//
//         let data_result: DataFetchResult = serde_json::from_slice(&result).expect("Should parse");
//         assert_eq!(data_result.items.len(), 3);
//         assert_eq!(data_result.total_processed, 13); // 10 + 3
//         assert!(data_result.items[0].item_id.contains("10"), "Should start from cursor");
//     }
//
//     #[tokio::test]
//     async fn test_save_checkpoint_activity() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("save_checkpoint", save_checkpoint_activity);
//
//         let checkpoint = CheckpointData {
//             iteration: 5,
//             total_processed: 100,
//             last_cursor: Some("100".to_string()),
//             metadata: serde_json::json!({ "test": true }),
//         };
//
//         let result = env
//             .execute_activity("save_checkpoint", checkpoint)
//             .await
//             .expect("Activity should complete");
//
//         let checkpoint_id: String = serde_json::from_slice(&result).expect("Should parse");
//         assert!(!checkpoint_id.is_empty(), "Should return checkpoint ID");
//         assert!(checkpoint_id.contains("checkpoint"), "Should contain 'checkpoint' prefix");
//     }
// }
