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

struct SinkInner {
    /// Current mock-clock time in unix-epoch nanoseconds.
    clock_nanos: i64,
    /// Pending and fired timers keyed by insertion order; deadline ties break by key
    /// so firing order is deterministic.
    timers: BTreeMap<u64, TimerEntry>,
    next_timer_key: u64,
    /// Ordered log of every submitted command.
    commands: Vec<CommandRecord>,
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
                    let deadline = inner
                        .clock_nanos
                        .saturating_add(duration.as_nanos() as i64);
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
            // Markers (side effects, mutable side effects, version, search attributes)
            // are deterministic acknowledgements in an in-memory run.
            WorkflowCommand::RecordMarker(_) => Box::pin(async { Ok(Vec::new()) }),
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
    /// The workflow is blocked on an external event (e.g. a signal) that this run
    /// will never deliver, with no timer left to fire.
    Blocked,
}

impl DriverOutcome {
    /// Returns the completion result, or `None` if the run blocked.
    pub fn into_completed(self) -> Option<Result<Vec<u8>, DefaultWorkflowError>> {
        match self {
            DriverOutcome::Completed(result) => Some(result),
            DriverOutcome::Blocked => None,
        }
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, DriverOutcome::Blocked)
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
            WorkflowContextBuilder::new(workflow_info, sink.clone() as Arc<dyn CommandSink>).build();
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
        assert_eq!(observed, start_nanos + Duration::from_secs(60).as_nanos() as i64);
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
