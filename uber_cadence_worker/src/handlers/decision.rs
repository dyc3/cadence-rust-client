use crate::executor::workflow::WorkflowExecutor;
use futures::FutureExt;
use std::sync::Arc;
use tracing::{error, info};
use uber_cadence_core::{CadenceError, TransportError};
use uber_cadence_proto::workflow_service::*;

/// Decision task handler
pub struct DecisionTaskHandler {
    service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync>,
    executor: Arc<WorkflowExecutor>,
    identity: String,
}

impl DecisionTaskHandler {
    pub fn new(
        service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync>,
        executor: Arc<WorkflowExecutor>,
        identity: String,
    ) -> Self {
        Self {
            service,
            executor,
            identity,
        }
    }

    pub async fn handle(
        &self,
        task: PollForDecisionTaskResponse,
    ) -> Result<RespondDecisionTaskCompletedResponse, CadenceError> {
        if task.task_token.is_empty() {
            return Err(CadenceError::Other("Empty task token received".into()));
        }

        // Execute workflow logic with panic protection
        let executor = self.executor.clone();
        let task_clone = task.clone();

        let result = std::panic::AssertUnwindSafe(async move {
            executor.execute_decision_task(task_clone).await
        })
        .catch_unwind()
        .await;

        let execution_result = match result {
            Ok(res) => res,
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic".to_string()
                };
                error!(panic_msg = %panic_msg, "workflow panicked");
                Err(CadenceError::Other(format!(
                    "Workflow panic: {}",
                    panic_msg
                )))
            }
        };

        match execution_result {
            Ok((decisions, query_results)) => {
                // Respond with decisions
                info!(
                    decision_count = decisions.len(),
                    "responding with decisions"
                );
                let response = self
                    .service
                    .respond_decision_task_completed(RespondDecisionTaskCompletedRequest {
                        task_token: task.task_token,
                        decisions,
                        identity: self.identity.clone(),
                        execution_context: None,
                        binary_checksum: "".to_string(),
                        query_results: Some(query_results),
                        force_create_new_decision_task: false,
                        sticky_attributes: None,
                        return_new_decision_task: false,
                        complete_execution_signal_decision_task: false,
                    })
                    .await;

                match &response {
                    Ok(_) => {
                        info!("successfully responded to decision task")
                    }
                    Err(e) => error!(
                        error = %e,
                        "failed to respond to decision task"
                    ),
                }

                Ok(response?)
            }
            Err(e) => {
                // Respond with failure
                let _ = self.service.respond_decision_task_failed(RespondDecisionTaskFailedRequest {
                    task_token: task.task_token,
                    cause: uber_cadence_proto::shared::DecisionTaskFailedCause::WorkflowWorkerUnhandledFailure,
                    details: Some(format!("Workflow execution failed: {}", e).into_bytes()),
                    identity: self.identity.clone(),
                    binary_checksum: "".to_string(),
                }).await;

                // Still return error to log/metrics
                Err(e)
            }
        }
    }
}
