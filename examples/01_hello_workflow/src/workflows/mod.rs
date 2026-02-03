//! Workflow implementations for the hello workflow example.

use crate::activities::{HelloInput, HelloOutput};
use cadence_core::ActivityOptions;
use cadence_workflow::context::WorkflowError;
use cadence_workflow::WorkflowContext;
use std::time::Duration;
use tracing::{info, warn};

/// A simple workflow that orchestrates a greeting activity
pub async fn hello_workflow(
    ctx: &mut WorkflowContext,
    input: HelloInput,
) -> Result<HelloOutput, WorkflowError> {
    info!("Starting hello_workflow for: {}", input.name);

    // Get workflow info
    let workflow_info = ctx.workflow_info();
    info!(
        "Workflow ID: {}, Run ID: {}",
        workflow_info.workflow_execution.workflow_id, workflow_info.workflow_execution.run_id
    );

    // Execute the greeting activity
    let activity_options = ActivityOptions {
        task_list: workflow_info.task_list.clone(),
        start_to_close_timeout: Duration::from_secs(30),
        ..Default::default()
    };

    let result = ctx
        .execute_activity(
            "format_greeting",
            Some(serde_json::to_vec(&input).unwrap()),
            activity_options,
        )
        .await?;

    // Parse the result - result is Vec<u8>, not Option<Vec<u8>>
    let output: HelloOutput = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

    info!("Workflow completed with message: {}", output.message);
    Ok(output)
}

/// A workflow that demonstrates signal handling
pub async fn greeting_with_signal_workflow(
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
    let result = ctx
        .execute_activity(
            "format_greeting",
            Some(serde_json::to_vec(&input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    // Parse the result - result is Vec<u8>, not Option<Vec<u8>>
    let output: HelloOutput = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

    Ok(output)
}
