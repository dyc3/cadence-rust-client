//! # Example 01: Hello Workflow
//!
//! This example demonstrates basic workflow and activity definitions.
//!
//! ## Features Demonstrated
//!
//! - Defining a simple workflow
//! - Defining an activity with heartbeats
//! - Input/output serialization with serde
//! - Using TestWorkflowEnvironment for testing
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p hello_workflow
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p hello_workflow
//! ```

use hello_workflow::{hello_workflow, format_greeting_activity, HelloInput, HelloOutput, GreetingType};
use examples_common::tracing_setup::init_tracing;
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    init_tracing();

    println!("\n=== Cadence Rust Client - Hello Workflow Example ===\n");
    println!("This example demonstrates:");
    println!("1. Defining workflows and activities");
    println!("2. Serializing/deserializing workflow inputs/outputs");
    println!("3. Workflow signals");
    println!("4. Activity execution");
    println!();

    // Example serialization demonstration
    let input = HelloInput {
        name: "Alice".to_string(),
        greeting_type: GreetingType::Excited,
    };

    let serialized = serde_json::to_vec(&input)?;
    println!("Example input serialized: {:?}", String::from_utf8_lossy(&serialized));

    let deserialized: HelloInput = serde_json::from_slice(&serialized)?;
    println!("Deserialized: {:?}", deserialized);
    
    println!("\nTo run the full workflow test:");
    println!("  cargo test -p hello_workflow");
    
    println!("\nExample completed successfully!");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::{TestWorkflowEnvironment, TestWorkflowContext};
//     use cadence_workflow::context::WorkflowError;
//     use std::time::Duration;
// 
//     // Test wrapper for hello_workflow that uses TestWorkflowContext
//     async fn test_hello_workflow(
//         ctx: &mut TestWorkflowContext,
//         input: HelloInput,
//     ) -> Result<HelloOutput, WorkflowError> {
//         use cadence_core::ActivityOptions;
//         use tracing::{info};
// 
//         info!("Starting hello_workflow for: {}", input.name);
// 
//         let workflow_info = ctx.workflow_info();
//         info!(
//             "Workflow ID: {}, Run ID: {}",
//             workflow_info.workflow_execution.workflow_id,
//             workflow_info.workflow_execution.run_id
//         );
// 
//         let activity_options = ActivityOptions {
//             task_list: workflow_info.task_list.clone(),
//             start_to_close_timeout: Duration::from_secs(30),
//             ..Default::default()
//         };
// 
//         let result = ctx
//             .execute_activity(
//                 "format_greeting",
//                 Some(serde_json::to_vec(&input).unwrap()),
//                 activity_options,
//             )
//             .await?;
// 
//         let output: HelloOutput = serde_json::from_slice(&result)
//             .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
// 
//         info!("Workflow completed with message: {}", output.message);
//         Ok(output)
//     }
// 
//     // Test wrapper for greeting_with_signal_workflow that uses TestWorkflowContext
//     async fn test_greeting_with_signal_workflow(
//         ctx: &mut TestWorkflowContext,
//         initial_input: HelloInput,
//     ) -> Result<HelloOutput, WorkflowError> {
//         use cadence_core::ActivityOptions;
//         use tracing::{info, warn};
// 
//         info!("Starting greeting_with_signal_workflow");
// 
//         let mut signal_channel = ctx.get_signal_channel("greeting_style");
// 
//         let greeting_type = tokio::select! {
//             signal_data = signal_channel.recv() => {
//                 if let Some(data) = signal_data {
//                     serde_json::from_slice(&data).unwrap_or(initial_input.greeting_type)
//                 } else {
//                     initial_input.greeting_type
//                 }
//             }
//             _ = ctx.sleep(Duration::from_secs(10)) => {
//                 warn!("No signal received within 10 seconds, using default");
//                 initial_input.greeting_type
//             }
//         };
// 
//         let input = HelloInput {
//             greeting_type,
//             ..initial_input
//         };
// 
//         let result = ctx
//             .execute_activity(
//                 "format_greeting",
//                 Some(serde_json::to_vec(&input).unwrap()),
//                 ActivityOptions::default(),
//             )
//             .await?;
// 
//         let output: HelloOutput = serde_json::from_slice(&result)
//             .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
// 
//         Ok(output)
//     }
// 
//     #[tokio::test]
//     async fn test_hello_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
// 
//         // Register the activity
//         env.register_activity("format_greeting", format_greeting_activity);
// 
//         // Register the test workflow with TestWorkflowContext
//         env.register_workflow("hello_workflow", test_hello_workflow);
// 
//         let input = HelloInput {
//             name: "Test".to_string(),
//             greeting_type: GreetingType::Casual,
//         };
// 
//         // Execute the workflow
//         let result = env
//             .execute_workflow("hello_workflow", input)
//             .await
//             .expect("Workflow should complete successfully");
// 
//         assert!(
//             result.message.contains("Test"),
//             "Output should contain the name"
//         );
//         assert!(
//             result.message.contains("Hey"),
//             "Casual greeting should use 'Hey'"
//         );
//     }
// 
//     #[tokio::test]
//     async fn test_greeting_types() {
//         let test_cases = vec![
//             (GreetingType::Formal, "Hello,"),
//             (GreetingType::Casual, "Hey"),
//             (GreetingType::Excited, "WOW"),
//         ];
// 
//         for (greeting_type, expected) in test_cases {
//             let mut env = TestWorkflowEnvironment::new();
//             env.register_activity("format_greeting", format_greeting_activity);
//             env.register_workflow("hello_workflow", test_hello_workflow);
// 
//             let input = HelloInput {
//                 name: "Tester".to_string(),
//                 greeting_type,
//             };
// 
//             let result = env
//                 .execute_workflow("hello_workflow", input)
//                 .await
//                 .expect("Workflow should complete");
// 
//             assert!(
//                 result.message.contains(expected),
//                 "Expected '{}' in message: {}",
//                 expected,
//                 result.message
//             );
//         }
//     }
// 
//     #[test]
//     fn test_serialization() {
//         let output = HelloOutput {
//             message: "Hello, World!".to_string(),
//             timestamp: 1234567890,
//         };
// 
//         let serialized = serde_json::to_vec(&output).unwrap();
//         let deserialized: HelloOutput = serde_json::from_slice(&serialized).unwrap();
// 
//         assert_eq!(output.message, deserialized.message);
//         assert_eq!(output.timestamp, deserialized.timestamp);
//     }
// 
//     #[tokio::test]
//     async fn test_signal_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("format_greeting", format_greeting_activity);
//         env.register_workflow("greeting_with_signal", test_greeting_with_signal_workflow);
// 
//         let initial_input = HelloInput {
//             name: "SignalTest".to_string(),
//             greeting_type: GreetingType::Formal,
//         };
// 
//         // Send a signal to change greeting type to Excited
//         let signal_data = serde_json::to_vec(&GreetingType::Excited).unwrap();
//         env.signal_workflow("greeting_style", signal_data);
// 
//         let result = env
//             .execute_workflow("greeting_with_signal", initial_input)
//             .await
//             .expect("Workflow should complete");
// 
//         // The signal should have changed the greeting type to Excited
//         assert!(
//             result.message.contains("WOW"),
//             "Signal should have changed greeting to Excited: {}",
//             result.message
//         );
//     }
// }
