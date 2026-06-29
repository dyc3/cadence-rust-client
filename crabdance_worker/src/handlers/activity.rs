//! Activity task handler for processing activity tasks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, oneshot, Mutex};
use tracing::{error, info, warn};

use crabdance_core::{
    CadenceError, ContextPropagator, DataConverter, InterceptorChain, InterceptorContext,
    JsonDataConverter, Operation, Outcome, TransportError,
};
use crabdance_proto::workflow_service::*;

use crate::heartbeat::HeartbeatManager;
use crate::interceptor::extract_propagation;
use crate::registry::{ActivityError, Registry};

/// Activity task handler
pub struct ActivityTaskHandler {
    service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync>,
    registry: Arc<dyn Registry>,
    heartbeat_manager: Arc<HeartbeatManager>,
    identity: String,
    resources: Option<Arc<dyn std::any::Any + Send + Sync>>,
    data_converter: Arc<dyn DataConverter>,
    interceptors: InterceptorChain,
    context_propagators: Vec<Arc<dyn ContextPropagator>>,
    /// Set on the session worker; handles the internal session creation/completion
    /// activities (token acquire/release).
    session_environment: Option<Arc<crate::session::SessionEnvironment>>,
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

/// The terminal outcome of an activity invocation — the small interface the
/// lifecycle reports from. The server response, the terminal metric + latency,
/// and the interceptor `after`-hook are all derived from this in one place
/// ([`ActivityTaskHandler::report_disposition`]), so the started/terminal balance
/// can't drift per exit path.
enum ActivityDisposition {
    /// The activity completed with this result payload.
    Completed(Vec<u8>),
    /// The activity failed; `reason`/`details` go to the server, `error` is returned.
    Failed {
        reason: String,
        details: Option<Vec<u8>>,
        error: CadenceError,
    },
    /// Async/external completion requested — **non-terminal on this worker**: no
    /// response, no terminal metric, no `after`-hook. The result arrives later via
    /// `Client::complete_activity[_by_id]`.
    Pending,
}

/// Map an activity error to the server-facing `(reason, details)` pair.
fn activity_failure_reason(err: &ActivityError) -> (String, Option<Vec<u8>>) {
    match err {
        ActivityError::ExecutionFailed(e) => (
            "ExecutionFailed".to_string(),
            Some(e.to_string().into_bytes()),
        ),
        ActivityError::Panic(e) => (
            "Panic".to_string(),
            Some(format!("Activity panicked: {e}").into_bytes()),
        ),
        ActivityError::Retryable(e) => ("Retryable".to_string(), Some(e.to_string().into_bytes())),
        ActivityError::NonRetryable(e) => {
            ("NonRetryable".to_string(), Some(e.to_string().into_bytes()))
        }
        ActivityError::Application(e) => (
            "ApplicationError".to_string(),
            Some(e.to_string().into_bytes()),
        ),
        ActivityError::RetryableWithDelay(e, _delay) => (
            "RetryableWithDelay".to_string(),
            Some(e.to_string().into_bytes()),
        ),
        ActivityError::Cancelled => ("Cancelled".to_string(), None),
        ActivityError::Timeout(t) => (format!("Timeout: {t:?}"), None),
        // ResultPending is intercepted before this point; here for exhaustiveness.
        ActivityError::ResultPending => ("ResultPending".to_string(), None),
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
            data_converter: Arc::new(JsonDataConverter),
            interceptors: InterceptorChain::default(),
            context_propagators: Vec::new(),
            session_environment: None,
        }
    }

    /// Attach the session environment so this handler serves the internal session
    /// creation/completion activities.
    pub fn with_session_environment(
        mut self,
        env: Arc<crate::session::SessionEnvironment>,
    ) -> Self {
        self.session_environment = Some(env);
        self
    }

    /// Serve the internal session-creation activity: acquire a concurrency token
    /// (idempotently, keyed by session id) and return the host-specific resource task
    /// list. Fails with "too many outstanding sessions" when the worker is full, and
    /// rolls the token back if the response could not be encoded/sent — so a Cadence
    /// retry neither double-acquires nor leaks the slot.
    async fn handle_session_creation(
        &self,
        task: &PollForActivityTaskResponse,
        env: &Arc<crate::session::SessionEnvironment>,
        activity_type: &str,
        started: Instant,
    ) -> Result<RespondActivityTaskCompletedResponse, CadenceError> {
        let session_id =
            String::from_utf8(task.input.clone().unwrap_or_default()).unwrap_or_default();

        if !env.acquire(&session_id) {
            let _ = self
                .service
                .respond_activity_task_failed(RespondActivityTaskFailedRequest {
                    task_token: task.task_token.clone(),
                    reason: Some("too many outstanding sessions".to_string()),
                    details: None,
                    identity: self.identity.clone(),
                })
                .await;
            self.record_terminal(activity_type, started, false);
            return Err(CadenceError::Other(
                "too many outstanding sessions".to_string(),
            ));
        }

        let response = crabdance_workflow::session::SessionCreationResponse {
            tasklist: env.resource_tasklist.clone(),
            host_name: env.host.clone(),
            resource_id: env.resource_id.clone(),
        };

        let send = serde_json::to_vec(&response)
            .map_err(|e| CadenceError::Other(format!("encode session response: {e}")));
        let send = match send {
            Ok(result) => self
                .service
                .respond_activity_task_completed(RespondActivityTaskCompletedRequest {
                    task_token: task.task_token.clone(),
                    result: Some(result),
                    identity: self.identity.clone(),
                })
                .await
                .map_err(CadenceError::from),
            Err(e) => Err(e),
        };

        match send {
            Ok(response) => {
                self.record_terminal(activity_type, started, true);
                Ok(response)
            }
            Err(e) => {
                // The session was never confirmed to the workflow; roll the token back
                // so the retry can re-acquire cleanly.
                env.release(&session_id);
                self.record_terminal(activity_type, started, false);
                Err(e)
            }
        }
    }

    /// Serve the internal session-completion activity: release the session's token
    /// (idempotently, keyed by session id).
    async fn handle_session_completion(
        &self,
        task: &PollForActivityTaskResponse,
        env: &Arc<crate::session::SessionEnvironment>,
        activity_type: &str,
        started: Instant,
    ) -> Result<RespondActivityTaskCompletedResponse, CadenceError> {
        let session_id =
            String::from_utf8(task.input.clone().unwrap_or_default()).unwrap_or_default();
        env.release(&session_id);
        self.record_terminal(activity_type, started, true);
        self.service
            .respond_activity_task_completed(RespondActivityTaskCompletedRequest {
                task_token: task.task_token.clone(),
                result: Some(Vec::new()),
                identity: self.identity.clone(),
            })
            .await
            .map_err(CadenceError::from)
    }

    /// Inject the worker's configured payload converter, threaded into every
    /// `ActivityContext` this handler builds.
    pub fn with_data_converter(mut self, converter: Arc<dyn DataConverter>) -> Self {
        self.data_converter = converter;
        self
    }

    /// Inject the worker's around-execution interceptors. They wrap each activity
    /// execution (before/veto + after/timing).
    pub fn with_interceptors(
        mut self,
        interceptors: Vec<Arc<dyn crabdance_core::Interceptor>>,
    ) -> Self {
        self.interceptors = InterceptorChain::new(interceptors);
        self
    }

    /// Inject the worker's context propagators, used to extract the propagation
    /// context from the activity task header for interceptors.
    pub fn with_context_propagators(
        mut self,
        propagators: Vec<Arc<dyn ContextPropagator>>,
    ) -> Self {
        self.context_propagators = propagators;
        self
    }

    /// Handle an activity task. Owns the lifecycle: emit `started`, run the activity
    /// to an [`ActivityDisposition`], then report that disposition once.
    pub async fn handle(
        &self,
        task: PollForActivityTaskResponse,
    ) -> Result<RespondActivityTaskCompletedResponse, CadenceError> {
        info!(
            activity_id = ?task.activity_id,
            "received activity task"
        );

        // Malformed-poll guards: not a real activity invocation, so no lifecycle metrics.
        if task.task_token.is_empty() {
            warn!("empty task token, skipping activity task");
            return Err(CadenceError::Other("Empty task token received".to_string()));
        }
        let activity_type = task
            .activity_type
            .as_ref()
            .ok_or_else(|| CadenceError::Other("Activity type missing from task".to_string()))?
            .name
            .clone();

        info!(activity_type = %activity_type, "handling activity task");
        let started = std::time::Instant::now();
        crate::metrics::incr(
            crate::metrics::ACTIVITY_TASK_STARTED,
            crate::metrics::TAG_ACTIVITY_TYPE,
            &activity_type,
        );

        // Internal session activities have a distinct lifecycle (no interceptors /
        // registry); they report their own terminal metric via `record_terminal`.
        if let Some(env) = self.session_environment.clone() {
            if activity_type == crabdance_workflow::SESSION_CREATION_ACTIVITY {
                return self
                    .handle_session_creation(&task, &env, &activity_type, started)
                    .await;
            }
            if activity_type == crabdance_workflow::SESSION_COMPLETION_ACTIVITY {
                return self
                    .handle_session_completion(&task, &env, &activity_type, started)
                    .await;
            }
        }

        // Build the interceptor context once (only when interceptors are configured).
        let interceptor_ctx = if self.interceptors.is_empty() {
            None
        } else {
            let (workflow_id, run_id) = task
                .workflow_execution
                .as_ref()
                .map(|we| (we.workflow_id.clone(), we.run_id.clone()))
                .unwrap_or_default();
            Some(InterceptorContext {
                operation: Operation::Activity,
                name: activity_type.clone(),
                workflow_id,
                run_id,
                is_replaying: false,
                propagation: extract_propagation(&task.header, &self.context_propagators),
            })
        };

        let disposition = self
            .run_activity(&task, &activity_type, interceptor_ctx.as_ref())
            .await;
        self.report_disposition(
            &task,
            &activity_type,
            interceptor_ctx.as_ref(),
            started,
            disposition,
        )
        .await
    }

    /// Run the activity to a terminal [`ActivityDisposition`] — interceptor veto,
    /// registry lookup, and execution. Performs **no** server I/O and emits **no**
    /// terminal metric, so it can be exercised through its return value.
    async fn run_activity(
        &self,
        task: &PollForActivityTaskResponse,
        activity_type: &str,
        interceptor_ctx: Option<&InterceptorContext>,
    ) -> ActivityDisposition {
        // before-hook: a veto fails the activity without executing it (policy gate /
        // fault injection).
        if let Some(ictx) = interceptor_ctx {
            if let Err(veto) = self.interceptors.before(ictx) {
                warn!(activity_type, error = %veto, "activity vetoed by interceptor");
                return ActivityDisposition::Failed {
                    reason: "InterceptorVeto".to_string(),
                    details: Some(format!("Activity vetoed by interceptor: {veto}").into_bytes()),
                    error: veto,
                };
            }
        }

        // Look up activity in registry
        let activity = match self.registry.get_activity(activity_type) {
            Some(a) => a,
            None => {
                tracing::warn!("Activity '{activity_type}' not registered in registry");
                return ActivityDisposition::Failed {
                    reason: format!("Activity '{activity_type}' not registered"),
                    details: None,
                    error: CadenceError::Other(format!(
                        "Activity '{activity_type}' not registered"
                    )),
                };
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
            activity_type: activity_type.to_string(),
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
            deadline: self.calculate_deadline(task),
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
        }
        .with_converter(self.data_converter.clone());

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

        // Span for the activity execution. Under the `otel` feature it is parented to
        // the remote trace carried in the task header, so the activity span joins the
        // workflow's trace (W3C `traceparent`).
        let exec_span = tracing::info_span!("activity.execute", activity_type = %activity_type);
        #[cfg(feature = "otel")]
        crate::otel::set_remote_parent(&exec_span, task.header.as_ref());

        // Execute with panic recovery using tokio::spawn
        let result = tokio::spawn(tracing::Instrument::instrument(future, exec_span)).await;

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
                Err(ActivityError::Panic(
                    crabdance_workflow::future::boxed_error(panic_msg),
                ))
            }
        };

        // Map the execution result to a terminal disposition.
        match execution_result {
            Ok(output) => ActivityDisposition::Completed(output),
            // Async completion: the activity asked not to be auto-responded.
            Err(err) if err.is_result_pending() => ActivityDisposition::Pending,
            Err(err) => {
                tracing::error!(activity_type, ?err, "activity failed");
                let (reason, details) = activity_failure_reason(&err);
                ActivityDisposition::Failed {
                    reason,
                    details,
                    error: CadenceError::Other(err.to_string()),
                }
            }
        }
    }

    /// Report a terminal [`ActivityDisposition`] once: fire the interceptor
    /// `after`-hook, emit the terminal metric + latency, and send the server
    /// response. [`ActivityDisposition::Pending`] is non-terminal — it does none of
    /// these (the result is delivered out of band).
    async fn report_disposition(
        &self,
        task: &PollForActivityTaskResponse,
        activity_type: &str,
        interceptor_ctx: Option<&InterceptorContext>,
        started: Instant,
        disposition: ActivityDisposition,
    ) -> Result<RespondActivityTaskCompletedResponse, CadenceError> {
        if let ActivityDisposition::Pending = disposition {
            info!(
                activity_type,
                "activity result pending; not auto-responding (async completion)"
            );
            return Ok(RespondActivityTaskCompletedResponse {});
        }

        let success = matches!(disposition, ActivityDisposition::Completed(_));

        // after-hook: every terminal outcome the before-hook saw gets an after.
        if let Some(ictx) = interceptor_ctx {
            self.interceptors.after(
                ictx,
                &Outcome {
                    duration: started.elapsed(),
                    success,
                },
            );
        }
        self.record_terminal(activity_type, started, success);

        match disposition {
            ActivityDisposition::Completed(output) => {
                info!(activity_type, "sending activity complete response");
                self.service
                    .respond_activity_task_completed(RespondActivityTaskCompletedRequest {
                        task_token: task.task_token.clone(),
                        result: Some(output),
                        identity: self.identity.clone(),
                    })
                    .await
                    .map_err(CadenceError::from)
            }
            ActivityDisposition::Failed {
                reason,
                details,
                error,
            } => {
                self.service
                    .respond_activity_task_failed(RespondActivityTaskFailedRequest {
                        task_token: task.task_token.clone(),
                        reason: Some(reason),
                        details,
                        identity: self.identity.clone(),
                    })
                    .await?;
                Err(error)
            }
            ActivityDisposition::Pending => unreachable!("pending handled above"),
        }
    }

    /// Emit the terminal counter (`completed`/`failed`) + latency for an activity
    /// type. The one place the started/terminal balance is kept.
    fn record_terminal(&self, activity_type: &str, started: Instant, success: bool) {
        let counter = if success {
            crate::metrics::ACTIVITY_TASK_COMPLETED
        } else {
            crate::metrics::ACTIVITY_TASK_FAILED
        };
        crate::metrics::incr(counter, crate::metrics::TAG_ACTIVITY_TYPE, activity_type);
        crate::metrics::record_latency(
            crate::metrics::ACTIVITY_TASK_LATENCY,
            crate::metrics::TAG_ACTIVITY_TYPE,
            activity_type,
            started.elapsed(),
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crabdance_workflow::future::boxed_error;

    fn details_string(details: Option<Vec<u8>>) -> String {
        String::from_utf8(details.expect("expected details")).unwrap()
    }

    #[test]
    fn failure_reason_maps_each_variant() {
        let (reason, details) =
            activity_failure_reason(&ActivityError::ExecutionFailed(boxed_error("boom")));
        assert_eq!(reason, "ExecutionFailed");
        assert!(details_string(details).contains("boom"));

        let (reason, details) =
            activity_failure_reason(&ActivityError::Panic(boxed_error("kaboom")));
        assert_eq!(reason, "Panic");
        let d = details_string(details);
        assert!(d.contains("Activity panicked") && d.contains("kaboom"));

        let (reason, details) =
            activity_failure_reason(&ActivityError::NonRetryable(boxed_error("nope")));
        assert_eq!(reason, "NonRetryable");
        assert!(details_string(details).contains("nope"));

        // Control-flow variants carry no details.
        let (reason, details) = activity_failure_reason(&ActivityError::Cancelled);
        assert_eq!(reason, "Cancelled");
        assert!(details.is_none());
    }
}
