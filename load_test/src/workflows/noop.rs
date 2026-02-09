// No-op workflow types - tests pure workflow overhead

use crabdance_worker::registry::{Workflow, WorkflowError};
use crabdance_workflow::WorkflowContext;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoopInput {
    pub id: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoopOutput {
    pub id: usize,
    pub completed: bool,
}

#[derive(Clone)]
pub struct NoopWorkflow;

impl Workflow for NoopWorkflow {
    fn execute(
        &self,
        _ctx: WorkflowContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async move {
            let input_bytes =
                input.ok_or_else(|| WorkflowError::ExecutionFailed("Missing input".to_string()))?;
            let input: NoopInput = serde_json::from_slice(&input_bytes)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            // Noop workflow - immediately return
            let output = NoopOutput {
                id: input.id,
                completed: true,
            };

            serde_json::to_vec(&output).map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))
        })
    }
}
