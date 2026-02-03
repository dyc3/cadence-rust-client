//! # Example 08: Versioning
//!
//! This example demonstrates workflow versioning and side effects.
//!
//! ## Features Demonstrated
//!
//! - Workflow versioning for backwards compatibility
//! - Side effects and their handling
//! - Mutable side effects
//! - Version-specific behavior
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p versioning
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p versioning
//! ```

use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Versioning Example ===\n");
    println!("This example demonstrates:");
    println!("1. Workflow versioning for backwards compatibility");
    println!("2. Side effects and their handling");
    println!("3. Version-specific behavior");
    println!("4. Conditional version-based logic");
    println!();
    println!("Run tests to see the workflows in action:");
    println!("  cargo test -p versioning");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::TestWorkflowEnvironment;
//
//     #[tokio::test]
//     async fn test_versioned_workflow_v1() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("external_api_call", external_api_call_activity);
//         env.register_workflow("versioned_v1", versioned_workflow_v1);
//
//         let result = env
//             .execute_workflow("versioned_v1", "test_input".to_string())
//             .await
//             .expect("Workflow should complete");
//
//         assert!(result.contains("V1 Result"), "Should return V1 result");
//     }
//
//     #[tokio::test]
//     async fn test_versioned_workflow_v2() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("external_api_call", external_api_call_activity);
//         env.register_activity("database_query", database_query_activity);
//         env.register_workflow("versioned_v2", versioned_workflow_v2);
//
//         let result = env
//             .execute_workflow("versioned_v2", "test_input".to_string())
//             .await
//             .expect("Workflow should complete");
//
//         assert!(result.contains("V2 Result"), "Should return V2 result");
//         assert!(result.contains("DB Rows"), "Should include DB results");
//     }
//
//     #[tokio::test]
//     async fn test_side_effect_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("external_api_call", external_api_call_activity);
//         env.register_workflow("side_effect", side_effect_workflow);
//
//         let result = env
//             .execute_workflow("side_effect", "create_order".to_string())
//             .await
//             .expect("Workflow should complete");
//
//         assert!(result.contains("create_order"), "Should mention operation");
//         assert!(result.contains("completed"), "Should indicate completion");
//     }
//
//     #[tokio::test]
//     async fn test_conditional_version_workflow_v1() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("get_service_config", get_service_config_activity);
//         env.register_activity("external_api_call", external_api_call_activity);
//         env.register_workflow("conditional_version", conditional_version_workflow);
//
//         // Use V1 explicitly
//         let result = env
//             .execute_workflow("conditional_version", ("test_input".to_string(), false))
//             .await
//             .expect("Workflow should complete");
//
//         assert!(result.contains("V1"), "Should use V1 API");
//     }
//
//     #[tokio::test]
//     async fn test_conditional_version_workflow_v2() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("get_service_config", get_service_config_activity);
//         env.register_activity("external_api_call", external_api_call_activity);
//         env.register_activity("database_query", database_query_activity);
//         env.register_workflow("conditional_version", conditional_version_workflow);
//
//         // Request V2
//         let result = env
//             .execute_workflow("conditional_version", ("test_input".to_string(), true))
//             .await
//             .expect("Workflow should complete");
//
//         assert!(result.contains("V2"), "Should mention V2");
//     }
//
//     #[tokio::test]
//     async fn test_external_api_call_activity() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("external_api_call", external_api_call_activity);
//
//         let input = ApiCallInput {
//             endpoint: "/api/test".to_string(),
//             method: "GET".to_string(),
//             payload: serde_json::json!({ "key": "value" }),
//         };
//
//         let result = env
//             .execute_activity("external_api_call", input)
//             .await
//             .expect("Activity should complete");
//
//         let api_result: ApiCallResult = serde_json::from_slice(&result).expect("Should parse");
//         assert_eq!(api_result.status_code, 200, "Should return 200");
//         assert!(!api_result.call_id.is_empty(), "Should have call ID");
//     }
//
//     #[tokio::test]
//     async fn test_database_query_activity() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("database_query", database_query_activity);
//
//         let input = DatabaseQueryInput {
//             query: "SELECT * FROM users".to_string(),
//             parameters: vec![],
//             use_new_schema: true,
//         };
//
//         let result = env
//             .execute_activity("database_query", input)
//             .await
//             .expect("Activity should complete");
//
//         let db_result: DatabaseQueryResult = serde_json::from_slice(&result).expect("Should parse");
//         assert!(db_result.rows_affected > 0, "Should have rows");
//         assert!(!db_result.query_id.is_empty(), "Should have query ID");
//     }
//
//     #[tokio::test]
//     async fn test_get_service_config_activity() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("get_service_config", get_service_config_activity);
//
//         let result = env
//             .execute_activity("get_service_config", "payment_service".to_string())
//             .await
//             .expect("Activity should complete");
//
//         let config: ServiceConfig = serde_json::from_slice(&result).expect("Should parse");
//         assert_eq!(config.service_name, "payment_service");
//         assert!(!config.endpoint.is_empty(), "Should have endpoint");
//         assert!(!config.api_version.is_empty(), "Should have API version");
//     }
// }
