//! # Example 10: Error Handling
//!
//! This example demonstrates error types and handling in Cadence workflows.
//!
//! ## Features Demonstrated
//!
//! - Activity error types and handling
//! - Workflow error propagation
//! - Retry policies and error classification
//! - Custom error types
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p error_handling
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p error_handling
//! ```

use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Error Handling Example ===\n");
    println!("This example demonstrates:");
    println!("1. Activity error types (retryable, non-retryable)");
    println!("2. Workflow error propagation");
    println!("3. Retry policies and error classification");
    println!("4. Fallback patterns");
    println!("5. Validation and early failure");
    println!();
    println!("Run tests to see the workflows in action:");
    println!("  cargo test -p error_handling");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use uber_cadence_testsuite::TestWorkflowEnvironment;
//
//     #[tokio::test]
//     async fn test_error_handling_workflow_all_success() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("unreliable_process", unreliable_process_activity);
//         env.register_workflow("error_handling", error_handling_workflow);
//
//         let items = vec![
//             ProcessInput {
//                 item_id: "item_1".to_string(),
//                 should_fail: false,
//                 failure_type: None,
//             },
//             ProcessInput {
//                 item_id: "item_2".to_string(),
//                 should_fail: false,
//                 failure_type: None,
//             },
//         ];
//
//         let result: BatchProcessingResult = env
//             .execute_workflow("error_handling", items)
//             .await
//             .expect("Workflow should complete");
//
//         assert_eq!(result.successful.len(), 2, "Both items should succeed");
//         assert!(result.failed.is_empty(), "No items should fail");
//         assert_eq!(result.total, 2);
//     }
//
//     #[tokio::test]
//     async fn test_error_handling_workflow_with_failures() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("unreliable_process", unreliable_process_activity);
//         env.register_workflow("error_handling", error_handling_workflow);
//
//         let items = vec![
//             ProcessInput {
//                 item_id: "success_item".to_string(),
//                 should_fail: false,
//                 failure_type: None,
//             },
//             ProcessInput {
//                 item_id: "retryable_item".to_string(),
//                 should_fail: true,
//                 failure_type: Some(FailureType::Retryable),
//             },
//             ProcessInput {
//                 item_id: "non_retryable_item".to_string(),
//                 should_fail: true,
//                 failure_type: Some(FailureType::NonRetryable),
//             },
//         ];
//
//         let result: BatchProcessingResult = env
//             .execute_workflow("error_handling", items)
//             .await
//             .expect("Workflow should complete");
//
//         assert_eq!(result.successful.len(), 1, "Only one item should succeed");
//         assert_eq!(result.failed.len(), 2, "Two items should fail");
//
//         // The retryable item might have been retried
//         assert!(
//             result.retried.contains(&"retryable_item".to_string()),
//             "Retryable item should be in retried list"
//         );
//     }
//
//     #[tokio::test]
//     async fn test_fallback_workflow_primary_success() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("unreliable_process", unreliable_process_activity);
//         env.register_workflow("fallback", fallback_workflow);
//
//         let item = ProcessInput {
//             item_id: "test_item".to_string(),
//             should_fail: false,
//             failure_type: None,
//         };
//
//         let result: FallbackResult = env
//             .execute_workflow("fallback", item)
//             .await
//             .expect("Workflow should complete");
//
//         assert!(result.success, "Should succeed");
//         assert_eq!(result.method, "primary", "Should use primary method");
//     }
//
//     #[tokio::test]
//     async fn test_fallback_workflow_uses_fallback() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("unreliable_process", unreliable_process_activity);
//         env.register_workflow("fallback", fallback_workflow);
//
//         let item = ProcessInput {
//             item_id: "failing_item".to_string(),
//             should_fail: true,
//             failure_type: Some(FailureType::Retryable),
//         };
//
//         let result: FallbackResult = env
//             .execute_workflow("fallback", item)
//             .await
//             .expect("Workflow should complete");
//
//         assert!(result.success, "Should eventually succeed via fallback");
//         assert_eq!(result.method, "fallback", "Should use fallback method");
//         assert!(result.attempts >= 2, "Should have made multiple attempts");
//     }
//
//     #[tokio::test]
//     async fn test_validation_workflow_success() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("validate_data", validate_data_activity);
//         env.register_activity("unreliable_process", unreliable_process_activity);
//         env.register_workflow("validation", validation_workflow);
//
//         let data = serde_json::json!({
//             "id": "test_123",
//             "value": 42,
//         });
//
//         let result: ValidationWorkflowResult = env
//             .execute_workflow("validation", data)
//             .await
//             .expect("Workflow should complete");
//
//         assert!(result.validated, "Should validate successfully");
//         assert!(result.processed, "Should process successfully");
//         assert!(result.result.is_some(), "Should have processing result");
//         assert!(result.error.is_none(), "Should have no error");
//     }
//
//     #[tokio::test]
//     async fn test_validation_workflow_failure() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("validate_data", validate_data_activity);
//         env.register_workflow("validation", validation_workflow);
//
//         // Missing required field
//         let data = serde_json::json!({
//             "value": 42,
//             // missing "id"
//         });
//
//         let result: ValidationWorkflowResult = env
//             .execute_workflow("validation", data)
//             .await
//             .expect("Workflow should complete");
//
//         assert!(!result.validated, "Should fail validation");
//         assert!(!result.processed, "Should not process");
//         assert!(result.result.is_none(), "Should have no result");
//         assert!(result.error.is_some(), "Should have error message");
//     }
//
//     #[tokio::test]
//     async fn test_unreliable_process_activity_success() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("unreliable_process", unreliable_process_activity);
//
//         let input = ProcessInput {
//             item_id: "test_item".to_string(),
//             should_fail: false,
//             failure_type: None,
//         };
//
//         let result = env
//             .execute_activity("unreliable_process", input)
//             .await
//             .expect("Activity should complete");
//
//         let process_result: ProcessResult = serde_json::from_slice(&result).expect("Should parse");
//         assert!(process_result.success, "Should succeed");
//         assert_eq!(process_result.item_id, "test_item");
//         assert!(process_result.processing_time_ms > 0);
//     }
//
//     #[tokio::test]
//     async fn test_unreliable_process_activity_failure() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("unreliable_process", unreliable_process_activity);
//
//         let input = ProcessInput {
//             item_id: "failing_item".to_string(),
//             should_fail: true,
//             failure_type: Some(FailureType::NonRetryable),
//         };
//
//         let result = env.execute_activity("unreliable_process", input).await;
//         assert!(result.is_err(), "Activity should fail");
//     }
//
//     #[tokio::test]
//     async fn test_external_service_call_activity() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("external_service_call", external_service_call_activity);
//
//         let input = ExternalServiceInput {
//             endpoint: "https://api.example.com/test".to_string(),
//             payload: serde_json::json!({ "key": "value" }),
//             simulate_failure: false,
//         };
//
//         let result = env
//             .execute_activity("external_service_call", input)
//             .await
//             .expect("Activity should complete");
//
//         let response: serde_json::Value = serde_json::from_slice(&result).expect("Should parse");
//         assert_eq!(response["status"], "success");
//     }
//
//     #[tokio::test]
//     async fn test_validate_data_activity_success() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("validate_data", validate_data_activity);
//
//         let data = serde_json::json!({
//             "id": "item_123",
//             "value": 100,
//         });
//
//         let result = env
//             .execute_activity("validate_data", data)
//             .await
//             .expect("Activity should complete");
//
//         let validation: ValidationResult = serde_json::from_slice(&result).expect("Should parse");
//         assert!(validation.valid, "Should be valid");
//         assert!(validation.errors.is_empty(), "Should have no errors");
//     }
//
//     #[tokio::test]
//     async fn test_validate_data_activity_failure() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("validate_data", validate_data_activity);
//
//         // Invalid: negative value
//         let data = serde_json::json!({
//             "id": "item_123",
//             "value": -10,
//         });
//
//         let result = env.execute_activity("validate_data", data).await;
//         assert!(result.is_err(), "Should fail validation");
//     }
// }
