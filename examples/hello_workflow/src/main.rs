//! Hello World Workflow Example
//!
//! This example demonstrates:
//! 1. Defining a simple workflow
//! 2. Defining an activity
//! 3. Setting up a worker
//! 4. Running the workflow

use cadence_activity::{ActivityContext, ActivityError, ActivityInfo};
use cadence_core::WorkflowExecution;
use cadence_workflow::{WorkflowContext, WorkflowError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// Input for the hello workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HelloInput {
    name: String,
    greeting_type: GreetingType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum GreetingType {
    Formal,
    Casual,
    Excited,
}

/// Output from the hello workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HelloOutput {
    message: String,
    timestamp: i64,
}

/// A simple activity that formats a greeting
async fn format_greeting_activity(ctx: &ActivityContext, input: HelloInput) -> Result<HelloOutput, ActivityError> {
    info!("Executing format_greeting_activity for: {}", input.name);
    
    // Get activity info
    let info = ctx.get_info();
    info!("Activity ID: {}, Attempt: {}", info.activity_id, info.attempt);
    
    // Simulate some work
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let message = match input.greeting_type {
        GreetingType::Formal => format!("Hello, {}. Welcome!", input.name),
        GreetingType::Casual => format!("Hey {}! What's up?", input.name),
        GreetingType::Excited => format!("WOW! It's {}! Amazing to see you!", input.name),
    };
    
    // Record heartbeat to show progress
    ctx.record_heartbeat(None);
    
    Ok(HelloOutput {
        message,
        timestamp: chrono::Utc::now().timestamp(),
    })
}

/// A workflow that orchestrates the greeting
async fn hello_workflow(ctx: &mut WorkflowContext, input: HelloInput) -> Result<HelloOutput, WorkflowError> {
    info!("Starting hello_workflow for: {}", input.name);
    
    // Get workflow info
    let info = ctx.workflow_info();
    info!(
        "Workflow ID: {}, Run ID: {}",
        info.workflow_execution.workflow_id,
        info.workflow_execution.run_id
    );
    
    // Execute the greeting activity
    let activity_result = ctx.execute_activity(
        "format_greeting",
        Some(serde_json::to_vec(&input).unwrap()),
        cadence_core::ActivityOptions {
            task_list: info.task_list.clone(),
            start_to_close_timeout: Duration::from_secs(30),
            ..Default::default()
        },
    ).await?;
    
    // Parse the result
    let output: HelloOutput = serde_json::from_slice(&activity_result.unwrap_or_default())
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    info!("Workflow completed with message: {}", output.message);
    
    Ok(output)
}

/// A workflow that demonstrates signals
async fn greeting_with_signal_workflow(
    ctx: &mut WorkflowContext,
    initial_input: HelloInput,
) -> Result<HelloOutput, WorkflowError> {
    info!("Starting greeting_with_signal_workflow");
    
    // Set up a signal handler
    let mut signal_channel = ctx.get_signal_channel("greeting_style");
    
    // Wait for a signal or timeout
    let greeting_type = tokio::select! {
        signal_data = signal_channel.recv() => {
            if let Some(data) = signal_data {
                serde_json::from_slice(&data).unwrap_or(initial_input.greeting_type)
            } else {
                initial_input.greeting_type
            }
        }
        _ = ctx.sleep(Duration::from_secs(10)) => {
            warn!("No signal received within 10 seconds, using default");
            initial_input.greeting_type
        }
    };
    
    let input = HelloInput {
        greeting_type,
        ..initial_input
    };
    
    // Execute the activity
    let activity_result = ctx.execute_activity(
        "format_greeting",
        Some(serde_json::to_vec(&input).unwrap()),
        cadence_core::ActivityOptions::default(),
    ).await?;
    
    let output: HelloOutput = serde_json::from_slice(&activity_result.unwrap_or_default())
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    Ok(output)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting Hello Workflow Example");
    
    // In a real application, you would:
    // 1. Connect to the Cadence server
    // 2. Create a worker
    // 3. Register workflows and activities
    // 4. Start the worker
    // 5. Use the client to start workflows
    
    println!("\n=== Cadence Rust Client - Hello Workflow Example ===\n");
    println!("This example demonstrates:");
    println!("1. Defining workflows and activities");
    println!("2. Serializing/deserializing workflow inputs/outputs");
    println!("3. Workflow signals");
    println!("4. Activity execution");
    println!();
    println!("To run this example with a real Cadence server:");
    println!("  1. Start Cadence server: docker-compose up -d");
    println!("  2. Run the worker: cargo run --example worker");
    println!("  3. Start a workflow: cargo run --example client");
    
    // Example serialization
    let input = HelloInput {
        name: "Alice".to_string(),
        greeting_type: GreetingType::Excited,
    };
    
    let serialized = serde_json::to_vec(&input)?;
    println!("\nExample input serialized: {:?}", String::from_utf8_lossy(&serialized));
    
    let deserialized: HelloInput = serde_json::from_slice(&serialized)?;
    println!("Deserialized: {:?}", deserialized);
    
    info!("Example completed successfully!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadence_testsuite::TestWorkflowEnvironment;
    
    #[tokio::test]
    async fn test_hello_workflow() {
        let env = TestWorkflowEnvironment::new();
        
        let input = HelloInput {
            name: "Test".to_string(),
            greeting_type: GreetingType::Casual,
        };
        
        // In a real test, you would register the activity and workflow
        // and then execute the workflow in the test environment
        let serialized = serde_json::to_vec(&input).unwrap();
        let deserialized: HelloInput = serde_json::from_slice(&serialized).unwrap();
        
        assert_eq!(input.name, deserialized.name);
        assert!(matches!(deserialized.greeting_type, GreetingType::Casual));
    }
    
    #[test]
    fn test_serialization() {
        let output = HelloOutput {
            message: "Hello, World!".to_string(),
            timestamp: 1234567890,
        };
        
        let serialized = serde_json::to_vec(&output).unwrap();
        let deserialized: HelloOutput = serde_json::from_slice(&serialized).unwrap();
        
        assert_eq!(output.message, deserialized.message);
        assert_eq!(output.timestamp, deserialized.timestamp);
    }
}
