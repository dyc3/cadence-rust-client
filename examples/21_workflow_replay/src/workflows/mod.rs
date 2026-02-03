//! Workflow implementations for workflow replay example.

use cadence_workflow::context::WorkflowError;
use cadence_workflow::WorkflowContext;
use tracing::info;

/// A simple workflow for replay demonstration
pub async fn replay_demo_workflow(
    _ctx: &mut WorkflowContext,
    input: String,
) -> Result<String, WorkflowError> {
    info!("Starting replay_demo_workflow with input: {}", input);
    Ok(format!("Processed: {}", input))
}
