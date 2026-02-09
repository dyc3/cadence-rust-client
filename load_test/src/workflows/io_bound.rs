// IO-bound workflow types

use crabdance_core::ActivityOptions;
use crabdance_worker::registry::{Workflow, WorkflowError};
use crabdance_workflow::WorkflowContext;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoWorkflowInput {
    pub id: usize,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoWorkflowOutput {
    pub id: usize,
    pub completed: bool,
}

#[derive(Clone)]
pub struct IoBoundWorkflow;

impl Workflow for IoBoundWorkflow {
    fn execute(
        &self,
        ctx: WorkflowContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async move {
            let input_bytes =
                input.ok_or_else(|| WorkflowError::ExecutionFailed("Missing input".to_string()))?;
            let input: IoWorkflowInput = serde_json::from_slice(&input_bytes)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            // Execute IO-simulated activity
            let activity_input = crate::activities::io_simulated::IoInput {
                id: input.id,
                delay_ms: input.delay_ms,
            };

            let result = ctx
                .execute_activity(
                    "io_simulated",
                    Some(serde_json::to_vec(&activity_input).unwrap()),
                    ActivityOptions {
                        schedule_to_close_timeout: Duration::from_secs(180),
                        start_to_close_timeout: Duration::from_secs(120),
                        ..Default::default()
                    },
                )
                .await
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            let output: crate::activities::io_simulated::IoOutput = serde_json::from_slice(&result)
                .map_err(|e| {
                    WorkflowError::ExecutionFailed(format!("Failed to parse result: {}", e))
                })?;

            let workflow_output = IoWorkflowOutput {
                id: output.id,
                completed: output.completed,
            };

            serde_json::to_vec(&workflow_output)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))
        })
    }
}
