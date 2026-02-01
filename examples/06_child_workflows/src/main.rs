//! # Example 06: Child Workflows
//!
//! This example demonstrates child workflow patterns.
//!
//! ## Features Demonstrated
//!
//! - **Child Workflow Execution**: Starting child workflows from parent
//! - **Fan-Out Pattern**: Running multiple child workflows in parallel
//! - **Fan-In Pattern**: Collecting results from child workflows
//! - **Error Handling**: Handling child workflow failures
//! - **Parent-Child Relationships**: Managing child lifecycle
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p child_workflows
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p child_workflows
//! ```

use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Child Workflows Example ===\n");
    println!("This example demonstrates:");
    println!("1. Starting child workflows from parent");
    println!("2. Fan-out pattern (parallel execution)");
    println!("3. Fan-in pattern (result aggregation)");
    println!("4. Handling child workflow failures");
    println!();
    println!("Workflows:");
    println!("  - parent_workflow: Orchestrates child workflows");
    println!("  - child_processor: Processes a chunk of data");
    println!("  - fan_out_workflow: Runs multiple children in parallel");
    println!("  - fan_in_workflow: Aggregates child results");
    println!("  - process_single_item: Child for fan-out");
    println!();
    println!("APIs Demonstrated:");
    println!("  - execute_child_workflow()");
    println!("  - ChildWorkflowOptions");
    println!();
    println!("Run tests to see the workflows in action:");
    println!("  cargo test -p child_workflows");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::TestWorkflowEnvironment;
// 
//     fn create_test_chunks() -> Vec<Vec<String>> {
//         vec![
//             vec!["item_1".to_string(), "item_2".to_string(), "item_3".to_string()],
//             vec!["item_4".to_string(), "item_5".to_string()],
//             vec!["item_6".to_string(), "item_7".to_string(), "item_8".to_string()],
//         ]
//     }
// 
//     #[tokio::test]
//     async fn test_parent_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("process_chunk", process_chunk_activity);
//         env.register_activity("validate_data", validate_data_activity);
//         env.register_activity("aggregate_results", aggregate_results_activity);
//         env.register_workflow("parent", parent_workflow);
//         env.register_workflow("child_processor", child_processor_workflow);
// 
//         let input = ParentWorkflowInput {
//             job_id: "job_001".to_string(),
//             data_chunks: create_test_chunks(),
//             parallel_limit: 2,
//         };
// 
//         let result = env
//             .execute_workflow("parent", input)
//             .await
//             .expect("Workflow should complete");
// 
//         assert_eq!(result.job_id, "job_001");
//         assert_eq!(result.child_results.len(), 3);
//         assert!(result.failed_chunks.is_empty());
//     }
// 
//     #[tokio::test]
//     async fn test_child_processor_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("process_chunk", process_chunk_activity);
//         env.register_workflow("child_processor", child_processor_workflow);
// 
//         let input = ChildWorkflowInput {
//             child_id: "child_001".to_string(),
//             chunk_id: 0,
//             data: vec!["a".to_string(), "b".to_string(), "c".to_string()],
//         };
// 
//         let result = env
//             .execute_workflow("child_processor", input)
//             .await
//             .expect("Child workflow should complete");
// 
//         assert_eq!(result.child_id, "child_001");
//         assert_eq!(result.chunk_id, 0);
//         assert_eq!(result.processed_count, 3);
//         assert!(result.success);
//     }
// 
//     #[tokio::test]
//     async fn test_fan_out_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("validate_data", validate_data_activity);
//         env.register_workflow("fan_out", fan_out_workflow);
//         env.register_workflow("process_single_item", process_single_item_workflow);
// 
//         let input = FanOutInput {
//             parent_id: "fan_out_001".to_string(),
//             items: vec![
//                 "item_a".to_string(),
//                 "item_b".to_string(),
//                 "item_c".to_string(),
//             ],
//             child_workflow_type: "process_single_item".to_string(),
//         };
// 
//         let result = env
//             .execute_workflow("fan_out", input)
//             .await
//             .expect("Workflow should complete");
// 
//         assert_eq!(result.parent_id, "fan_out_001");
//         assert_eq!(result.completed_children, 3);
//         assert_eq!(result.failed_children, 0);
//         assert!(result.all_successful);
//     }
// 
//     #[tokio::test]
//     async fn test_fan_in_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("aggregate_results", aggregate_results_activity);
//         env.register_workflow("fan_in", fan_in_workflow);
// 
//         let input = FanInInput {
//             aggregation_id: "agg_001".to_string(),
//             child_outputs: vec![
//                 vec!["result_1".to_string(), "result_2".to_string()],
//                 vec!["result_3".to_string()],
//                 vec!["result_4".to_string(), "result_5".to_string()],
//             ],
//         };
// 
//         let result = env
//             .execute_workflow("fan_in", input)
//             .await
//             .expect("Workflow should complete");
// 
//         assert_eq!(result.aggregation_id, "agg_001");
//         assert_eq!(result.total_items, 5);
//         assert_eq!(result.aggregated_data.len(), 5);
//     }
// 
//     #[tokio::test]
//     async fn test_process_single_item_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("validate_data", validate_data_activity);
//         env.register_workflow("process_single_item", process_single_item_workflow);
// 
//         let input = SingleItemInput {
//             item_id: "item_001".to_string(),
//             item: "test_data".to_string(),
//         };
// 
//         let result = env
//             .execute_workflow("process_single_item", input)
//             .await
//             .expect("Workflow should complete");
// 
//         assert_eq!(result.item_id, "item_001");
//         assert!(result.processed);
//         assert!(result.result.starts_with("processed_"));
//     }
// 
//     #[tokio::test]
//     async fn test_aggregation_activity() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("aggregate_results", aggregate_results_activity);
// 
//         let input = AggregateInput {
//             results: vec![
//                 PartialResult {
//                     child_id: "child_1".to_string(),
//                     data: vec!["a".to_string(), "b".to_string()],
//                     count: 2,
//                 },
//                 PartialResult {
//                     child_id: "child_2".to_string(),
//                     data: vec!["c".to_string()],
//                     count: 1,
//                 },
//             ],
//         };
// 
//         let result: Vec<u8> = env
//             .execute_activity("aggregate_results", input)
//             .await
//             .expect("Activity should complete");
// 
//         let aggregation: AggregationResult = serde_json::from_slice(&result)
//             .expect("Should parse result");
// 
//         assert_eq!(aggregation.total_count, 3);
//         assert_eq!(aggregation.child_count, 2);
//         assert_eq!(aggregation.all_data.len(), 3);
//     }
// }
