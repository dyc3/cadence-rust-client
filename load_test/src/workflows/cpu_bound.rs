// CPU-bound workflow types

use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;
use uber_cadence_core::ActivityOptions;
use uber_cadence_worker::registry::{Workflow, WorkflowError};
use uber_cadence_workflow::WorkflowContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuWorkflowInput {
    pub id: usize,
    pub iterations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuWorkflowOutput {
    pub id: usize,
    pub result: u64,
}

#[derive(Clone)]
pub struct CpuBoundWorkflow;

impl Workflow for CpuBoundWorkflow {
    fn execute(
        &self,
        ctx: WorkflowContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async move {
            let input_bytes =
                input.ok_or_else(|| WorkflowError::ExecutionFailed("Missing input".to_string()))?;
            let input: CpuWorkflowInput = serde_json::from_slice(&input_bytes)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            // Execute CPU-intensive activity
            let activity_input = crate::activities::cpu_intensive::CpuInput {
                id: input.id,
                iterations: input.iterations,
            };

            let result = ctx
                .execute_activity(
                    "cpu_intensive",
                    Some(serde_json::to_vec(&activity_input).unwrap()),
                    ActivityOptions {
                        schedule_to_close_timeout: Duration::from_secs(90),
                        start_to_close_timeout: Duration::from_secs(60),
                        ..Default::default()
                    },
                )
                .await
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            let output: crate::activities::cpu_intensive::CpuOutput =
                serde_json::from_slice(&result).map_err(|e| {
                    WorkflowError::ExecutionFailed(format!("Failed to parse result: {}", e))
                })?;

            let workflow_output = CpuWorkflowOutput {
                id: output.id,
                result: output.result,
            };

            serde_json::to_vec(&workflow_output)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))
        })
    }
}
