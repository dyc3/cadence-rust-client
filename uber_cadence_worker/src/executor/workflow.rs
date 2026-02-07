use crate::executor::cache::{CachedWorkflow, WorkflowCache, WorkflowExecutionKey};
use crate::executor::replay::ReplayEngine;
use crate::local_activity_queue::LocalActivityQueue;
use crate::registry::Registry;
use crate::replay_verifier::match_replay_with_history;
use crate::WorkerOptions;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use uber_cadence_core::{CadenceError, ReplayContext, WorkflowInfo};
use uber_cadence_proto::shared::{
    ActivityType, ContinueAsNewWorkflowExecutionDecisionAttributes, Decision, EventType,
    HistoryEvent, RequestCancelExternalWorkflowExecutionDecisionAttributes,
    ScheduleActivityTaskDecisionAttributes, SignalExternalWorkflowExecutionDecisionAttributes,
    StartChildWorkflowExecutionDecisionAttributes, StartTimerDecisionAttributes, TaskList,
    TaskListKind, WorkflowType,
};
use uber_cadence_proto::workflow_service::PollForDecisionTaskResponse;
use uber_cadence_proto::{QueryResultType, WorkflowQueryResult};
use uber_cadence_workflow::commands::WorkflowCommand;
use uber_cadence_workflow::context::{CommandSink, WorkflowContext};
use uber_cadence_workflow::dispatcher::{WorkflowDispatcher, WorkflowTask};
use uber_cadence_workflow::future::WorkflowError;
use uber_cadence_workflow::state_machine::{
    ActivityDecisionStateMachine, ChildWorkflowDecisionStateMachine, DecisionId,
    StateMachineDecisionType, TimerDecisionStateMachine,
};

// Type alias to reduce complexity
type LocalActivityWakers =
    Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<Result<Vec<u8>, WorkflowError>>>>>;

struct ReplayCommandSink {
    engine: Arc<Mutex<ReplayEngine>>,
    default_task_list: String,
    #[expect(dead_code)]
    local_activity_queue: LocalActivityQueue,
    #[expect(dead_code)]
    workflow_info: WorkflowInfo,
    // Track pending local activity submissions
    pending_local_activity_submissions: Arc<Mutex<Vec<PendingLocalActivitySubmission>>>,
    // Map of activity_id to waker for unblocking workflow when local activity completes
    local_activity_wakers: LocalActivityWakers,
}

// Represents a local activity that needs to be executed
struct PendingLocalActivitySubmission {
    activity_id: String,
    activity_type: String,
    args: Option<Vec<u8>>,
    options: uber_cadence_workflow::LocalActivityOptions,
}

impl CommandSink for ReplayCommandSink {
    fn submit(
        &self,
        command: WorkflowCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        let engine = self.engine.clone();
        let default_task_list = self.default_task_list.clone();
        let pending_local_activity_submissions = self.pending_local_activity_submissions.clone();
        let local_activity_wakers = self.local_activity_wakers.clone();

        Box::pin(async move {
            eprintln!("[CommandSink] Submitting command (stderr): {:?}", command);
            match command {
                WorkflowCommand::ScheduleActivity(cmd) => {
                    eprintln!(
                        "[CommandSink] Scheduling Activity (stderr): {}",
                        cmd.activity_id
                    );

                    // Check 1: Is the activity already completed?
                    let already_completed = {
                        let engine_lock = engine.lock().unwrap();
                        engine_lock
                            .get_activity_result(&cmd.activity_id)
                            .map(|r| match r {
                                Ok(data) => Ok(data.clone()),
                                Err(e) => Err(e.to_string()),
                            })
                    };

                    if let Some(result) = already_completed {
                        println!(
                            "[CommandSink] Activity {} completed/failed immediately (from history)",
                            cmd.activity_id
                        );
                        return match result {
                            Ok(data) => Ok(data),
                            Err(e) => Err(WorkflowError::ActivityFailed(e)),
                        };
                    }

                    // Check 2: Try to add decision - if it already exists (from replay), add_decision returns false
                    let task_list_name = if cmd.options.task_list.is_empty() {
                        default_task_list.clone()
                    } else {
                        cmd.options.task_list.clone()
                    };

                    println!(
                        "[CommandSink] Scheduling Activity '{}' on task list: '{}'",
                        cmd.activity_id, task_list_name
                    );

                    let attrs = ScheduleActivityTaskDecisionAttributes {
                        activity_id: cmd.activity_id.clone(),
                        activity_type: Some(ActivityType {
                            name: cmd.activity_type,
                        }),
                        task_list: Some(TaskList {
                            name: task_list_name,
                            kind: TaskListKind::Normal,
                        }),
                        input: cmd.args,
                        schedule_to_close_timeout_seconds: Some(
                            cmd.options.schedule_to_close_timeout.as_secs() as i32,
                        ),
                        start_to_close_timeout_seconds: Some(
                            cmd.options.start_to_close_timeout.as_secs() as i32,
                        ),
                        schedule_to_start_timeout_seconds: Some(
                            cmd.options.schedule_to_start_timeout.as_secs() as i32,
                        ),
                        heartbeat_timeout_seconds: Some(
                            cmd.options.heartbeat_timeout.as_secs() as i32
                        ),
                        retry_policy: cmd.options.retry_policy.map(|p| {
                            uber_cadence_proto::shared::RetryPolicy {
                                initial_interval_in_seconds: p.initial_interval.as_secs() as i32,
                                backoff_coefficient: p.backoff_coefficient,
                                maximum_interval_in_seconds: p.maximum_interval.as_secs() as i32,
                                maximum_attempts: p.maximum_attempts,
                                non_retryable_error_types: p.non_retryable_error_types,
                                expiration_interval_in_seconds: p.expiration_interval.as_secs()
                                    as i32,
                            }
                        }),
                        header: None,
                    };

                    let decision = Box::new(ActivityDecisionStateMachine::new(
                        cmd.activity_id.clone(),
                        attrs,
                    ));

                    let added = {
                        let mut engine_lock = engine.lock().unwrap();
                        engine_lock.decisions_helper.add_decision(decision)
                    };

                    if !added {
                        // Decision already exists (pre-created during replay)
                        // Activity is in progress, block until completed
                        println!(
                            "[CommandSink] Activity {} already scheduled, blocking until completion",
                            cmd.activity_id
                        );
                        return std::future::pending().await;
                    }

                    // New activity, will be included in pending decisions
                }
                WorkflowCommand::StartTimer(cmd) => {
                    // Check if timer already fired
                    let already_fired = {
                        let engine_lock = engine.lock().unwrap();
                        engine_lock.get_timer_result(&cmd.timer_id).is_some()
                    };

                    if already_fired {
                        return Ok(Vec::new());
                    }

                    // Try to add decision - if it already exists (from replay), add_decision returns false
                    let attrs = StartTimerDecisionAttributes {
                        timer_id: cmd.timer_id.clone(),
                        start_to_fire_timeout_seconds: cmd.duration.as_secs() as i64,
                    };

                    let decision =
                        Box::new(TimerDecisionStateMachine::new(cmd.timer_id.clone(), attrs));

                    let added = {
                        let mut engine_lock = engine.lock().unwrap();
                        engine_lock.decisions_helper.add_decision(decision)
                    };

                    if !added {
                        // Decision already exists (pre-created during replay)
                        // Timer is in progress, block until fired
                        println!(
                            "[CommandSink] Timer {} already scheduled, blocking until fired",
                            cmd.timer_id
                        );
                        return std::future::pending().await;
                    }
                }
                WorkflowCommand::CancelTimer(cmd) => {
                    let mut engine_lock = engine.lock().unwrap();
                    let id = DecisionId::new(StateMachineDecisionType::Timer, cmd.timer_id);
                    if let Some(decision) = engine_lock.decisions_helper.get_decision(&id) {
                        decision.cancel();
                    }
                    return Ok(Vec::new());
                }
                WorkflowCommand::StartChildWorkflow(cmd) => {
                    // Check if child workflow already completed
                    let already_completed = {
                        let engine_lock = engine.lock().unwrap();
                        engine_lock
                            .get_child_workflow_result(&cmd.workflow_id)
                            .map(|r| match r {
                                Ok(data) => Ok(data.clone()),
                                Err(e) => Err(e.to_string()),
                            })
                    };

                    if let Some(result) = already_completed {
                        return match result {
                            Ok(data) => Ok(data),
                            Err(e) => Err(WorkflowError::ChildWorkflowFailed(e)),
                        };
                    }

                    // Try to add decision - if it already exists (from replay), add_decision returns false
                    let attrs = StartChildWorkflowExecutionDecisionAttributes {
                        domain: cmd.options.domain.unwrap_or_default(),
                        workflow_id: cmd.workflow_id.clone(),
                        workflow_type: Some(uber_cadence_proto::shared::WorkflowType {
                            name: cmd.workflow_type,
                        }),
                        task_list: Some(TaskList {
                            name: cmd.options.task_list.unwrap_or_default(),
                            kind: TaskListKind::Normal,
                        }),
                        input: cmd.args,
                        execution_start_to_close_timeout_seconds: Some(
                            cmd.options.execution_start_to_close_timeout.as_secs() as i32,
                        ),
                        task_start_to_close_timeout_seconds: Some(
                            cmd.options.task_start_to_close_timeout.as_secs() as i32,
                        ),
                        retry_policy: cmd.options.retry_policy.map(|p| {
                            uber_cadence_proto::shared::RetryPolicy {
                                initial_interval_in_seconds: p.initial_interval.as_secs() as i32,
                                backoff_coefficient: p.backoff_coefficient,
                                maximum_interval_in_seconds: p.maximum_interval.as_secs() as i32,
                                maximum_attempts: p.maximum_attempts,
                                non_retryable_error_types: p.non_retryable_error_types,
                                expiration_interval_in_seconds: p.expiration_interval.as_secs()
                                    as i32,
                            }
                        }),
                        cron_schedule: cmd.options.cron_schedule,
                        header: None,
                        memo: None,
                        search_attributes: None,
                        workflow_id_reuse_policy: None,
                        parent_close_policy: None,
                        control: None,
                        decision_task_completed_event_id: 0,
                    };

                    let decision = Box::new(ChildWorkflowDecisionStateMachine::new(
                        cmd.workflow_id.clone(),
                        attrs,
                    ));

                    let added = {
                        let mut engine_lock = engine.lock().unwrap();
                        engine_lock.decisions_helper.add_decision(decision)
                    };

                    if !added {
                        // Decision already exists (pre-created during replay)
                        // Child workflow is in progress, block until completed
                        println!(
                            "[CommandSink] Child workflow {} already initiated, blocking until completion",
                            cmd.workflow_id
                        );
                        return std::future::pending().await;
                    }
                }
                WorkflowCommand::ContinueAsNewWorkflow(cmd) => {
                    let mut engine_lock = engine.lock().unwrap();

                    let attrs = ContinueAsNewWorkflowExecutionDecisionAttributes {
                        workflow_type: Some(WorkflowType {
                            name: cmd.workflow_type,
                        }),
                        task_list: Some(TaskList {
                            name: cmd.options.task_list,
                            kind: TaskListKind::Normal,
                        }),
                        input: cmd.input,
                        execution_start_to_close_timeout_seconds: Some(
                            cmd.options.execution_start_to_close_timeout.as_secs() as i32,
                        ),
                        task_start_to_close_timeout_seconds: Some(
                            cmd.options.task_start_to_close_timeout.as_secs() as i32,
                        ),
                        backoff_start_interval_in_seconds: None,
                        retry_policy: cmd.options.retry_policy.map(|p| {
                            uber_cadence_proto::shared::RetryPolicy {
                                initial_interval_in_seconds: p.initial_interval.as_secs() as i32,
                                backoff_coefficient: p.backoff_coefficient,
                                maximum_interval_in_seconds: p.maximum_interval.as_secs() as i32,
                                maximum_attempts: p.maximum_attempts,
                                non_retryable_error_types: p.non_retryable_error_types,
                                expiration_interval_in_seconds: p.expiration_interval.as_secs()
                                    as i32,
                            }
                        }),
                        initiator: None,
                        failure_details: None,
                        last_completion_result: None,
                        cron_schedule: cmd.options.cron_schedule,
                        header: None,
                        memo: None,
                        search_attributes: None,
                    };

                    engine_lock
                        .decisions_helper
                        .continue_as_new_workflow_execution(attrs);
                    drop(engine_lock);
                    return Ok(Vec::new());
                }
                WorkflowCommand::SignalExternalWorkflow(cmd) => {
                    let mut engine_lock = engine.lock().unwrap();

                    if let Some(result) =
                        engine_lock.get_signal_external_workflow_result(&cmd.signal_id)
                    {
                        return match result {
                            Ok(_) => Ok(Vec::new()),
                            Err(e) => Err(WorkflowError::ActivityFailed(e.to_string())),
                        };
                    }

                    let attrs = SignalExternalWorkflowExecutionDecisionAttributes {
                        domain: cmd.domain.unwrap_or_default(),
                        workflow_execution: Some(uber_cadence_proto::shared::WorkflowExecution {
                            workflow_id: cmd.workflow_id.clone(),
                            run_id: cmd.run_id.unwrap_or_default(),
                        }),
                        signal_name: cmd.signal_name,
                        input: cmd.args,
                        control: Some(cmd.signal_id.clone()),
                        child_workflow_only: cmd.child_workflow_only,
                    };

                    engine_lock
                        .decisions_helper
                        .signal_external_workflow_execution(cmd.signal_id, attrs);
                    drop(engine_lock);
                    // Don't return pending yet, wait for completion?
                    // Cadence semantics: signal returns Future that completes when signal is *accepted* by external workflow (or at least sent).
                    // So we treat it like an activity/timer.
                }
                WorkflowCommand::RequestCancelExternalWorkflow(cmd) => {
                    let mut engine_lock = engine.lock().unwrap();

                    if let Some(result) = engine_lock
                        .get_request_cancel_external_workflow_result(&cmd.cancellation_id)
                    {
                        return match result {
                            Ok(_) => Ok(Vec::new()),
                            Err(e) => Err(WorkflowError::ActivityFailed(e.to_string())),
                        };
                    }

                    let attrs = RequestCancelExternalWorkflowExecutionDecisionAttributes {
                        domain: cmd.domain.unwrap_or_default(),
                        workflow_execution: Some(uber_cadence_proto::shared::WorkflowExecution {
                            workflow_id: cmd.workflow_id.clone(),
                            run_id: cmd.run_id.unwrap_or_default(),
                        }),
                        control: Some(cmd.cancellation_id.clone()),
                        child_workflow_only: cmd.child_workflow_only,
                    };

                    engine_lock
                        .decisions_helper
                        .request_cancel_external_workflow_execution(cmd.cancellation_id, attrs);
                    drop(engine_lock);
                }
                WorkflowCommand::ScheduleLocalActivity(cmd) => {
                    eprintln!(
                        "[CommandSink] Scheduling Local Activity (stderr): {}",
                        cmd.activity_id
                    );

                    // Check if local activity result already exists in replay cache
                    let already_completed = {
                        let engine_lock = engine.lock().unwrap();
                        engine_lock.get_local_activity_result(&cmd.activity_id)
                    };

                    if let Some(marker_data) = already_completed {
                        eprintln!(
                            "[CommandSink] Local activity {} completed from replay cache",
                            cmd.activity_id
                        );
                        // Return cached result from replay
                        if let Some(result) = marker_data.result_json {
                            return Ok(result);
                        } else if let Some(reason) = marker_data.err_reason {
                            return Err(WorkflowError::ActivityFailed(reason));
                        } else {
                            return Err(WorkflowError::ActivityFailed("Unknown error".to_string()));
                        }
                    }

                    // Not in replay cache - record this as a pending submission
                    eprintln!(
                        "[CommandSink] Local activity {} not in cache, recording as pending",
                        cmd.activity_id
                    );

                    // Create a channel to receive the result when local activity completes
                    let (result_tx, result_rx) = tokio::sync::oneshot::channel();

                    {
                        let mut pending = pending_local_activity_submissions.lock().unwrap();
                        pending.push(PendingLocalActivitySubmission {
                            activity_id: cmd.activity_id.clone(),
                            activity_type: cmd.activity_type.clone(),
                            args: cmd.args.clone(),
                            options: cmd.options.clone(),
                        });

                        // Store the result sender so we can signal when complete
                        let mut wakers = local_activity_wakers.lock().unwrap();
                        wakers.insert(cmd.activity_id.clone(), result_tx);
                    }

                    // Wait for the result to be sent by the executor
                    eprintln!(
                        "[CommandSink] Local activity {} recorded as pending, awaiting result",
                        cmd.activity_id
                    );
                    match result_rx.await {
                        Ok(result) => return result,
                        Err(_) => {
                            return Err(WorkflowError::ActivityFailed(
                                "Channel closed before result received".to_string(),
                            ))
                        }
                    }
                }
                WorkflowCommand::RecordMarker(cmd) => {
                    eprintln!(
                        "[CommandSink] Recording Marker (stderr): {}",
                        cmd.marker_name
                    );

                    let attrs = uber_cadence_proto::shared::RecordMarkerDecisionAttributes {
                        marker_name: cmd.marker_name.clone(),
                        details: Some(cmd.details),
                        header: cmd.header,
                    };

                    // Generate a unique marker ID based on marker name and sequence
                    // Generate a unique marker ID based on marker name and timestamp
                    let marker_id = format!("{}_{}", cmd.marker_name, {
                        // Use timestamp nanos as a simple unique identifier
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos()
                            .to_string()
                    });

                    let decision = Box::new(
                        uber_cadence_workflow::state_machine::MarkerDecisionStateMachine::new(
                            marker_id.clone(),
                            attrs,
                        ),
                    );

                    let mut engine_lock = engine.lock().unwrap();
                    engine_lock.decisions_helper.add_decision(decision);
                    drop(engine_lock);

                    println!("[CommandSink] Recorded marker: {}", marker_id);

                    // Return immediately since markers are synchronous
                    return Ok(Vec::new());
                }
                _ => unimplemented!("Command not supported"),
            }
            std::future::pending().await
        })
    }
}

/// Workflow executor
pub struct WorkflowExecutor {
    registry: Arc<dyn Registry>,
    cache: Arc<WorkflowCache>,
    #[expect(dead_code)]
    options: WorkerOptions,
    task_list: String,
    local_activity_queue: LocalActivityQueue,
}

/// Workflow execution state
pub struct WorkflowExecution {
    pub engine: Arc<Mutex<ReplayEngine>>,
    pub workflow_info: WorkflowInfo,
}

impl WorkflowExecutor {
    pub fn new(
        registry: Arc<dyn Registry>,
        cache: Arc<WorkflowCache>,
        options: WorkerOptions,
        task_list: String,
        local_activity_queue: LocalActivityQueue,
    ) -> Self {
        Self {
            registry,
            cache,
            options,
            task_list,
            local_activity_queue,
        }
    }

    pub async fn execute_decision_task(
        &self,
        task: PollForDecisionTaskResponse,
    ) -> Result<(Vec<Decision>, HashMap<String, WorkflowQueryResult>), CadenceError> {
        let history = task
            .history
            .ok_or_else(|| CadenceError::Other("No history in task".into()))?;
        let events = history.events;
        let workflow_type = task
            .workflow_type
            .ok_or_else(|| CadenceError::Other("No workflow type".into()))?
            .name;

        let workflow_id = task
            .workflow_execution
            .as_ref()
            .map(|x| x.workflow_id.clone())
            .unwrap_or_default();
        let run_id = task
            .workflow_execution
            .as_ref()
            .map(|x| x.run_id.clone())
            .unwrap_or_default();

        println!(
            "[WorkflowExecutor] Executing decision task for workflow={} run={}",
            workflow_id, run_id
        );

        let key = WorkflowExecutionKey {
            workflow_id: workflow_id.clone(),
            run_id: run_id.clone(),
        };

        // Check cache for sticky execution
        let (engine_arc, from_cache) = if let Some(cached) = self.cache.get(&key) {
            let execution = cached.execution.lock().unwrap();
            (execution.engine.clone(), true)
        } else {
            let engine = ReplayEngine::new();
            (Arc::new(Mutex::new(engine)), false)
        };

        println!(
            "[WorkflowExecutor] Engine from cache: {}, last_processed_event_id: {}",
            from_cache,
            engine_arc.lock().unwrap().last_processed_event_id
        );

        {
            let mut engine = engine_arc.lock().unwrap();
            engine.replay_history(&events)?;
            println!(
                "[WorkflowExecutor] After replay, last_processed_event_id: {}",
                engine.last_processed_event_id
            );
        }

        // Extract timestamps from ReplayEngine for deterministic time
        let (workflow_start_time_nanos, workflow_task_time_nanos) = {
            let engine = engine_arc.lock().unwrap();
            (
                engine.get_workflow_start_time_nanos(),
                engine.get_workflow_task_time_nanos(),
            )
        };

        // Determine deterministic "now" with priority:
        // 1. workflow_task_time_nanos (most recent DecisionTaskStarted)
        // 2. workflow_start_time_nanos (WorkflowExecutionStarted)
        // 3. task.started_timestamp (fallback if history missing)
        let current_time_nanos = workflow_task_time_nanos
            .or(workflow_start_time_nanos)
            .or(task.started_timestamp)
            .unwrap_or(0);

        // Convert start time to DateTime for WorkflowInfo
        let start_time = if let Some(start_nanos) = workflow_start_time_nanos {
            chrono::DateTime::from_timestamp(
                start_nanos / 1_000_000_000,
                (start_nanos % 1_000_000_000) as u32,
            )
            .unwrap_or_else(chrono::Utc::now)
        } else {
            // Fallback to current time if no start time found
            chrono::DateTime::from_timestamp(
                current_time_nanos / 1_000_000_000,
                (current_time_nanos % 1_000_000_000) as u32,
            )
            .unwrap_or_else(chrono::Utc::now)
        };

        let workflow_info = WorkflowInfo {
            workflow_execution: uber_cadence_core::WorkflowExecution {
                workflow_id: workflow_id.clone(),
                run_id: run_id.clone(),
            },
            workflow_type: uber_cadence_core::WorkflowType {
                name: workflow_type.clone(),
            },
            task_list: self.task_list.clone(),
            start_time,
            execution_start_to_close_timeout: std::time::Duration::from_secs(3600),
            task_start_to_close_timeout: std::time::Duration::from_secs(10),
            attempt: 1,
            continued_execution_run_id: None,
            parent_workflow_execution: None,
            cron_schedule: None,
            memo: None,
            search_attributes: None,
        };

        // Create sink with workflow_info
        let pending_local_activity_submissions = Arc::new(Mutex::new(Vec::new()));
        let local_activity_wakers = Arc::new(Mutex::new(HashMap::new()));
        let sink = Arc::new(ReplayCommandSink {
            engine: engine_arc.clone(),
            default_task_list: self.task_list.clone(),
            local_activity_queue: self.local_activity_queue.clone(),
            workflow_info: workflow_info.clone(),
            pending_local_activity_submissions: pending_local_activity_submissions.clone(),
            local_activity_wakers: local_activity_wakers.clone(),
        });

        // Extract signals and side effect caches from engine (requires lock)
        let (
            signals_map,
            cancel_requested,
            side_effect_results,
            mutable_side_effects,
            change_versions,
            local_activity_results_arc,
            is_replay,
        ) = {
            let engine = engine_arc.lock().unwrap();
            (
                engine.signals.clone(),
                engine.cancel_requested,
                engine.side_effect_results.clone(),
                engine.mutable_side_effects.clone(),
                engine.change_versions.clone(),
                engine.local_activity_results.clone(), // Clone the Arc, not the HashMap
                engine.is_replay,
            )
        };
        let signals_arc = Arc::new(Mutex::new(signals_map));
        let query_handlers_arc = Arc::new(Mutex::new(HashMap::new()));
        let side_effect_results_arc = Arc::new(Mutex::new(side_effect_results));
        let mutable_side_effects_arc = Arc::new(Mutex::new(mutable_side_effects));
        let change_versions_arc = Arc::new(Mutex::new(change_versions));

        let context = WorkflowContext::with_sink(
            workflow_info.clone(),
            sink,
            signals_arc,
            query_handlers_arc.clone(),
            side_effect_results_arc,
            mutable_side_effects_arc,
            change_versions_arc,
            local_activity_results_arc,
        );

        // Set deterministic current time
        context.set_current_time_nanos(current_time_nanos);

        // Set replay mode and cancellation
        context.set_replay_mode(is_replay);
        if cancel_requested {
            context.set_cancelled(true);
        }

        // Find workflow
        let workflow = self
            .registry
            .get_workflow(&workflow_type)
            .ok_or_else(|| CadenceError::Other(format!("Workflow not found: {}", workflow_type)))?;

        // Extract args from history (WorkflowExecutionStartedEventAttributes)
        let args = if let Some(first_event) = events.first() {
            if let Some(
                uber_cadence_proto::shared::EventAttributes::WorkflowExecutionStartedEventAttributes(
                    attrs,
                ),
            ) = &first_event.attributes
            {
                println!(
                    "[WorkflowExecutor] WorkflowStarted input length: {}",
                    attrs.input.len()
                );
                if attrs.input.is_empty() {
                    None
                } else {
                    Some(attrs.input.clone())
                }
            } else {
                println!("[WorkflowExecutor] First event is NOT WorkflowExecutionStarted. Type: {:?}, Attrs: {:?}", first_event.event_type, first_event.attributes);
                None
            }
        } else {
            println!("[WorkflowExecutor] No events in history");
            None
        };

        let workflow_future = workflow.execute(context.clone(), args);

        // Create dispatcher for managing spawned tasks
        let dispatcher = Arc::new(Mutex::new(WorkflowDispatcher::new()));
        context.set_dispatcher(dispatcher.clone());

        // Create root workflow task
        let root_task = WorkflowTask::new(0, "root".to_string(), workflow_future);

        // Add root task to dispatcher
        {
            let mut disp = dispatcher.lock().unwrap();
            disp.spawn_task(root_task);
        }

        println!("[WorkflowExecutor] Starting workflow execution polling loop");

        // Main execution loop - execute dispatcher and process local activities
        loop {
            // Execute dispatcher until all tasks blocked
            let all_done = {
                let mut disp = dispatcher.lock().unwrap();
                disp.execute_until_all_blocked()
                    .map_err(|e| CadenceError::Other(format!("Dispatcher error: {}", e)))?
            };

            // Check if root task (workflow) completed
            if all_done {
                println!("[WorkflowExecutor] Workflow Completed!");
                let mut engine = engine_arc.lock().unwrap();

                // Get result from root task
                let disp = dispatcher.lock().unwrap();
                if let Some(result_any) = disp.get_task_result(0) {
                    // Downcast to Result<Vec<u8>, WorkflowError>
                    if let Ok(result) = result_any.downcast::<Result<Vec<u8>, WorkflowError>>() {
                        match *result {
                            Ok(output) => {
                                println!("[WorkflowExecutor] Workflow Result: OK");
                                engine.decisions_helper.complete_workflow_execution(output);
                            }
                            Err(e) => {
                                println!("[WorkflowExecutor] Workflow Result: ERR - {:?}", e);
                                engine
                                    .decisions_helper
                                    .fail_workflow_execution(e.to_string(), "".to_string());
                            }
                        }
                    } else {
                        println!("[WorkflowExecutor] Failed to downcast workflow result");
                        engine
                            .decisions_helper
                            .fail_workflow_execution("Type error".to_string(), "".to_string());
                    }
                } else {
                    println!("[WorkflowExecutor] Root task result not found");
                    engine
                        .decisions_helper
                        .fail_workflow_execution("Missing result".to_string(), "".to_string());
                }
                break; // Exit loop - workflow is done
            }

            println!("[WorkflowExecutor] All tasks blocked");

            // Check if there are pending local activity submissions
            let pending_submissions = {
                let mut pending = pending_local_activity_submissions.lock().unwrap();
                std::mem::take(&mut *pending)
            };

            if !pending_submissions.is_empty() {
                use crate::local_activity_queue::LocalActivityTask;
                use uber_cadence_workflow::local_activity::{
                    encode_local_activity_marker, LocalActivityMarkerData,
                };

                println!(
                    "[WorkflowExecutor] Executing {} pending local activities",
                    pending_submissions.len()
                );

                // Execute each pending local activity
                for submission in pending_submissions {
                    println!(
                        "[WorkflowExecutor] Executing local activity: {}",
                        submission.activity_id
                    );

                    // Create oneshot channel for result
                    let (result_tx, result_rx) = tokio::sync::oneshot::channel();

                    // Create and submit local activity task
                    let task = LocalActivityTask {
                        activity_id: submission.activity_id.clone(),
                        activity_type: submission.activity_type.clone(),
                        args: submission.args,
                        options: submission.options,
                        workflow_info: workflow_info.clone(),
                        header: None,
                        attempt: 0,
                        scheduled_time: std::time::SystemTime::now(),
                        result_sender: result_tx,
                    };

                    // Submit task to queue
                    if self.local_activity_queue.send(task).is_err() {
                        println!(
                            "[WorkflowExecutor] Failed to submit local activity task: {}",
                            submission.activity_id
                        );
                        continue;
                    }

                    // Wait for result
                    match result_rx.await {
                        Ok(result) => {
                            println!(
                                "[WorkflowExecutor] Local activity {} completed: {:?}",
                                submission.activity_id,
                                result.as_ref().map(|r| r.len()).map_err(|e| e.to_string())
                            );

                            // Record marker with result
                            let marker_data = match &result {
                                Ok(result_bytes) => LocalActivityMarkerData::success(
                                    submission.activity_id.clone(),
                                    submission.activity_type.clone(),
                                    result_bytes.clone(),
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs() as i64,
                                    0, // attempt
                                ),
                                Err(err) => LocalActivityMarkerData::failure(
                                    submission.activity_id.clone(),
                                    submission.activity_type.clone(),
                                    err.to_string(),
                                    None,
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs() as i64,
                                    0, // attempt
                                ),
                            };

                            // Encode and record marker
                            let marker_details = encode_local_activity_marker(&marker_data);
                            let attrs = uber_cadence_proto::shared::RecordMarkerDecisionAttributes {
                                marker_name:
                                    uber_cadence_workflow::local_activity::LOCAL_ACTIVITY_MARKER_NAME
                                        .to_string(),
                                details: Some(marker_details),
                                header: None,
                            };

                            let marker_id =
                                format!("local_activity_{}_{}", submission.activity_id, {
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_nanos()
                                });

                            let decision = Box::new(
                                uber_cadence_workflow::state_machine::MarkerDecisionStateMachine::new(
                                    marker_id.clone(),
                                    attrs,
                                ),
                            );

                            let mut engine_lock = engine_arc.lock().unwrap();
                            engine_lock.decisions_helper.add_decision(decision);

                            // Store result in engine's local activity cache for workflow to retrieve
                            engine_lock
                                .local_activity_results
                                .lock()
                                .unwrap()
                                .insert(submission.activity_id.clone(), marker_data.clone());
                            drop(engine_lock);

                            println!("[WorkflowExecutor] Recorded marker and cached result for local activity: {}", marker_id);

                            // Signal the workflow that this local activity is complete
                            let waker = local_activity_wakers
                                .lock()
                                .unwrap()
                                .remove(&submission.activity_id);
                            if let Some(tx) = waker {
                                let result_to_send = match result {
                                    Ok(bytes) => Ok(bytes),
                                    Err(err) => Err(WorkflowError::ActivityFailed(err.to_string())),
                                };
                                let _ = tx.send(result_to_send);
                                println!(
                                    "[WorkflowExecutor] Sent result to workflow for activity: {}",
                                    submission.activity_id
                                );
                            } else {
                                println!(
                                    "[WorkflowExecutor] Warning: No waker found for activity: {}",
                                    submission.activity_id
                                );
                            }
                        }
                        Err(e) => {
                            println!(
                                "[WorkflowExecutor] Local activity {} channel closed: {:?}",
                                submission.activity_id, e
                            );
                        }
                    }
                }

                println!("[WorkflowExecutor] All pending local activities processed, re-polling workflow");
                // Continue loop to poll workflow again with the new results
                continue;
            } else {
                // No pending local activities, workflow is genuinely blocked (waiting for regular activities/timers)
                println!("[WorkflowExecutor] No pending local activities, workflow blocked on external events");
                break; // Exit loop
            }
        }

        // Handle Query
        let mut query_results = HashMap::new();
        if let Some(queries) = &task.queries {
            let handlers = query_handlers_arc.lock().unwrap();
            for (id, query) in queries {
                if let Some(handler) = handlers.get(&query.query_type) {
                    let result_data = handler(query.query_args.clone().unwrap_or_default());
                    let answer = WorkflowQueryResult {
                        query_type: Some(query.query_type.clone()),
                        answer: Some(result_data),
                        error_message: None,
                        query_result_type: QueryResultType::Answered,
                    };
                    query_results.insert(id.clone(), answer);
                } else {
                    let answer = WorkflowQueryResult {
                        query_type: Some(query.query_type.clone()),
                        answer: None,
                        error_message: Some(format!("Unknown query type: {}", query.query_type)),
                        query_result_type: QueryResultType::Failed,
                    };
                    query_results.insert(id.clone(), answer);
                }
            }
        }

        // Save to cache
        let execution = WorkflowExecution {
            engine: engine_arc.clone(),
            workflow_info,
        };

        let last_event_id = {
            let _engine = engine_arc.lock().unwrap();
            // engine.last_event_id? We don't have that in ReplayEngine yet.
            // Use last event from history?
            if let Some(last) = events.last() {
                last.event_id
            } else {
                0
            }
        };

        let cached = CachedWorkflow {
            execution: Arc::new(Mutex::new(execution)),
            last_event_id,
        };

        self.cache.insert(key, cached);

        let mut engine = engine_arc.lock().unwrap();
        let decisions = engine.decisions_helper.get_pending_decisions();

        // Verify replay determinism if in replay mode
        if engine.is_replay {
            let replay_context = ReplayContext::new(
                &workflow_type,
                &workflow_id,
                &run_id,
                &self.task_list,
                // Domain is not directly available in task, use empty string as default
                // In production, this should come from the poll response or worker config
                "",
            );

            // Find the last DecisionTaskStarted event ID to extract only new events
            let last_decision_task_completed_id = events
                .iter()
                .rev()
                .find(|e| e.event_type == EventType::DecisionTaskCompleted)
                .map(|e| e.event_id)
                .unwrap_or(0);

            // Extract events that occurred since the last decision task
            let new_history_events: Vec<HistoryEvent> = events
                .iter()
                .filter(|e| e.event_id > last_decision_task_completed_id)
                .cloned()
                .collect();

            if let Err(non_deterministic_error) =
                match_replay_with_history(&decisions, &new_history_events, &replay_context)
            {
                println!(
                    "[WorkflowExecutor] Non-determinism detected for workflow={}: {:?}",
                    workflow_id, non_deterministic_error
                );

                // For now, we log the error and continue
                // In the future, this should apply NonDeterministicWorkflowPolicy:
                // - BlockWorkflow: return error causing DecisionTaskFailed
                // - FailWorkflow: generate FailWorkflowExecution decision

                // TODO: Apply NonDeterministicWorkflowPolicy based on worker configuration
                // For now, we'll fail the workflow execution
                engine.decisions_helper.fail_workflow_execution(
                    format!(
                        "Non-deterministic workflow execution: {}",
                        non_deterministic_error
                    ),
                    "".to_string(),
                );
            }
        }

        // Mark decisions as sent to prevent them from being returned again
        engine.decisions_helper.mark_decisions_sent();

        println!(
            "[WorkflowExecutor] Generated {} decisions for workflow={}",
            decisions.len(),
            workflow_id
        );
        if let Some(d) = decisions.first() {
            println!(
                "[WorkflowExecutor] First decision type: {:?}",
                d.decision_type
            );
        }

        Ok((decisions, query_results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Workflow;
    use crate::registry::WorkflowRegistry;
    use crate::WorkerOptions;
    use uber_cadence_proto::shared::{
        EventAttributes, EventType, History, HistoryEvent, WorkflowExecutionStartedEventAttributes,
    };

    #[derive(Clone)]
    struct TestWorkflow;

    impl Workflow for TestWorkflow {
        fn execute(
            &self,
            ctx: WorkflowContext,
            input: Option<Vec<u8>>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, crate::registry::WorkflowError>> + Send>>
        {
            Box::pin(async move {
                let activity_opts = uber_cadence_core::ActivityOptions::default();
                let res = ctx
                    .execute_activity("test_activity", input, activity_opts)
                    .await
                    .map_err(|e| crate::registry::WorkflowError::ActivityFailed(e.to_string()))?;
                Ok(res)
            })
        }
    }

    #[derive(Clone)]
    struct TimeWorkflow;

    impl Workflow for TimeWorkflow {
        fn execute(
            &self,
            ctx: WorkflowContext,
            _input: Option<Vec<u8>>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, crate::registry::WorkflowError>> + Send>>
        {
            Box::pin(async move {
                // Return current time as nanoseconds
                let now = ctx.now();
                let nanos = now.timestamp_nanos_opt().unwrap_or(0);
                Ok(nanos.to_le_bytes().to_vec())
            })
        }
    }

    #[tokio::test]
    async fn test_workflow_deterministic_time() {
        let registry = Arc::new(WorkflowRegistry::new());
        registry.register_workflow("TimeWorkflow", Box::new(TimeWorkflow));

        let cache = Arc::new(WorkflowCache::new(10));
        let options = WorkerOptions::default();
        let local_activity_queue = LocalActivityQueue::new();
        let executor = WorkflowExecutor::new(
            registry,
            cache,
            options,
            "test-list".to_string(),
            local_activity_queue,
        );

        // Define timestamps in nanoseconds
        let t0_nanos = 1_000_000_000_000i64; // Workflow start time
        let t1_nanos = 2_000_000_000_000i64; // First decision task time
        let t2_nanos = 3_000_000_000_000i64; // Second decision task time

        // Create history with WorkflowExecutionStarted at T0 and DecisionTaskStarted at T1
        let events = vec![
            HistoryEvent {
                event_id: 1,
                event_type: EventType::WorkflowExecutionStarted,
                attributes: Some(EventAttributes::WorkflowExecutionStartedEventAttributes(
                    Box::new(WorkflowExecutionStartedEventAttributes {
                        workflow_type: Some(WorkflowType {
                            name: "TimeWorkflow".to_string(),
                        }),
                        parent_workflow_execution: None,
                        task_list: Some(uber_cadence_proto::shared::TaskList {
                            name: "test-list".to_string(),
                            kind: uber_cadence_proto::shared::TaskListKind::Normal,
                        }),
                        input: vec![],
                        execution_start_to_close_timeout_seconds: 10,
                        task_start_to_close_timeout_seconds: 10,
                        identity: "test-identity".to_string(),
                        continued_execution_run_id: None,
                        initiator: None,
                        continued_failure_details: None,
                        last_completion_result: None,
                        original_execution_run_id: None,
                        first_execution_run_id: None,
                        retry_policy: None,
                        attempt: 0,
                        expiration_timestamp: None,
                        cron_schedule: None,
                        first_decision_task_backoff_seconds: 0,
                    }),
                )),
                timestamp: t0_nanos,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 2,
                event_type: EventType::DecisionTaskScheduled,
                attributes: Some(EventAttributes::DecisionTaskScheduledEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::DecisionTaskScheduledEventAttributes {
                            task_list: Some(uber_cadence_proto::shared::TaskList {
                                name: "test-list".to_string(),
                                kind: uber_cadence_proto::shared::TaskListKind::Normal,
                            }),
                            start_to_close_timeout_seconds: 10,
                            attempt: 0,
                        },
                    ),
                )),
                timestamp: t0_nanos,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 3,
                event_type: EventType::DecisionTaskStarted,
                attributes: Some(EventAttributes::DecisionTaskStartedEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::DecisionTaskStartedEventAttributes {
                            scheduled_event_id: 2,
                            identity: "test-worker".to_string(),
                            request_id: "req-1".to_string(),
                        },
                    ),
                )),
                timestamp: t1_nanos,
                version: 0,
                task_id: 0,
            },
        ];

        let task = PollForDecisionTaskResponse {
            task_token: vec![1],
            workflow_execution: Some(uber_cadence_proto::shared::WorkflowExecution {
                workflow_id: "time-wf".to_string(),
                run_id: "time-run".to_string(),
            }),
            workflow_type: Some(WorkflowType {
                name: "TimeWorkflow".to_string(),
            }),
            history: Some(History {
                events: events.clone(),
            }),
            previous_started_event_id: 0,
            started_event_id: 3,
            attempt: 0,
            backlog_count_hint: 0,
            queries: None,
            next_page_token: Some(vec![]),
            query: None,
            workflow_execution_task_list: None,
            scheduled_timestamp: Some(t1_nanos),
            started_timestamp: Some(t1_nanos),
        };

        let result = executor.execute_decision_task(task).await.unwrap();
        let (decisions, _) = result;

        // Should complete workflow with the timestamp
        assert_eq!(decisions.len(), 1);
        let decision = &decisions[0];
        assert_eq!(
            decision.decision_type,
            uber_cadence_proto::shared::DecisionType::CompleteWorkflowExecution
        );

        // Check that the returned time is T1 (decision task time)
        if let Some(
            uber_cadence_proto::shared::DecisionAttributes::CompleteWorkflowExecutionDecisionAttributes(
                attr,
            ),
        ) = &decision.attributes
        {
            let result_bytes = attr.result.as_ref().unwrap();
            let returned_nanos = i64::from_le_bytes(result_bytes.as_slice().try_into().unwrap());
            assert_eq!(
                returned_nanos, t1_nanos,
                "Expected workflow time to be T1 (decision task time)"
            );
        } else {
            panic!(
                "Expected CompleteWorkflowExecution attributes, found {:?}",
                decision.attributes
            );
        }

        // Test with a second decision task at T2
        let events_with_t2 = vec![
            events[0].clone(),
            events[1].clone(),
            events[2].clone(),
            HistoryEvent {
                event_id: 4,
                event_type: EventType::DecisionTaskCompleted,
                attributes: Some(EventAttributes::DecisionTaskCompletedEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::DecisionTaskCompletedEventAttributes {
                            scheduled_event_id: 2,
                            started_event_id: 3,
                            identity: "test-worker".to_string(),
                            binary_checksum: "checksum".to_string(),
                        },
                    ),
                )),
                timestamp: t1_nanos,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 5,
                event_type: EventType::DecisionTaskScheduled,
                attributes: Some(EventAttributes::DecisionTaskScheduledEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::DecisionTaskScheduledEventAttributes {
                            task_list: Some(uber_cadence_proto::shared::TaskList {
                                name: "test-list".to_string(),
                                kind: uber_cadence_proto::shared::TaskListKind::Normal,
                            }),
                            start_to_close_timeout_seconds: 10,
                            attempt: 0,
                        },
                    ),
                )),
                timestamp: t1_nanos,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 6,
                event_type: EventType::DecisionTaskStarted,
                attributes: Some(EventAttributes::DecisionTaskStartedEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::DecisionTaskStartedEventAttributes {
                            scheduled_event_id: 5,
                            identity: "test-worker".to_string(),
                            request_id: "req-2".to_string(),
                        },
                    ),
                )),
                timestamp: t2_nanos,
                version: 0,
                task_id: 0,
            },
        ];

        let task2 = PollForDecisionTaskResponse {
            task_token: vec![2],
            workflow_execution: Some(uber_cadence_proto::shared::WorkflowExecution {
                workflow_id: "time-wf-2".to_string(), // Different workflow ID
                run_id: "time-run-2".to_string(),
            }),
            workflow_type: Some(WorkflowType {
                name: "TimeWorkflow".to_string(),
            }),
            history: Some(History {
                events: events_with_t2,
            }),
            previous_started_event_id: 3,
            started_event_id: 6,
            attempt: 0,
            backlog_count_hint: 0,
            queries: None,
            next_page_token: Some(vec![]),
            query: None,
            workflow_execution_task_list: None,
            scheduled_timestamp: Some(t2_nanos),
            started_timestamp: Some(t2_nanos),
        };

        let result2 = executor.execute_decision_task(task2).await.unwrap();
        let (decisions2, _) = result2;

        // Should complete workflow with the timestamp T2
        assert_eq!(decisions2.len(), 1);
        let decision2 = &decisions2[0];
        assert_eq!(
            decision2.decision_type,
            uber_cadence_proto::shared::DecisionType::CompleteWorkflowExecution
        );

        // Check that the returned time is T2 (second decision task time)
        if let Some(
            uber_cadence_proto::shared::DecisionAttributes::CompleteWorkflowExecutionDecisionAttributes(
                attr,
            ),
        ) = &decision2.attributes
        {
            let result_bytes = attr.result.as_ref().unwrap();
            let returned_nanos = i64::from_le_bytes(result_bytes.as_slice().try_into().unwrap());
            assert_eq!(
                returned_nanos, t2_nanos,
                "Expected workflow time to be T2 (second decision task time)"
            );
        } else {
            panic!(
                "Expected CompleteWorkflowExecution attributes, found {:?}",
                decision2.attributes
            );
        }
    }

    #[tokio::test]
    async fn test_workflow_executor_replay() {
        let registry = Arc::new(WorkflowRegistry::new());
        registry.register_workflow("TestWorkflow", Box::new(TestWorkflow));

        let cache = Arc::new(WorkflowCache::new(10));
        let options = WorkerOptions::default();
        let local_activity_queue = LocalActivityQueue::new();
        let executor = WorkflowExecutor::new(
            registry,
            cache,
            options,
            "test-list".to_string(),
            local_activity_queue,
        );

        // Create history with started event + activity scheduled/completed
        let events = vec![
            HistoryEvent {
                event_id: 1,
                event_type: EventType::WorkflowExecutionStarted,
                attributes: Some(EventAttributes::WorkflowExecutionStartedEventAttributes(
                    Box::new(WorkflowExecutionStartedEventAttributes {
                        workflow_type: Some(WorkflowType {
                            name: "TestWorkflow".to_string(),
                        }),
                        parent_workflow_execution: None,
                        task_list: Some(uber_cadence_proto::shared::TaskList {
                            name: "test-list".to_string(),
                            kind: uber_cadence_proto::shared::TaskListKind::Normal,
                        }),
                        input: vec![1, 2, 3],
                        execution_start_to_close_timeout_seconds: 10,
                        task_start_to_close_timeout_seconds: 10,
                        identity: "test-identity".to_string(),
                        continued_execution_run_id: None,
                        initiator: None,
                        continued_failure_details: None,
                        last_completion_result: None,
                        original_execution_run_id: None,
                        first_execution_run_id: None,
                        retry_policy: None,
                        attempt: 0,
                        expiration_timestamp: None,
                        cron_schedule: None,
                        first_decision_task_backoff_seconds: 0,
                    }),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 2,
                event_type: EventType::DecisionTaskScheduled,
                attributes: Some(EventAttributes::DecisionTaskScheduledEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::DecisionTaskScheduledEventAttributes {
                            task_list: Some(uber_cadence_proto::shared::TaskList {
                                name: "test-list".to_string(),
                                kind: uber_cadence_proto::shared::TaskListKind::Normal,
                            }),
                            start_to_close_timeout_seconds: 10,
                            attempt: 0,
                        },
                    ),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 3,
                event_type: EventType::DecisionTaskStarted,
                attributes: Some(EventAttributes::DecisionTaskStartedEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::DecisionTaskStartedEventAttributes {
                            scheduled_event_id: 2,
                            identity: "test-worker".to_string(),
                            request_id: "req-1".to_string(),
                        },
                    ),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 4,
                event_type: EventType::DecisionTaskCompleted,
                attributes: Some(EventAttributes::DecisionTaskCompletedEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::DecisionTaskCompletedEventAttributes {
                            scheduled_event_id: 2,
                            started_event_id: 3,
                            identity: "test-worker".to_string(),
                            binary_checksum: "checksum".to_string(),
                        },
                    ),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 5,
                event_type: EventType::ActivityTaskScheduled,
                attributes: Some(EventAttributes::ActivityTaskScheduledEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::ActivityTaskScheduledEventAttributes {
                            activity_id: "0".to_string(), // ID generated by sequence is 0
                            activity_type: Some(ActivityType {
                                name: "test_activity".to_string(),
                            }),
                            task_list: Some(uber_cadence_proto::shared::TaskList {
                                name: "test-list".to_string(),
                                kind: uber_cadence_proto::shared::TaskListKind::Normal,
                            }),
                            input: None,
                            schedule_to_close_timeout_seconds: None,
                            schedule_to_start_timeout_seconds: None,
                            start_to_close_timeout_seconds: None,
                            heartbeat_timeout_seconds: None,
                            decision_task_completed_event_id: 4,
                            retry_policy: None,
                        },
                    ),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 6,
                event_type: EventType::ActivityTaskStarted,
                attributes: Some(EventAttributes::ActivityTaskStartedEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::ActivityTaskStartedEventAttributes {
                            scheduled_event_id: 5,
                            identity: "test-worker".to_string(),
                            request_id: "req-2".to_string(),
                            attempt: 0,
                            last_failure_details: None,
                        },
                    ),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 7,
                event_type: EventType::ActivityTaskCompleted,
                attributes: Some(EventAttributes::ActivityTaskCompletedEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::ActivityTaskCompletedEventAttributes {
                            scheduled_event_id: 5,
                            started_event_id: 6,
                            result: Some(vec![4, 5, 6]),
                            identity: "test-worker".to_string(),
                        },
                    ),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 8,
                event_type: EventType::DecisionTaskScheduled,
                attributes: Some(EventAttributes::DecisionTaskScheduledEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::DecisionTaskScheduledEventAttributes {
                            task_list: Some(uber_cadence_proto::shared::TaskList {
                                name: "test-list".to_string(),
                                kind: uber_cadence_proto::shared::TaskListKind::Normal,
                            }),
                            start_to_close_timeout_seconds: 10,
                            attempt: 0,
                        },
                    ),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 9,
                event_type: EventType::DecisionTaskStarted,
                attributes: Some(EventAttributes::DecisionTaskStartedEventAttributes(
                    Box::new(
                        uber_cadence_proto::shared::DecisionTaskStartedEventAttributes {
                            scheduled_event_id: 8,
                            identity: "test-worker".to_string(),
                            request_id: "req-3".to_string(),
                        },
                    ),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
        ];

        let task = PollForDecisionTaskResponse {
            task_token: vec![1],
            workflow_execution: Some(uber_cadence_proto::shared::WorkflowExecution {
                workflow_id: "w1".to_string(),
                run_id: "r1".to_string(),
            }),
            workflow_type: Some(WorkflowType {
                name: "TestWorkflow".to_string(),
            }),
            history: Some(History { events }),
            previous_started_event_id: 0,
            started_event_id: 9,
            attempt: 0,
            backlog_count_hint: 0,
            queries: None,
            next_page_token: Some(vec![]),
            query: None,
            workflow_execution_task_list: None,
            scheduled_timestamp: Some(0),
            started_timestamp: Some(0),
        };

        let result = executor.execute_decision_task(task).await.unwrap();
        let (decisions, _) = result;

        // Should complete workflow with result
        assert_eq!(decisions.len(), 1);
        let decision = &decisions[0];
        assert_eq!(
            decision.decision_type,
            uber_cadence_proto::shared::DecisionType::CompleteWorkflowExecution
        );

        // Check attributes via pattern matching
        if let Some(
            uber_cadence_proto::shared::DecisionAttributes::CompleteWorkflowExecutionDecisionAttributes(
                attr,
            ),
        ) = &decision.attributes
        {
            assert_eq!(attr.result, Some(vec![4, 5, 6]));
        } else {
            panic!(
                "Expected CompleteWorkflowExecution attributes, found {:?}",
                decision.attributes
            );
        }
    }
}
