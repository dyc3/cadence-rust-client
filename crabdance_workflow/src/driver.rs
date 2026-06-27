//! In-memory workflow driver loop with a mock clock and a pluggable command resolver.
//!
//! This is the shared, in-process foundation that powers both the replay harness
//! (resolve commands from recorded history) and the unit-test suite (resolve by
//! running or mocking activities). It mirrors the production worker loop
//! (`crabdance_worker`'s executor): it drives the *real* [`WorkflowContext`] and
//! [`WorkflowDispatcher`] to "all tasks blocked", and — when the workflow is blocked
//! only on timers — auto-fires the earliest timer on a deterministic mock clock. This
//! is the Rust analogue of the Go client's `startMainLoop` / `autoFireNextTimer`.
//!
//! The seam is the existing [`CommandSink`] trait: every workflow operation
//! (`execute_activity`, `new_timer`, `execute_child_workflow`, signals, markers) is
//! submitted through it. Production supplies a server-backed sink; here
//! [`InMemoryCommandSink`] resolves timers against the mock clock, acknowledges
//! markers, and delegates everything else to a caller-supplied [`CommandResolver`].

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use crabdance_core::WorkflowInfo;
use serde::{Deserialize, Serialize};

use crate::commands::{StartTimerCommand, WorkflowCommand};
use crate::context::{CommandSink, WorkflowContext, WorkflowContextBuilder};
use crate::dispatcher::{WorkflowDispatcher, WorkflowTask};
use crate::future::DefaultWorkflowError;

/// Safety bound on driver iterations to surface a non-terminating workflow as a
/// clear panic rather than an infinite hang in a test.
const MAX_DRIVER_ITERATIONS: usize = 1_000_000;

/// An ordered, owned summary of a command submitted through the sink.
///
/// `WorkflowCommand` is not `Clone`, so the driver records this lightweight
/// projection of each submission. The recorded sequence is what the replay harness
/// asserts against history (the emitted-command sequence) and what the testsuite
/// inspects to verify a workflow scheduled the operations it was expected to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandRecord {
    ScheduleActivity {
        activity_id: String,
        activity_type: String,
    },
    ScheduleLocalActivity {
        activity_id: String,
        activity_type: String,
    },
    StartChildWorkflow {
        workflow_id: String,
        workflow_type: String,
    },
    StartTimer {
        timer_id: String,
        duration_millis: u128,
    },
    CancelTimer {
        timer_id: String,
    },
    SignalExternalWorkflow {
        workflow_id: String,
        signal_name: String,
    },
    RequestCancelExternalWorkflow {
        workflow_id: String,
    },
    RecordMarker {
        marker_name: String,
    },
    UpsertSearchAttributes {
        keys: Vec<String>,
    },
    ContinueAsNewWorkflow {
        workflow_type: String,
    },
    CompleteWorkflow,
    FailWorkflow,
    CancelWorkflow,
}

impl CommandRecord {
    fn from_command(command: &WorkflowCommand) -> Self {
        match command {
            WorkflowCommand::ScheduleActivity(c) => CommandRecord::ScheduleActivity {
                activity_id: c.activity_id.clone(),
                activity_type: c.activity_type.clone(),
            },
            WorkflowCommand::ScheduleLocalActivity(c) => CommandRecord::ScheduleLocalActivity {
                activity_id: c.activity_id.clone(),
                activity_type: c.activity_type.clone(),
            },
            WorkflowCommand::StartChildWorkflow(c) => CommandRecord::StartChildWorkflow {
                workflow_id: c.workflow_id.clone(),
                workflow_type: c.workflow_type.clone(),
            },
            WorkflowCommand::StartTimer(c) => CommandRecord::StartTimer {
                timer_id: c.timer_id.clone(),
                duration_millis: c.duration.as_millis(),
            },
            WorkflowCommand::CancelTimer(c) => CommandRecord::CancelTimer {
                timer_id: c.timer_id.clone(),
            },
            WorkflowCommand::SignalExternalWorkflow(c) => CommandRecord::SignalExternalWorkflow {
                workflow_id: c.workflow_id.clone(),
                signal_name: c.signal_name.clone(),
            },
            WorkflowCommand::RequestCancelExternalWorkflow(c) => {
                CommandRecord::RequestCancelExternalWorkflow {
                    workflow_id: c.workflow_id.clone(),
                }
            }
            WorkflowCommand::RecordMarker(c) => CommandRecord::RecordMarker {
                marker_name: c.marker_name.clone(),
            },
            WorkflowCommand::UpsertSearchAttributes(c) => CommandRecord::UpsertSearchAttributes {
                keys: c.search_attributes.iter().map(|(k, _)| k.clone()).collect(),
            },
            WorkflowCommand::ContinueAsNewWorkflow(c) => CommandRecord::ContinueAsNewWorkflow {
                workflow_type: c.workflow_type.clone(),
            },
            WorkflowCommand::CompleteWorkflow(_) => CommandRecord::CompleteWorkflow,
            WorkflowCommand::FailWorkflow(_) => CommandRecord::FailWorkflow,
            WorkflowCommand::CancelWorkflow(_) => CommandRecord::CancelWorkflow,
        }
    }
}

/// How a [`CommandResolver`] resolves a single non-timer command.
pub enum Resolution {
    /// The command resolves immediately to this result. `Ok(bytes)` unblocks the
    /// awaiting workflow task with the given payload; `Err` fails the operation.
    Done(Result<Vec<u8>, DefaultWorkflowError>),
    /// The command blocks indefinitely (e.g. awaiting a signal that never arrives in
    /// this run). The driver will report [`DriverOutcome::Blocked`] once no timer can
    /// make further progress.
    Blocked,
}

/// Resolves the non-timer, non-marker commands a workflow submits during an
/// in-memory driver run.
///
/// Timers are resolved by the driver's mock clock and markers are acknowledged
/// automatically, so implementors only handle activities, local activities, child
/// workflows, external signals/cancels and continue-as-new. The replay harness
/// implements this by looking results up in recorded history; the testsuite
/// implements it by running or mocking activities.
pub trait CommandResolver: Send + Sync {
    fn resolve(&self, command: &WorkflowCommand) -> Resolution;
}

/// Blanket impl so a closure can be used directly as a resolver.
impl<F> CommandResolver for F
where
    F: Fn(&WorkflowCommand) -> Resolution + Send + Sync,
{
    fn resolve(&self, command: &WorkflowCommand) -> Resolution {
        self(command)
    }
}

struct TimerEntry {
    deadline_nanos: i64,
    fired: bool,
}

/// Details of a continue-as-new emitted by the workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuedAsNew {
    pub workflow_type: String,
    pub input: Option<Vec<u8>>,
}

struct SinkInner {
    /// Current mock-clock time in unix-epoch nanoseconds.
    clock_nanos: i64,
    /// Pending and fired timers keyed by insertion order; deadline ties break by key
    /// so firing order is deterministic.
    timers: BTreeMap<u64, TimerEntry>,
    next_timer_key: u64,
    /// Ordered log of every submitted command.
    commands: Vec<CommandRecord>,
    /// Set when the workflow emits a `ContinueAsNewWorkflow` command. This is a terminal
    /// outcome: `continue_as_new` parks the root task forever after submitting, so
    /// without this the driver would otherwise mistake it for a blocked workflow.
    continued_as_new: Option<ContinuedAsNew>,
}

/// A [`CommandSink`] that resolves timers against a mock clock, acknowledges markers,
/// and delegates all other commands to a [`CommandResolver`].
pub struct InMemoryCommandSink {
    inner: Arc<Mutex<SinkInner>>,
    resolver: Arc<dyn CommandResolver>,
}

impl InMemoryCommandSink {
    fn new(start_nanos: i64, resolver: Arc<dyn CommandResolver>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SinkInner {
                clock_nanos: start_nanos,
                timers: BTreeMap::new(),
                next_timer_key: 0,
                commands: Vec::new(),
                continued_as_new: None,
            })),
            resolver,
        }
    }

    /// The ordered sequence of commands submitted so far.
    pub fn recorded_commands(&self) -> Vec<CommandRecord> {
        self.inner.lock().unwrap().commands.clone()
    }

    /// Current mock-clock time in unix-epoch nanoseconds.
    pub fn clock_nanos(&self) -> i64 {
        self.inner.lock().unwrap().clock_nanos
    }

    /// The earliest unfired timer as `(key, deadline_nanos)`, if any.
    fn earliest_unfired_timer(&self) -> Option<(u64, i64)> {
        let inner = self.inner.lock().unwrap();
        inner
            .timers
            .iter()
            .filter(|(_, t)| !t.fired)
            .min_by_key(|(key, t)| (t.deadline_nanos, **key))
            .map(|(key, t)| (*key, t.deadline_nanos))
    }

    /// The continue-as-new the workflow emitted, if any (a terminal outcome).
    fn continued_as_new(&self) -> Option<ContinuedAsNew> {
        self.inner.lock().unwrap().continued_as_new.clone()
    }

    /// Fire the given timer and advance the mock clock to `new_clock_nanos`.
    fn fire_timer(&self, key: u64, new_clock_nanos: i64) {
        let mut inner = self.inner.lock().unwrap();
        inner.clock_nanos = new_clock_nanos;
        if let Some(timer) = inner.timers.get_mut(&key) {
            timer.fired = true;
        }
    }
}

impl CommandSink for InMemoryCommandSink {
    fn submit(
        &self,
        command: WorkflowCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, DefaultWorkflowError>> + Send>> {
        {
            let mut inner = self.inner.lock().unwrap();
            inner.commands.push(CommandRecord::from_command(&command));
        }

        match command {
            WorkflowCommand::StartTimer(StartTimerCommand { duration, .. }) => {
                let key = {
                    let mut inner = self.inner.lock().unwrap();
                    let key = inner.next_timer_key;
                    inner.next_timer_key += 1;
                    let deadline = inner.clock_nanos.saturating_add(duration.as_nanos() as i64);
                    inner.timers.insert(
                        key,
                        TimerEntry {
                            deadline_nanos: deadline,
                            fired: false,
                        },
                    );
                    key
                };
                Box::pin(InMemoryTimerFuture {
                    inner: self.inner.clone(),
                    key,
                })
            }
            // Markers (side effects, mutable side effects, version) and search-attribute
            // upserts are deterministic acknowledgements in an in-memory run.
            WorkflowCommand::RecordMarker(_) | WorkflowCommand::UpsertSearchAttributes(_) => {
                Box::pin(async { Ok(Vec::new()) })
            }
            // Continue-as-new is terminal: record it (the workflow then parks forever)
            // and acknowledge so the submitting `.await` resolves.
            WorkflowCommand::ContinueAsNewWorkflow(c) => {
                let mut inner = self.inner.lock().unwrap();
                inner.continued_as_new = Some(ContinuedAsNew {
                    workflow_type: c.workflow_type,
                    input: c.input,
                });
                Box::pin(async { Ok(Vec::new()) })
            }
            other => match self.resolver.resolve(&other) {
                Resolution::Done(result) => Box::pin(async move { result }),
                Resolution::Blocked => Box::pin(std::future::pending()),
            },
        }
    }
}

/// Future returned for a timer submission; resolves once the driver fires the timer.
struct InMemoryTimerFuture {
    inner: Arc<Mutex<SinkInner>>,
    key: u64,
}

impl Future for InMemoryTimerFuture {
    type Output = Result<Vec<u8>, DefaultWorkflowError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.inner.lock().unwrap();
        match inner.timers.get(&self.key) {
            Some(timer) if timer.fired => Poll::Ready(Ok(Vec::new())),
            _ => Poll::Pending,
        }
    }
}

/// The result of driving a workflow with [`WorkflowDriver::run`].
#[derive(Debug)]
pub enum DriverOutcome {
    /// The workflow's root task completed with this result.
    Completed(Result<Vec<u8>, DefaultWorkflowError>),
    /// The workflow terminated by continuing as new (a terminal outcome, not a block).
    ContinuedAsNew(ContinuedAsNew),
    /// The workflow is blocked on an external event (e.g. a signal) that this run
    /// will never deliver, with no timer left to fire.
    Blocked,
}

impl DriverOutcome {
    /// Returns the completion result, or `None` if the run continued-as-new or blocked.
    pub fn into_completed(self) -> Option<Result<Vec<u8>, DefaultWorkflowError>> {
        match self {
            DriverOutcome::Completed(result) => Some(result),
            DriverOutcome::ContinuedAsNew(_) | DriverOutcome::Blocked => None,
        }
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, DriverOutcome::Blocked)
    }

    /// The continue-as-new details if the workflow terminated by continuing as new.
    pub fn continued_as_new(&self) -> Option<&ContinuedAsNew> {
        match self {
            DriverOutcome::ContinuedAsNew(c) => Some(c),
            _ => None,
        }
    }
}

/// Drives a real [`WorkflowContext`] / [`WorkflowDispatcher`] to completion in-process.
///
/// Build it with a [`CommandResolver`], optionally configure the [`context`](Self::context)
/// (seed replay caches, set replay mode, register query handlers), then [`run`](Self::run)
/// the workflow future.
pub struct WorkflowDriver {
    context: WorkflowContext,
    dispatcher: Arc<Mutex<WorkflowDispatcher>>,
    sink: Arc<InMemoryCommandSink>,
}

impl WorkflowDriver {
    /// Create a driver for `workflow_info`, resolving commands with `resolver`.
    ///
    /// The mock clock starts at `workflow_info.start_time`.
    pub fn new(workflow_info: WorkflowInfo, resolver: Arc<dyn CommandResolver>) -> Self {
        let start_nanos = workflow_info.start_time.timestamp_nanos_opt().unwrap_or(0);
        let sink = Arc::new(InMemoryCommandSink::new(start_nanos, resolver));

        let context =
            WorkflowContextBuilder::new(workflow_info, sink.clone() as Arc<dyn CommandSink>)
                .build();
        context.set_current_time_nanos(start_nanos);

        let dispatcher = Arc::new(Mutex::new(WorkflowDispatcher::new()));
        context.set_dispatcher(dispatcher.clone());

        Self {
            context,
            dispatcher,
            sink,
        }
    }

    /// A clone of the workflow context to pass to the workflow function. Use it to
    /// seed replay state (`set_replay_mode`, `set_change_versions`, …) before `run`.
    pub fn context(&self) -> WorkflowContext {
        self.context.clone()
    }

    /// Access the in-memory sink (recorded commands, mock-clock time).
    pub fn sink(&self) -> &InMemoryCommandSink {
        &self.sink
    }

    /// The ordered sequence of commands the workflow has submitted.
    pub fn recorded_commands(&self) -> Vec<CommandRecord> {
        self.sink.recorded_commands()
    }

    /// Current mock-clock time in unix-epoch nanoseconds.
    pub fn now_nanos(&self) -> i64 {
        self.sink.clock_nanos()
    }

    /// Run `root` (the workflow function applied to [`context`](Self::context)) to
    /// completion, auto-firing timers on the mock clock whenever the workflow blocks
    /// solely on them.
    pub fn run<F>(&self, root: F) -> DriverOutcome
    where
        F: Future<Output = Result<Vec<u8>, DefaultWorkflowError>> + Send + 'static,
    {
        {
            let mut dispatcher = self.dispatcher.lock().unwrap();
            dispatcher.spawn_task(WorkflowTask::new(0, "root".to_string(), root));
        }

        for _ in 0..MAX_DRIVER_ITERATIONS {
            let all_done = {
                let mut dispatcher = self.dispatcher.lock().unwrap();
                dispatcher
                    .execute_until_all_blocked()
                    .expect("dispatcher should not already be executing")
            };

            if all_done {
                let dispatcher = self.dispatcher.lock().unwrap();
                let result_any = dispatcher
                    .get_task_result(0)
                    .expect("root task result missing after completion");
                let result = *result_any
                    .downcast::<Result<Vec<u8>, DefaultWorkflowError>>()
                    .expect("root task output must be Result<Vec<u8>, DefaultWorkflowError>");
                return DriverOutcome::Completed(result);
            }

            // Continue-as-new is terminal: the root task parks forever after emitting
            // the command, so treat it as a completed (continued) run rather than blocked.
            if let Some(continued) = self.sink.continued_as_new() {
                return DriverOutcome::ContinuedAsNew(continued);
            }

            // Blocked: advance the mock clock to the earliest pending timer, if any.
            if let Some((key, deadline)) = self.sink.earliest_unfired_timer() {
                let new_clock = deadline.max(self.sink.clock_nanos());
                self.sink.fire_timer(key, new_clock);
                self.context.set_current_time_nanos(new_clock);
                continue;
            }

            // No timer can make progress: the workflow is blocked on external input.
            return DriverOutcome::Blocked;
        }

        panic!(
            "workflow driver exceeded {} iterations; the workflow may not terminate",
            MAX_DRIVER_ITERATIONS
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;
    use crabdance_core::{WorkflowExecution, WorkflowType};

    use super::*;

    fn test_workflow_info() -> WorkflowInfo {
        WorkflowInfo {
            workflow_execution: WorkflowExecution {
                workflow_id: "wf-1".to_string(),
                run_id: "run-1".to_string(),
            },
            workflow_type: WorkflowType {
                name: "TestWorkflow".to_string(),
            },
            task_list: "test-tl".to_string(),
            start_time: Utc::now(),
            execution_start_to_close_timeout: Duration::from_secs(3600),
            task_start_to_close_timeout: Duration::from_secs(10),
            attempt: 1,
            continued_execution_run_id: None,
            parent_workflow_execution: None,
            cron_schedule: None,
            memo: None,
            search_attributes: None,
        }
    }

    /// A resolver that returns the activity type's name bytes as the result.
    fn echo_resolver() -> Arc<dyn CommandResolver> {
        Arc::new(|command: &WorkflowCommand| match command {
            WorkflowCommand::ScheduleActivity(c) => {
                Resolution::Done(Ok(c.activity_type.clone().into_bytes()))
            }
            _ => Resolution::Blocked,
        })
    }

    #[test]
    fn runs_workflow_to_completion() {
        let driver = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = driver.context();

        let outcome = driver.run(async move {
            let result = ctx
                .execute_activity("greet", None, Default::default())
                .await?;
            Ok(result)
        });

        let result = outcome.into_completed().expect("should complete").unwrap();
        assert_eq!(result, b"greet".to_vec());
    }

    #[test]
    fn auto_fires_timer_on_mock_clock() {
        let info = test_workflow_info();
        let start_nanos = info.start_time.timestamp_nanos_opt().unwrap();
        let driver = WorkflowDriver::new(info, echo_resolver());
        let ctx = driver.context();

        let outcome = driver.run(async move {
            ctx.sleep(Duration::from_secs(60)).await;
            // After the timer fires, deterministic time has advanced by the duration.
            let now = ctx.now().timestamp_nanos_opt().unwrap();
            Ok(now.to_string().into_bytes())
        });

        let result = outcome.into_completed().expect("should complete").unwrap();
        let observed: i64 = String::from_utf8(result).unwrap().parse().unwrap();
        assert_eq!(
            observed,
            start_nanos + Duration::from_secs(60).as_nanos() as i64
        );
    }

    #[test]
    fn fires_multiple_timers_in_deadline_order() {
        let driver = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = driver.context();

        let outcome = driver.run(async move {
            // Two concurrent timers; the shorter one should resolve first but both fire.
            let short = ctx.new_timer(Duration::from_secs(5));
            let long = ctx.new_timer(Duration::from_secs(10));
            futures::future::join(short, long).await;
            Ok(Vec::new())
        });

        assert!(matches!(outcome, DriverOutcome::Completed(Ok(_))));
        let timers: Vec<_> = driver
            .recorded_commands()
            .into_iter()
            .filter(|c| matches!(c, CommandRecord::StartTimer { .. }))
            .collect();
        assert_eq!(timers.len(), 2);
    }

    #[test]
    fn reports_blocked_when_waiting_on_signal() {
        let driver = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = driver.context();

        let outcome = driver.run(async move {
            let mut channel = ctx.get_signal_channel("never-arrives");
            let _ = channel.recv().await;
            Ok(Vec::new())
        });

        assert!(outcome.is_blocked());
    }

    #[test]
    fn get_version_records_marker_synchronously_in_order() {
        // Regression test for the get_version determinism leak: the version marker must
        // be recorded synchronously and in order (before the subsequent activity),
        // not via a detached tokio::spawn that escapes the dispatcher.
        let driver = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = driver.context();

        let _ = driver.run(async move {
            let _v = ctx.get_version("change-1", crate::context::DEFAULT_VERSION, 1);
            ctx.execute_activity("after", None, Default::default())
                .await?;
            Ok(Vec::new())
        });

        let commands = driver.recorded_commands();
        // The version marker is recorded first and synchronously, followed by the
        // CadenceChangeVersion search-attribute upsert, then the activity — all before
        // the next await point, proving in-order synchronous recording.
        assert_eq!(
            commands[0],
            CommandRecord::RecordMarker {
                marker_name: "Version".to_string()
            },
            "version marker must be recorded first, in order"
        );
        assert!(matches!(
            commands[1],
            CommandRecord::UpsertSearchAttributes { .. }
        ));
        assert!(matches!(
            commands[2],
            CommandRecord::ScheduleActivity { .. }
        ));
    }

    #[test]
    fn upsert_search_attributes_emits_command_and_updates_set() {
        let driver = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = driver.context();
        let read_ctx = ctx.clone();

        let outcome = driver.run(async move {
            ctx.upsert_search_attributes(vec![(
                "CustomKeyword".to_string(),
                b"\"orders\"".to_vec(),
            )]);
            Ok(Vec::new())
        });

        assert!(matches!(outcome, DriverOutcome::Completed(Ok(_))));
        // The decision was emitted through the deterministic pipeline...
        let commands = driver.recorded_commands();
        assert_eq!(
            commands,
            vec![CommandRecord::UpsertSearchAttributes {
                keys: vec!["CustomKeyword".to_string()]
            }]
        );
        // ...and the attribute is visible to the workflow.
        let attrs = read_ctx.get_search_attributes();
        assert_eq!(
            attrs.get("CustomKeyword").map(|v| v.as_slice()),
            Some(b"\"orders\"".as_slice())
        );
    }

    #[test]
    fn workflow_accessors_reflect_state() {
        let driver = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = driver.context();

        // is_replaying
        assert!(!ctx.is_replaying());
        ctx.set_replay_mode(true);
        assert!(ctx.is_replaying());

        // history size
        ctx.set_history_size(5, 1024);
        assert_eq!(ctx.get_history_count(), 5);
        assert_eq!(ctx.get_total_history_bytes(), 1024);

        // cron last-completion result
        assert!(!ctx.has_last_completion_result());
        ctx.set_last_completion_result(Some(b"prev".to_vec()));
        assert!(ctx.has_last_completion_result());
        assert_eq!(ctx.get_last_completion_result(), Some(b"prev".to_vec()));

        // NewValue
        let value = ctx.new_value(serde_json::to_vec(&42i32).unwrap());
        assert_eq!(value.decode::<i32>().unwrap(), 42);
    }

    #[test]
    fn replay_aware_logging_guard() {
        let driver = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = driver.context();

        // Live execution: always log.
        assert!(ctx.should_log());

        // Replay: suppressed by default...
        ctx.set_replay_mode(true);
        assert!(!ctx.should_log());

        // ...unless logging-in-replay is enabled (Go's EnableLoggingInReplay).
        ctx.set_logging_enabled_in_replay(true);
        assert!(ctx.should_log());

        // These must not panic regardless of guard state.
        ctx.log_info("hello");
        ctx.log_error("oops");
    }

    #[test]
    fn propagation_context_round_trips_through_propagator() {
        use crabdance_core::{
            ContextPropagator, PropagationContext, W3CTraceContextPropagator, TRACEPARENT_HEADER,
        };

        let driver = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = driver.context();

        // The worker would set this after extracting the start header.
        let mut incoming = PropagationContext::new();
        incoming.set(TRACEPARENT_HEADER, b"00-abc-def-01".to_vec());
        ctx.set_propagation_context(incoming);

        // Workflow code can read the propagated trace context...
        assert_eq!(
            ctx.propagation_context().get(TRACEPARENT_HEADER),
            Some(&b"00-abc-def-01"[..])
        );

        // ...and inject it into an outbound activity/child carrier.
        let propagators: Vec<Arc<dyn ContextPropagator>> =
            vec![Arc::new(W3CTraceContextPropagator)];
        let carrier = ctx.inject_propagation(&propagators);
        assert_eq!(
            carrier.get(TRACEPARENT_HEADER).map(|v| v.as_slice()),
            Some(&b"00-abc-def-01"[..])
        );
    }

    #[test]
    fn activity_result_pending_sentinel() {
        use crate::future::{ActivityError, DefaultActivityError};

        let pending: DefaultActivityError = ActivityError::result_pending();
        assert!(pending.is_result_pending());

        let failed = ActivityError::retryable("boom");
        assert!(!failed.is_result_pending());
    }

    #[test]
    fn workflow_error_predicates() {
        use crate::future::WorkflowError;

        let generic = WorkflowError::message("boom");
        assert!(generic.is_workflow_error());
        assert!(!generic.is_cancelled());

        let cancelled: DefaultWorkflowError = WorkflowError::Cancelled;
        assert!(cancelled.is_cancelled());
        assert!(!cancelled.is_workflow_error());

        let can: DefaultWorkflowError = WorkflowError::ContinueAsNew;
        assert!(can.is_continue_as_new());
        assert!(!can.is_workflow_error());

        let nd: DefaultWorkflowError = WorkflowError::NonDeterministic("mismatch".to_string());
        assert!(nd.is_non_deterministic());
        assert!(nd.is_workflow_error());
    }

    #[test]
    fn get_version_options_select_first_execution_version() {
        use crate::context::{GetVersionOptions, DEFAULT_VERSION};

        // default → max_supported
        let d = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = d.context();
        d.run(async move {
            assert_eq!(ctx.get_version("a", DEFAULT_VERSION, 5), 5);
            Ok(Vec::new())
        });

        // ExecuteWithVersion → the supplied version
        let d = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = d.context();
        d.run(async move {
            let v = ctx.get_version_with_options(
                "a",
                DEFAULT_VERSION,
                5,
                GetVersionOptions::execute_with_version(3),
            );
            assert_eq!(v, 3);
            Ok(Vec::new())
        });

        // ExecuteWithMinVersion → min_supported
        let d = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = d.context();
        d.run(async move {
            let v = ctx.get_version_with_options(
                "a",
                2,
                5,
                GetVersionOptions::execute_with_min_version(),
            );
            assert_eq!(v, 2);
            Ok(Vec::new())
        });
    }

    #[test]
    fn continue_as_new_is_terminal_not_blocked() {
        // Regression: continue_as_new submits its command then parks the root task
        // forever. The driver must treat that as a terminal ContinuedAsNew outcome, not
        // mistake the parked task for a blocked workflow.
        let driver = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = driver.context();

        let outcome = driver.run(async move {
            ctx.continue_as_new("NextRun", Some(b"carry".to_vec()), Default::default())
                .await
        });

        match outcome {
            DriverOutcome::ContinuedAsNew(c) => {
                assert_eq!(c.workflow_type, "NextRun");
                assert_eq!(c.input.as_deref(), Some(b"carry".as_slice()));
            }
            other => panic!("expected ContinuedAsNew, got {other:?}"),
        }

        // The terminal command is recorded in the emitted sequence.
        assert!(driver
            .recorded_commands()
            .iter()
            .any(|c| matches!(c, CommandRecord::ContinueAsNewWorkflow { .. })));
    }

    #[test]
    fn records_command_sequence() {
        let driver = WorkflowDriver::new(test_workflow_info(), echo_resolver());
        let ctx = driver.context();

        let _ = driver.run(async move {
            ctx.sleep(Duration::from_secs(1)).await;
            ctx.execute_activity("a", None, Default::default()).await?;
            Ok(Vec::new())
        });

        let commands = driver.recorded_commands();
        assert!(matches!(commands[0], CommandRecord::StartTimer { .. }));
        assert!(matches!(
            commands[1],
            CommandRecord::ScheduleActivity { .. }
        ));
    }
}
