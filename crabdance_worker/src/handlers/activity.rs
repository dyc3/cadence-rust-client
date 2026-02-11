//! Activity task handler for processing activity tasks.

use crate::heartbeat::HeartbeatManager;
use crate::registry::{ActivityError, Registry};
use crabdance_core::{CadenceError, TransportError};
use crabdance_proto::workflow_service::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, oneshot, Mutex};
use tracing::{error, info, warn};

/// Activity task handler
pub struct ActivityTaskHandler {
    service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync>,
    registry: Arc<dyn Registry>,
    heartbeat_manager: Arc<HeartbeatManager>,
    identity: String,
    resources: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

struct ActivityRuntimeImpl {
    heartbeat_details: Arc<Mutex<Option<Vec<u8>>>>,
    cancelled: Arc<AtomicBool>,
}

impl crabdance_activity::ActivityRuntime for ActivityRuntimeImpl {
    fn record_heartbeat(&self, details: Option<Vec<u8>>) {
        let mut d = self.heartbeat_details.blocking_lock();
        *d = details;
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl ActivityTaskHandler {
    pub fn new(
        service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync>,
        registry: Arc<dyn Registry>,
        identity: String,
        resources: Option<Arc<dyn std::any::Any + Send + Sync>>,
    ) -> Self {
        let heartbeat_manager = Arc::new(HeartbeatManager::new(service.clone(), identity.clone()));
        Self {
            service,
            registry,
            heartbeat_manager,
            identity,
            resources,
        }
    }

    /// Handle an activity task
    pub async fn handle(
        &self,
        task: PollForActivityTaskResponse,
    ) -> Result<RespondActivityTaskCompletedResponse, CadenceError> {
        info!(
            activity_id = ?task.activity_id,
            "received activity task"
        );

        // Check if task has actual work (not an empty poll response)
        if task.task_token.is_empty() {
            warn!("empty task token, skipping activity task");
            return Err(CadenceError::Other("Empty task token received".to_string()));
        }

        // Extract activity name from task
        let activity_type = task
            .activity_type
            .as_ref()
            .ok_or_else(|| CadenceError::Other("Activity type missing from task".to_string()))?
            .name
            .clone();

        info!(activity_type = %activity_type, "handling activity task");

        // Look up activity in registry
        let activity = match self.registry.get_activity(&activity_type) {
            Some(a) => a,
            None => {
                // Activity not registered - send failure response
                tracing::warn!("Activity '{}' not registered in registry", activity_type);
                let _ = self
                    .service
                    .respond_activity_task_failed(RespondActivityTaskFailedRequest {
                        task_token: task.task_token.clone(),
                        reason: Some(format!("Activity '{}' not registered", activity_type)),
                        details: None,
                        identity: self.identity.clone(),
                    })
                    .await;

                return Err(CadenceError::Other(format!(
                    "Activity '{}' not registered",
                    activity_type
                )));
            }
        };

        // Create synchronization primitives
        let heartbeat_details = Arc::new(Mutex::new(None));
        let cancelled = Arc::new(AtomicBool::new(false));
        let (server_cancel_tx, mut server_cancel_rx) = broadcast::channel(1);

        // Handle server cancellation signal
        let cancelled_clone = cancelled.clone();
        tokio::spawn(async move {
            if server_cancel_rx.recv().await.is_ok() {
                cancelled_clone.store(true, Ordering::Relaxed);
            }
        });

        // Create runtime
        let runtime = Arc::new(ActivityRuntimeImpl {
            heartbeat_details: heartbeat_details.clone(),
            cancelled: cancelled.clone(),
        });

        // Create activity info
        let activity_info = crabdance_activity::ActivityInfo {
            activity_id: task.activity_id.clone(),
            activity_type: activity_type.clone(),
            task_token: task.task_token.clone(),
            workflow_execution: crabdance_activity::WorkflowExecution {
                workflow_id: task
                    .workflow_execution
                    .as_ref()
                    .map(|we| we.workflow_id.clone())
                    .unwrap_or_default(),
                run_id: task
                    .workflow_execution
                    .as_ref()
                    .map(|we| we.run_id.clone())
                    .unwrap_or_default(),
            },
            attempt: task.attempt,
            scheduled_time: chrono::Utc::now(), // TODO: Extract correctly from task metadata if available
            started_time: chrono::Utc::now(),
            deadline: self.calculate_deadline(&task),
            heartbeat_timeout: Duration::from_secs(
                task.heartbeat_timeout_seconds.unwrap_or(0) as u64
            ),
            heartbeat_details: task.heartbeat_details.clone(),
        };

        // Create activity context
        let context = match &self.resources {
            Some(resources) => crabdance_activity::ActivityContext::with_resources(
                activity_info,
                Some(runtime),
                resources.clone(),
            ),
            None => crabdance_activity::ActivityContext::new(activity_info, Some(runtime)),
        };

        // TODO: Pass worker stop channel to context if available

        // Start heartbeat if needed
        let heartbeat_timeout =
            Duration::from_secs(task.heartbeat_timeout_seconds.unwrap_or(0) as u64);
        let (cancel_heartbeat_tx, cancel_heartbeat_rx) = oneshot::channel();

        let _heartbeat_handle = if heartbeat_timeout > Duration::from_secs(0) {
            Some(self.heartbeat_manager.start_heartbeat(
                task.task_token.clone(),
                heartbeat_timeout,
                cancel_heartbeat_rx,
                Some(server_cancel_tx),
                heartbeat_details,
            ))
        } else {
            None
        };

        // Execute activity with panic recovery
        let input = task.input.clone();
        info!(activity_type = %activity_type, "executing activity");
        let context_ref = &context;
        let future = activity.execute(context_ref, input);

        // Execute with panic recovery using tokio::spawn
        let result = tokio::spawn(future).await;

        // Stop heartbeat
        let _ = cancel_heartbeat_tx.send(());

        let execution_result = match result {
            Ok(Ok(output)) => {
                info!(activity_type = %activity_type, "activity execution succeeded");
                Ok(output)
            }
            Ok(Err(e)) => {
                error!(
                    activity_type = %activity_type,
                    error = %e,
                    "activity execution failed"
                );
                Err(e)
            }
            Err(join_error) => {
                let panic_msg = if join_error.is_panic() {
                    format!("Activity panicked: {}", join_error)
                } else {
                    format!("Activity task cancelled: {}", join_error)
                };
                error!(
                    activity_type = %activity_type,
                    panic_msg = %panic_msg,
                    "activity panicked"
                );
                Err(ActivityError::Panic(panic_msg))
            }
        };

        // Send response based on result
        match execution_result {
            Ok(output) => {
                info!(activity_type = %activity_type, "sending activity complete response");
                let response = self
                    .service
                    .respond_activity_task_completed(RespondActivityTaskCompletedRequest {
                        task_token: task.task_token.clone(),
                        result: Some(output),
                        identity: self.identity.clone(),
                    })
                    .await?;

                tracing::info!("Activity '{}' completed successfully", activity_type);
                Ok(response)
            }
            Err(err) => {
                let (reason, details) = match &err {
                    ActivityError::ExecutionFailed(msg) => {
                        ("ExecutionFailed".to_string(), Some(msg.as_bytes().to_vec()))
                    }
                    ActivityError::Panic(msg) => (
                        "Panic".to_string(),
                        Some(format!("Activity panicked: {}", msg).into_bytes()),
                    ),
                    ActivityError::Retryable(msg) => {
                        ("Retryable".to_string(), Some(msg.as_bytes().to_vec()))
                    }
                    ActivityError::NonRetryable(msg) => {
                        ("NonRetryable".to_string(), Some(msg.as_bytes().to_vec()))
                    }
                    ActivityError::Application(msg) => (
                        "ApplicationError".to_string(),
                        Some(msg.as_bytes().to_vec()),
                    ),
                    ActivityError::RetryableWithDelay(msg, _delay) => (
                        "RetryableWithDelay".to_string(),
                        Some(msg.as_bytes().to_vec()),
                    ),
                    ActivityError::Cancelled => ("Cancelled".to_string(), None),
                    ActivityError::Timeout(t) => (format!("Timeout: {:?}", t), None),
                };

                let _ = self
                    .service
                    .respond_activity_task_failed(RespondActivityTaskFailedRequest {
                        task_token: task.task_token.clone(),
                        reason: Some(reason),
                        details,
                        identity: self.identity.clone(),
                    })
                    .await?;

                tracing::error!(activity_type, ?err, "Activity failed");
                Err(CadenceError::Other(format!(
                    "Activity execution error: {}",
                    err
                )))
            }
        }
    }

    /// Calculate activity deadline from task timeouts
    fn calculate_deadline(&self, task: &PollForActivityTaskResponse) -> Option<Instant> {
        // Use schedule_to_close_timeout as the overall deadline
        let timeout_seconds = task.schedule_to_close_timeout_seconds.unwrap_or(0);
        if timeout_seconds > 0 {
            Some(Instant::now() + Duration::from_secs(timeout_seconds as u64))
        } else {
            None
        }
    }
}
