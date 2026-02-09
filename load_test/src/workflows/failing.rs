// Failing workflow types

use crabdance_core::{ActivityOptions, RetryPolicy};
use crabdance_worker::registry::{Workflow, WorkflowError};
use crabdance_workflow::WorkflowContext;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailingWorkflowInput {
    pub id: usize,
    pub failure_rate: f64,
    pub max_retries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailingWorkflowOutput {
    pub id: usize,
    pub attempt: u32,
    pub succeeded: bool,
}

#[derive(Clone)]
pub struct FailingWorkflow;

impl Workflow for FailingWorkflow {
    fn execute(
        &self,
        ctx: WorkflowContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async move {
            let input_bytes =
                input.ok_or_else(|| WorkflowError::ExecutionFailed("Missing input".to_string()))?;
            let input: FailingWorkflowInput = serde_json::from_slice(&input_bytes)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            // Execute failing activity with retries
            let activity_input = crate::activities::failing::FailingInput {
                id: input.id,
                failure_rate: input.failure_rate,
            };

            let result = ctx
                .execute_activity(
                    "failing",
                    Some(serde_json::to_vec(&activity_input).unwrap()),
                    ActivityOptions {
                        schedule_to_close_timeout: Duration::from_secs(60),
                        start_to_close_timeout: Duration::from_secs(30),
                        retry_policy: Some(RetryPolicy {
                            initial_interval: Duration::from_secs(1),
                            backoff_coefficient: 2.0,
                            maximum_interval: Duration::from_secs(10),
                            maximum_attempts: input.max_retries as i32,
                            non_retryable_error_types: vec![],
                            expiration_interval: Duration::from_secs(0),
                        }),
                        ..Default::default()
                    },
                )
                .await;

            let workflow_output = match result {
                Ok(data) => {
                    let output: crate::activities::failing::FailingOutput =
                        serde_json::from_slice(&data).map_err(|e| {
                            WorkflowError::ExecutionFailed(format!("Failed to parse result: {}", e))
                        })?;
                    FailingWorkflowOutput {
                        id: output.id,
                        attempt: output.attempt,
                        succeeded: true,
                    }
                }
                Err(_) => FailingWorkflowOutput {
                    id: input.id,
                    attempt: input.max_retries as u32,
                    succeeded: false,
                },
            };

            serde_json::to_vec(&workflow_output)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))
        })
    }
}
