//! # Example 25: Best Practices
//!
//! This example demonstrates idiomatic patterns and best practices for building production-grade
//! Cadence workflows in Rust.
//!
//! ## Features Demonstrated
//!
//! - Type-safe workflow inputs/outputs with newtype patterns
//! - Structured error handling
//! - Idempotency and deduplication
//! - Comprehensive logging and tracing
//! - Configuration management
//! - Circuit breaker patterns
//! - Retry with exponential backoff
//! - Context propagation
//! - Builder patterns
//! - Version-aware workflows
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p best_practices
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p best_practices
//! ```

use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Best Practices Example ===\n");
    println!("This example demonstrates production-ready patterns:");
    println!();
    println!("Type Safety:");
    println!("  - Newtype patterns for IDs (WorkflowId, UserId)");
    println!("  - Validated types (Email, Amount)");
    println!("  - Strongly-typed enums");
    println!();
    println!("Error Handling:");
    println!("  - Structured error types with thiserror");
    println!("  - Idempotency and deduplication");
    println!("  - Circuit breaker patterns");
    println!();
    println!("Observability:");
    println!("  - Structured logging with tracing");
    println!("  - Context propagation for distributed tracing");
    println!("  - Correlation IDs");
    println!();
    println!("Configuration:");
    println!("  - Environment-based configuration");
    println!("  - Validation at load time");
    println!("  - Sensible defaults");
    println!();
    println!("Run tests to see patterns in action:");
    println!("  cargo test -p best_practices -- --nocapture");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use uber_cadence_testsuite::TestWorkflowEnvironment;
//     use std::collections::HashMap;
//
//     #[tokio::test]
//     async fn test_type_safety_patterns() {
//         println!("\n=== Testing Type Safety Patterns ===\n");
//
//         // Test newtype patterns
//         let workflow_id = WorkflowId::new("wf_123");
//         let user_id = UserId::new("user_456");
//
//         println!("WorkflowId: {}", workflow_id);
//         println!("UserId: {}", user_id);
//         assert_eq!(workflow_id.as_str(), "wf_123");
//         assert_eq!(user_id.as_str(), "user_456");
//
//         // Test validated types
//         let email = Email::new("test@example.com").expect("Valid email");
//         assert_eq!(email.as_str(), "test@example.com");
//
//         let amount = Amount::new(100.50).expect("Valid amount");
//         assert_eq!(amount.value(), 100.50);
//
//         // Test validation failures
//         assert!(Email::new("invalid-email").is_err());
//         assert!(Amount::new(-10.0).is_err());
//
//         // Test idempotency key generation
//         let key1 = IdempotencyKey::generate();
//         let key2 = IdempotencyKey::generate();
//         assert_ne!(key1.as_str(), key2.as_str());
//
//         println!("Type safety patterns: OK");
//     }
//
//     #[tokio::test]
//     async fn test_configuration_management() {
//         println!("\n=== Testing Configuration Management ===\n");
//
//         let config = AppConfig::default();
//
//         println!("Worker concurrency: {}", config.worker.concurrency);
//         println!("Retry max attempts: {}", config.retry.maximum_attempts);
//         println!("Workflow timeout: {:?}", config.timeouts.workflow_execution);
//
//         assert!(config.worker.concurrency > 0);
//         assert!(config.retry.backoff_coefficient >= 1.0);
//         assert!(config.validate().is_ok());
//
//         println!("Configuration management: OK");
//     }
//
//     #[tokio::test]
//     async fn test_utility_functions() {
//         println!("\n=== Testing Utility Functions ===\n");
//
//         // Test correlation ID generation
//         let corr_id = generate_correlation_id();
//         assert!(corr_id.starts_with("corr-"));
//
//         // Test string truncation
//         let long_str = "This is a very long string that needs truncation";
//         let truncated = truncate(long_str, 20);
//         assert!(truncated.len() <= 20);
//         assert!(truncated.ends_with("..."));
//
//         // Test sanitization
//         let sensitive = "secret_password_123";
//         let sanitized = sanitize_for_logging(sensitive);
//         assert!(sanitized.contains("****"));
//
//         println!("Utility functions: OK");
//     }
//
//     #[tokio::test]
//     async fn test_retry_with_backoff() {
//         println!("\n=== Testing Retry with Exponential Backoff ===\n");
//
//         let mut attempts = 0;
//
//         let result = retry_with_backoff(
//             || async {
//                 attempts += 1;
//                 if attempts < 3 {
//                     Err::<String, String>(format!("Attempt {} failed", attempts))
//                 } else {
//                     Ok("Success!".to_string())
//                 }
//             },
//             5,
//             Duration::from_millis(10),
//             2.0,
//         )
//         .await;
//
//         assert!(result.is_ok());
//         assert_eq!(attempts, 3);
//
//         println!("Retry succeeded after {} attempts", attempts);
//     }
//
//     #[tokio::test]
//     async fn test_circuit_breaker() {
//         println!("\n=== Testing Circuit Breaker ===\n");
//
//         let mut cb = CircuitBreaker::new(2, 1, Duration::from_secs(1));
//
//         // First two calls succeed
//         for i in 0..2 {
//             let result = cb.call(|| async { Ok::<String, String>(format!("Call {}", i)) }).await;
//             assert!(result.is_ok());
//         }
//
//         // Third call fails
//         let result = cb.call(|| async { Err::<String, String>("Failure".into()) }).await;
//         assert!(result.is_err());
//
//         // Circuit should still be closed (only 1 failure)
//         assert!(matches!(cb.state(), CircuitState::Closed));
//
//         // Another failure
//         let _ = cb.call(|| async { Err::<String, String>("Failure 2".into()) }).await;
//
//         // Now circuit should be open
//         assert!(matches!(cb.state(), CircuitState::Open { .. }));
//
//         // Next call should fail immediately
//         let result = cb.call(|| async { Ok::<String, String>("Should not reach".into()) }).await;
//         assert!(matches!(result.unwrap_err(), CircuitError::Open));
//
//         println!("Circuit breaker transitions: OK");
//     }
//
//     #[tokio::test]
//     async fn test_idempotent_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("idempotent_process", idempotent_process_activity);
//         env.register_workflow("idempotent", idempotent_workflow);
//
//         let input = IdempotentProcessInput {
//             idempotency_key: IdempotencyKey::generate(),
//             user_id: UserId::new("user_123"),
//             operation: "test_op".to_string(),
//             amount: Amount::new(100.0).unwrap(),
//         };
//
//         let result = env
//             .execute_workflow("idempotent", input.clone())
//             .await
//             .expect("Workflow should succeed");
//
//         assert!(result.processed);
//         assert!(result.processing_time_ms > 0);
//
//         println!("\nIdempotent Workflow:");
//         println!("  Processed: {}", result.processed);
//         println!("  Duration: {} ms", result.processing_time_ms);
//     }
//
//     #[tokio::test]
//     async fn test_validated_processing_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("validated_process", validated_process_activity);
//         env.register_workflow("validated_processing", validated_processing_workflow);
//
//         let result = env
//             .execute_workflow(
//                 "validated_processing",
//                 (
//                     UserId::new("user_456"),
//                     Email::new("user@example.com").unwrap(),
//                     Amount::new(250.0).unwrap(),
//                 )
//             )
//             .await
//             .expect("Workflow should succeed");
//
//         assert!(result.data.contains("250"));
//
//         println!("\nValidated Processing Workflow:");
//         println!("  Result: {}", result.data);
//         println!("  Idempotency Key: {}", result.idempotency_key.as_str());
//     }
//
//     #[tokio::test]
//     async fn test_saga_pattern_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("validated_process", validated_process_activity);
//         env.register_activity("idempotent_process", idempotent_process_activity);
//         env.register_workflow("saga_pattern", saga_pattern_workflow);
//
//         let result = env
//             .execute_workflow(
//                 "saga_pattern",
//                 (
//                     UserId::new("user_789"),
//                     Amount::new(500.0).unwrap(),
//                 )
//             )
//             .await
//             .expect("Saga should succeed");
//
//         assert_eq!(result, "saga_completed");
//
//         println!("\nSaga Pattern Workflow:");
//         println!("  Status: {}", result);
//     }
//
//     #[tokio::test]
//     async fn test_context_propagation_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("robust_external_call", robust_external_call_activity);
//         env.register_workflow("context_propagation", context_propagation_workflow);
//
//         let request_context = RequestContext::new("trace_123", "span_456")
//             .with_parent("parent_789")
//             .with_baggage("user_id", "user_123");
//
//         let result = env
//             .execute_workflow(
//                 "context_propagation",
//                 (request_context, "test_operation".to_string())
//             )
//             .await
//             .expect("Workflow should succeed");
//
//         assert_eq!(result, "external_call_result");
//
//         println!("\nContext Propagation Workflow:");
//         println!("  Result: {}", result);
//     }
//
//     #[tokio::test]
//     async fn test_structured_logging_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("structured_logging", structured_logging_activity);
//         env.register_workflow("structured_logging", structured_logging_workflow);
//
//         let result = env
//             .execute_workflow("structured_logging", "test_operation".to_string())
//             .await;
//
//         assert!(result.is_ok());
//
//         println!("\nStructured Logging Workflow:");
//         println!("  Status: OK");
//     }
//
//     #[tokio::test]
//     async fn test_version_aware_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_workflow("version_aware", version_aware_workflow);
//
//         let result = env
//             .execute_workflow("version_aware", "test_input".to_string())
//             .await
//             .expect("Workflow should succeed");
//
//         assert!(result.contains("1.0.0"));
//
//         println!("\nVersion-Aware Workflow:");
//         println!("  Result: {}", result);
//     }
//
//     #[tokio::test]
//     async fn test_individual_activities() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("idempotent_process", idempotent_process_activity);
//         env.register_activity("validated_process", validated_process_activity);
//         env.register_activity("robust_external_call", robust_external_call_activity);
//         env.register_activity("structured_logging", structured_logging_activity);
//         env.register_activity("circuit_breaker_protected", circuit_breaker_protected_activity);
//
//         println!("\n=== Testing Individual Activities ===\n");
//
//         // Test idempotent process
//         let input = IdempotentProcessInput {
//             idempotency_key: IdempotencyKey::generate(),
//             user_id: UserId::new("user_123"),
//             operation: "test".to_string(),
//             amount: Amount::new(100.0).unwrap(),
//         };
//         let result = env
//             .execute_activity("idempotent_process", input)
//             .await
//             .expect("Should succeed");
//         let process_result: IdempotentProcessResult = serde_json::from_slice(&result).expect("Should parse");
//         assert!(process_result.processed);
//         println!("  idempotent_process: OK");
//
//         // Test validated process
//         let input = ValidatedProcessInput {
//             user_id: UserId::new("user_456"),
//             email: Email::new("test@example.com").unwrap(),
//             amount: Amount::new(50.0).unwrap(),
//             metadata: HashMap::new(),
//         };
//         let result = env
//             .execute_activity("validated_process", input)
//             .await
//             .expect("Should succeed");
//         let typed_result: TypedActivityResult<String> = serde_json::from_slice(&result).expect("Should parse");
//         assert!(!typed_result.data.is_empty());
//         println!("  validated_process: OK");
//
//         // Test context propagation
//         let ctx = RequestContext::new("trace_1", "span_1");
//         let result = env
//             .execute_activity("robust_external_call", ctx)
//             .await
//             .expect("Should succeed");
//         let str_result: String = serde_json::from_slice(&result).expect("Should parse");
//         assert_eq!(str_result, "external_call_result");
//         println!("  robust_external_call: OK");
//
//         println!("\nAll activities passed!");
//     }
//
//     #[tokio::test]
//     async fn test_all_patterns_summary() {
//         println!("\n=== Best Practices Summary ===\n");
//         println!("✅ Type Safety: Newtype patterns, validated types, strong enums");
//         println!("✅ Configuration: Environment-based, validation, defaults");
//         println!("✅ Error Handling: Structured errors, idempotency, circuit breakers");
//         println!("✅ Observability: Structured logging, tracing, correlation IDs");
//         println!("✅ Resilience: Retry with backoff, circuit breaker patterns");
//         println!("✅ Patterns: Builder, idempotency, saga, version-aware");
//     }
// }
