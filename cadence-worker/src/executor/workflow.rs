use crate::executor::cache::{CachedWorkflow, WorkflowCache, WorkflowExecutionKey};
use crate::executor::replay::ReplayEngine;
use crate::registry::Registry;
use crate::WorkerOptions;
use cadence_core::{CadenceError, WorkflowInfo};
use cadence_proto::shared::{
    ActivityType, ContinueAsNewWorkflowExecutionDecisionAttributes, Decision,
    RequestCancelExternalWorkflowExecutionDecisionAttributes,
    ScheduleActivityTaskDecisionAttributes, SignalExternalWorkflowExecutionDecisionAttributes,
    StartChildWorkflowExecutionDecisionAttributes, StartTimerDecisionAttributes, TaskList,
    TaskListKind, WorkflowType,
};
use cadence_proto::workflow_service::PollForDecisionTaskResponse;
use cadence_proto::{QueryResultType, WorkflowQueryResult};
use cadence_workflow::commands::WorkflowCommand;
use cadence_workflow::context::{CommandSink, WorkflowContext};
use cadence_workflow::future::WorkflowError;
use cadence_workflow::state_machine::{
    ActivityDecisionStateMachine, ChildWorkflowDecisionStateMachine, DecisionId,
    StateMachineDecisionType, TimerDecisionStateMachine,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

struct ReplayCommandSink {
    engine: Arc<Mutex<ReplayEngine>>,
    default_task_list: String,
}

impl CommandSink for ReplayCommandSink {
    fn submit(
        &self,
        command: WorkflowCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        let engine = self.engine.clone();
        let default_task_list = self.default_task_list.clone();

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
                            cadence_proto::shared::RetryPolicy {
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
                        workflow_type: Some(cadence_proto::shared::WorkflowType {
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
                            cadence_proto::shared::RetryPolicy {
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
                            cadence_proto::shared::RetryPolicy {
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
                        workflow_execution: Some(cadence_proto::shared::WorkflowExecution {
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
                        workflow_execution: Some(cadence_proto::shared::WorkflowExecution {
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
                _ => unimplemented!("Command not supported"),
            }
            std::future::pending().await
        })
    }
}

/// Workflow executor
pub struct WorkflowExecutor {
    #[allow(dead_code)]
    registry: Arc<dyn Registry>,
    #[allow(dead_code)]
    cache: Arc<WorkflowCache>,
    #[allow(dead_code)]
    options: WorkerOptions,
    task_list: String,
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
    ) -> Self {
        Self {
            registry,
            cache,
            options,
            task_list,
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

        let sink = Arc::new(ReplayCommandSink {
            engine: engine_arc.clone(),
            default_task_list: self.task_list.clone(),
        });

        let workflow_info = WorkflowInfo {
            workflow_execution: cadence_core::WorkflowExecution {
                workflow_id: workflow_id.clone(),
                run_id: run_id.clone(),
            },
            workflow_type: cadence_core::WorkflowType {
                name: workflow_type.clone(),
            },
            task_list: self.task_list.clone(),
            start_time: chrono::Utc::now(),
            execution_start_to_close_timeout: std::time::Duration::from_secs(3600),
            task_start_to_close_timeout: std::time::Duration::from_secs(10),
            attempt: 1,
            continued_execution_run_id: None,
            parent_workflow_execution: None,
            cron_schedule: None,
            memo: None,
            search_attributes: None,
        };

        // Extract signals from engine (requires lock)
        let (signals_map, cancel_requested) = {
            let engine = engine_arc.lock().unwrap();
            (engine.signals.clone(), engine.cancel_requested)
        };
        let signals_arc = Arc::new(Mutex::new(signals_map));
        let query_handlers_arc = Arc::new(Mutex::new(HashMap::new()));

        let context = WorkflowContext::with_sink(
            workflow_info.clone(),
            sink,
            signals_arc,
            query_handlers_arc.clone(),
        );
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
                cadence_proto::shared::EventAttributes::WorkflowExecutionStartedEventAttributes(
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

        let mut workflow_future = workflow.execute(context, args);

        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        println!("[WorkflowExecutor] Starting workflow execution polling");
        match workflow_future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(result) => {
                println!("[WorkflowExecutor] Workflow Completed!");
                let mut engine = engine_arc.lock().unwrap();
                match result {
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
            }
            std::task::Poll::Pending => {
                println!("[WorkflowExecutor] Workflow Pending (Blocked)");
                // Blocked, return pending decisions
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
    use cadence_proto::shared::{
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
                let activity_opts = cadence_core::ActivityOptions::default();
                let res = ctx
                    .execute_activity("test_activity", input, activity_opts)
                    .await
                    .map_err(|e| crate::registry::WorkflowError::ActivityFailed(e.to_string()))?;
                Ok(res)
            })
        }

        fn clone_box(&self) -> Box<dyn Workflow> {
            Box::new(self.clone())
        }
    }

    #[tokio::test]
    async fn test_workflow_executor_replay() {
        let registry = Arc::new(WorkflowRegistry::new());
        registry.register_workflow("TestWorkflow", Box::new(TestWorkflow));

        let cache = Arc::new(WorkflowCache::new(10));
        let options = WorkerOptions::default();
        let executor = WorkflowExecutor::new(registry, cache, options, "test-list".to_string());

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
                        task_list: Some(cadence_proto::shared::TaskList {
                            name: "test-list".to_string(),
                            kind: cadence_proto::shared::TaskListKind::Normal,
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
                        cadence_proto::shared::DecisionTaskScheduledEventAttributes {
                            task_list: Some(cadence_proto::shared::TaskList {
                                name: "test-list".to_string(),
                                kind: cadence_proto::shared::TaskListKind::Normal,
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
                    Box::new(cadence_proto::shared::DecisionTaskStartedEventAttributes {
                        scheduled_event_id: 2,
                        identity: "test-worker".to_string(),
                        request_id: "req-1".to_string(),
                    }),
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
                        cadence_proto::shared::DecisionTaskCompletedEventAttributes {
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
                        cadence_proto::shared::ActivityTaskScheduledEventAttributes {
                            activity_id: "0".to_string(), // ID generated by sequence is 0
                            activity_type: Some(ActivityType {
                                name: "test_activity".to_string(),
                            }),
                            task_list: Some(cadence_proto::shared::TaskList {
                                name: "test-list".to_string(),
                                kind: cadence_proto::shared::TaskListKind::Normal,
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
                    Box::new(cadence_proto::shared::ActivityTaskStartedEventAttributes {
                        scheduled_event_id: 5,
                        identity: "test-worker".to_string(),
                        request_id: "req-2".to_string(),
                        attempt: 0,
                        last_failure_details: None,
                    }),
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
                        cadence_proto::shared::ActivityTaskCompletedEventAttributes {
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
                        cadence_proto::shared::DecisionTaskScheduledEventAttributes {
                            task_list: Some(cadence_proto::shared::TaskList {
                                name: "test-list".to_string(),
                                kind: cadence_proto::shared::TaskListKind::Normal,
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
                    Box::new(cadence_proto::shared::DecisionTaskStartedEventAttributes {
                        scheduled_event_id: 8,
                        identity: "test-worker".to_string(),
                        request_id: "req-3".to_string(),
                    }),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
        ];

        let task = PollForDecisionTaskResponse {
            task_token: vec![1],
            workflow_execution: Some(cadence_proto::shared::WorkflowExecution {
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
            cadence_proto::shared::DecisionType::CompleteWorkflowExecution
        );

        // Check attributes via pattern matching
        if let Some(
            cadence_proto::shared::DecisionAttributes::CompleteWorkflowExecutionDecisionAttributes(
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
