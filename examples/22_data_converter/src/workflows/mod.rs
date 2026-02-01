//! Workflow implementations for data converter example.

use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use tracing::info;

/// A workflow demonstrating custom serialization
pub async fn data_converter_workflow(
    ctx: &mut WorkflowContext,
    input: String,
) -> Result<String, WorkflowError> {
    info!("Starting data_converter_workflow with input: {}", input);
    Ok(format!("Processed: {}", input))
}
