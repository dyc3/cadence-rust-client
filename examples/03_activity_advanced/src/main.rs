//! # Example 03: Activity Advanced
//!
//! This example demonstrates activity heartbeats, cancellation, and deadlines.
//!
//! ## Features Demonstrated
//!
//! - **Activity Heartbeats**: Long-running activities reporting progress
//! - **Cancellation Handling**: Gracefully handling activity cancellation
//! - **Deadline Management**: Working with activity time limits
//! - **Heartbeat Details**: Resuming from previous attempts with saved state
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p activity_advanced
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p activity_advanced
//! ```

use activity_advanced::*;
use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Activity Advanced Example ===\n");
    println!("This example demonstrates:");
    println!("1. Activity heartbeats for long-running tasks");
    println!("2. Cancellation handling with cleanup");
    println!("3. Deadline management and graceful stops");
    println!("4. Heartbeat details for resuming interrupted work");
    println!();
    println!("Activities:");
    println!("  - process_large_file: File processing with progress tracking");
    println!("  - migrate_data: Data migration with cancellation support");
    println!("  - deadline_aware_task: Respecting time limits");
    println!();
    println!("Run tests to see the workflows in action:");
    println!("  cargo test -p activity_advanced");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::{TestWorkflowEnvironment, TestWorkflowContext};
//     use cadence_workflow::context::WorkflowError;
//     use cadence_core::ActivityOptions;
//     use std::time::Duration;
//     use tracing::info;
// 
//     /// Test workflow wrapper for file_processing_workflow using TestWorkflowContext
//     async fn test_file_processing_workflow(
//         ctx: &mut TestWorkflowContext,
//         input: FileProcessingInput,
//     ) -> Result<FileProcessingResult, WorkflowError> {
//         info!("Starting file processing workflow for: {}", input.file_path);
// 
//         let activity_options = ActivityOptions {
//             schedule_to_close_timeout: Duration::from_secs(300),
//             heartbeat_timeout: Duration::from_secs(30),
//             ..Default::default()
//         };
// 
//         let result = ctx
//             .execute_activity(
//                 "process_large_file",
//                 Some(serde_json::to_vec(&input).unwrap()),
//                 activity_options,
//             )
//             .await?;
// 
//         let processing_result: FileProcessingResult = serde_json::from_slice(&result)
//             .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
// 
//         info!(
//             "File processing completed: {} chunks processed",
//             processing_result.chunks_processed
//         );
// 
//         Ok(processing_result)
//     }
// 
//     /// Test workflow wrapper for data_migration_workflow using TestWorkflowContext
//     async fn test_data_migration_workflow(
//         ctx: &mut TestWorkflowContext,
//         input: DataMigrationInput,
//     ) -> Result<MigrationResult, WorkflowError> {
//         info!(
//             "Starting data migration workflow: {} -> {}",
//             input.source_table,
//             input.destination_table
//         );
// 
//         let activity_options = ActivityOptions {
//             schedule_to_close_timeout: Duration::from_secs(600),
//             heartbeat_timeout: Duration::from_secs(60),
//             ..Default::default()
//         };
// 
//         let result = ctx
//             .execute_activity(
//                 "migrate_data",
//                 Some(serde_json::to_vec(&input).unwrap()),
//                 activity_options,
//             )
//             .await?;
// 
//         let migration_result: MigrationResult = serde_json::from_slice(&result)
//             .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
// 
//         info!(
//             "Data migration completed: {} records migrated",
//             migration_result.records_migrated
//         );
// 
//         Ok(migration_result)
//     }
// 
//     /// Test workflow wrapper for deadline_aware_workflow using TestWorkflowContext
//     async fn test_deadline_aware_workflow(
//         ctx: &mut TestWorkflowContext,
//         input: DeadlineAwareInput,
//     ) -> Result<DeadlineAwareResult, WorkflowError> {
//         info!("Starting deadline-aware workflow for task: {}", input.task_name);
// 
//         let activity_options = ActivityOptions {
//             schedule_to_close_timeout: Duration::from_secs(5),
//             heartbeat_timeout: Duration::from_secs(2),
//             ..Default::default()
//         };
// 
//         let result = ctx
//             .execute_activity(
//                 "deadline_aware_task",
//                 Some(serde_json::to_vec(&input).unwrap()),
//                 activity_options,
//             )
//             .await?;
// 
//         let task_result: DeadlineAwareResult = serde_json::from_slice(&result)
//             .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
// 
//         if task_result.stopped_by_deadline {
//             info!("Task stopped gracefully due to deadline");
//         } else if task_result.completed {
//             info!("Task completed successfully before deadline");
//         }
// 
//         Ok(task_result)
//     }
// 
//     /// Test workflow wrapper for resilient_processing_workflow using TestWorkflowContext
//     async fn test_resilient_processing_workflow(
//         ctx: &mut TestWorkflowContext,
//         input: FileProcessingInput,
//     ) -> Result<FileProcessingResult, WorkflowError> {
//         use cadence_core::RetryPolicy;
// 
//         info!(
//             "Starting resilient processing workflow for: {}",
//             input.file_path
//         );
// 
//         let retry_policy = RetryPolicy {
//             initial_interval: Duration::from_secs(1),
//             backoff_coefficient: 2.0,
//             maximum_interval: Duration::from_secs(60),
//             maximum_attempts: 3,
//             non_retryable_error_types: vec![],
//             expiration_interval: None,
//         };
// 
//         let activity_options = ActivityOptions {
//             schedule_to_close_timeout: Duration::from_secs(300),
//             heartbeat_timeout: Duration::from_secs(30),
//             retry_policy: Some(retry_policy),
//             ..Default::default()
//         };
// 
//         let result = ctx
//             .execute_activity(
//                 "process_large_file",
//                 Some(serde_json::to_vec(&input).unwrap()),
//                 activity_options,
//             )
//             .await?;
// 
//         let processing_result: FileProcessingResult = serde_json::from_slice(&result)
//             .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
// 
//         info!(
//             "Resilient processing completed with {} chunks",
//             processing_result.chunks_processed
//         );
// 
//         Ok(processing_result)
//     }
// 
//     #[tokio::test]
//     async fn test_file_processing_with_heartbeats() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("process_large_file", process_large_file_activity);
//         env.register_workflow("file_processing", test_file_processing_workflow);
// 
//         let input = FileProcessingInput {
//             file_path: "/data/large_file.dat".to_string(),
//             chunk_size: 1024 * 1024, // 1MB chunks
//         };
// 
//         let result = env
//             .execute_workflow("file_processing", input)
//             .await
//             .expect("Workflow should complete");
// 
//         assert_eq!(result.chunks_processed, 10);
//         assert!(result.total_bytes > 0);
//         assert!(!result.checksum.is_empty());
//     }
// 
//     #[tokio::test]
//     async fn test_data_migration() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("migrate_data", migrate_data_activity);
//         env.register_workflow("data_migration", test_data_migration_workflow);
// 
//         let input = DataMigrationInput {
//             source_table: "legacy_orders".to_string(),
//             destination_table: "orders_v2".to_string(),
//             batch_size: 10,
//         };
// 
//         let result = env
//             .execute_workflow("data_migration", input)
//             .await
//             .expect("Workflow should complete");
// 
//         assert_eq!(result.records_migrated, 100);
//         assert_eq!(result.source_table, "legacy_orders");
//         assert_eq!(result.destination_table, "orders_v2");
//     }
// 
//     #[tokio::test]
//     async fn test_deadline_aware_activity() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("deadline_aware_task", deadline_aware_activity);
//         env.register_workflow("deadline_aware", test_deadline_aware_workflow);
// 
//         let input = DeadlineAwareInput {
//             task_name: "data_processing".to_string(),
//             estimated_duration_secs: 10,
//         };
// 
//         let result = env
//             .execute_workflow("deadline_aware", input)
//             .await
//             .expect("Workflow should complete");
// 
//         // With a 5-second timeout and 50ms per item, we should process some items
//         assert!(result.items_processed > 0);
//         assert!(result.stopped_by_deadline || result.completed);
//     }
// 
//     #[tokio::test]
//     async fn test_heartbeat_recovery() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("process_large_file", process_large_file_activity);
//         env.register_workflow("resilient_processing", test_resilient_processing_workflow);
// 
//         let input = FileProcessingInput {
//             file_path: "/data/recoverable_file.dat".to_string(),
//             chunk_size: 1024,
//         };
// 
//         let result = env
//             .execute_workflow("resilient_processing", input)
//             .await
//             .expect("Workflow should complete");
// 
//         assert_eq!(result.chunks_processed, 10);
//     }
// 
//     #[tokio::test]
//     async fn test_activity_directly() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         env.register_activity("process_large_file", process_large_file_activity);
// 
//         let input = FileProcessingInput {
//             file_path: "/data/test.dat".to_string(),
//             chunk_size: 1000,
//         };
// 
//         let result = env
//             .execute_activity("process_large_file", input)
//             .await
//             .expect("Activity should complete");
// 
//         let processing_result: FileProcessingResult = result;
// 
//         assert_eq!(processing_result.chunks_processed, 10);
//         assert!(processing_result.checksum.starts_with("checksum-"));
//     }
// }
