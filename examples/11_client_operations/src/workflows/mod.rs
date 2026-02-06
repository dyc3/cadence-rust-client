//! Workflow implementations for client operations example.
//!
//! This module provides workflows that demonstrate various client operations
//! like signals, queries, and long-running operations.

use cadence_workflow::context::WorkflowError;
use cadence_workflow::WorkflowContext;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

/// Input for the signal handling workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalWorkflowInput {
    pub workflow_id: String,
    pub initial_value: i32,
}

/// Output from the signal handling workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalWorkflowOutput {
    pub final_value: i32,
    pub signals_received: u32,
}

/// A workflow that demonstrates signal handling patterns
pub async fn signal_handling_workflow(
    ctx: &mut WorkflowContext,
    input: SignalWorkflowInput,
) -> Result<SignalWorkflowOutput, WorkflowError> {
    info!(
        "Starting signal_handling_workflow with initial value: {}",
        input.initial_value
    );

    let mut value = input.initial_value;
    let mut signals_received = 0u32;

    // Set up signal channels for different signal types
    let mut increment_channel = ctx.get_signal_channel("increment");
    let mut multiply_channel = ctx.get_signal_channel("multiply");

    // Process signals for up to 30 seconds or until we receive 5 signals
    let deadline = ctx.sleep(Duration::from_secs(30));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline => {
                info!("Signal processing deadline reached");
                break;
            }
            signal_data = increment_channel.recv() => {
                if let Some(data) = signal_data {
                    if let Ok(amount) = serde_json::from_slice::<i32>(&data) {
                        value += amount;
                        signals_received += 1;
                        info!("Incremented value by {} to {}", amount, value);
                    }
                }
                if signals_received >= 5 {
                    break;
                }
            }
            signal_data = multiply_channel.recv() => {
                if let Some(data) = signal_data {
                    if let Ok(factor) = serde_json::from_slice::<i32>(&data) {
                        value *= factor;
                        signals_received += 1;
                        info!("Multiplied value by {} to {}", factor, value);
                    }
                }
                if signals_received >= 5 {
                    break;
                }
            }
        }
    }

    info!(
        "Signal workflow completed. Final value: {}, Signals received: {}",
        value, signals_received
    );

    Ok(SignalWorkflowOutput {
        final_value: value,
        signals_received,
    })
}

/// Query response for the queryable workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusQueryResponse {
    pub current_step: String,
    pub progress_percent: u8,
    pub data: serde_json::Value,
}

/// Input for the queryable workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryableWorkflowInput {
    pub steps: Vec<String>,
}

/// Output from the queryable workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryableWorkflowOutput {
    pub completed_steps: Vec<String>,
    pub total_duration_ms: u64,
}

/// A workflow that demonstrates query handling
pub async fn queryable_workflow(
    ctx: &mut WorkflowContext,
    input: QueryableWorkflowInput,
) -> Result<QueryableWorkflowOutput, WorkflowError> {
    info!(
        "Starting queryable_workflow with {} steps",
        input.steps.len()
    );

    let start_time = ctx.current_time();
    let mut completed_steps = Vec::new();

    for (idx, step) in input.steps.iter().enumerate() {
        let progress = ((idx as f32 / input.steps.len() as f32) * 100.0) as u8;
        info!(
            "Executing step {}: {} ({}% complete)",
            idx + 1,
            step,
            progress
        );

        // Simulate step execution
        ctx.sleep(Duration::from_millis(100)).await;

        completed_steps.push(step.clone());
        info!("Completed step: {}", step);
    }

    let end_time = ctx.current_time();
    let duration = end_time.signed_duration_since(start_time);

    info!(
        "Queryable workflow completed {} steps in {:?}",
        completed_steps.len(),
        duration
    );

    Ok(QueryableWorkflowOutput {
        completed_steps,
        total_duration_ms: duration.num_milliseconds() as u64,
    })
}

/// Input for cancellable workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellableWorkflowInput {
    pub duration_seconds: u64,
    pub checkpoints: Vec<String>,
}

/// Output from cancellable workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellableWorkflowOutput {
    pub reached_checkpoint: String,
    pub was_cancelled: bool,
}

/// A workflow that demonstrates cancellation handling
pub async fn cancellable_workflow(
    ctx: &mut WorkflowContext,
    input: CancellableWorkflowInput,
) -> Result<CancellableWorkflowOutput, WorkflowError> {
    info!(
        "Starting cancellable_workflow for {} seconds",
        input.duration_seconds
    );

    let mut last_checkpoint = "start".to_string();

    for checkpoint in &input.checkpoints {
        info!("Reaching checkpoint: {}", checkpoint);

        // Check for cancellation
        if ctx.is_cancelled() {
            warn!("Workflow cancelled at checkpoint: {}", checkpoint);
            return Ok(CancellableWorkflowOutput {
                reached_checkpoint: last_checkpoint,
                was_cancelled: true,
            });
        }

        // Simulate work at checkpoint
        ctx.sleep(Duration::from_millis(100)).await;

        last_checkpoint = checkpoint.clone();
        info!("Passed checkpoint: {}", checkpoint);
    }

    info!("Cancellable workflow completed successfully");

    Ok(CancellableWorkflowOutput {
        reached_checkpoint: last_checkpoint,
        was_cancelled: false,
    })
}
