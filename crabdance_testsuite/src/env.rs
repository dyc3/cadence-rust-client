//! Unit-testing baseline that drives the **real** [`WorkflowContext`] — the Go-parity
//! core for testing workflows without a Cadence server.
//!
//! Unlike the legacy `TestWorkflowContext` mimic (which reimplemented a parallel
//! context and so could not test the workflows you actually deploy), this environment
//! runs the genuine [`WorkflowContext`]/dispatcher via the in-memory
//! [`WorkflowDriver`](crabdance_workflow::WorkflowDriver). It resolves activities by
//! running real closures or mocked values, auto-fires timers on a mock clock, and
//! supports signals, versions, side effects and child workflows in-test.
//!
//! On top of that baseline it adds the Go-testsuite convenience layer: fluent
//! cardinality (`.once()` / `.times(n)`) with [`assert_expectations`](TestRunResult::assert_expectations),
//! lifecycle listeners, [`register_delayed_callback`](WorkflowTestEnv::register_delayed_callback)
//! that fires at workflow time, cron `last_completion_result`, and per-attempt
//! activity results for retry simulation.
//!
//! ```ignore
//! let mut env = WorkflowTestEnv::new();
//! env.on_activity_fn("double", |args| {
//!     let n: i32 = serde_json::from_slice(args.unwrap()).unwrap();
//!     Ok(serde_json::to_vec(&(n * 2)).unwrap())
//! }).once();
//!
//! let run = env.execute(|ctx| async move {
//!     let out = ctx.execute_activity("double", Some(serde_json::to_vec(&21).unwrap()), Default::default()).await?;
//!     Ok(out)
//! });
//!
//! run.assert_expectations();
//! let n: i32 = serde_json::from_slice(&run.unwrap_output()).unwrap();
//! assert_eq!(n, 42);
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crabdance_core::{WorkflowExecution, WorkflowInfo, WorkflowType};
use crabdance_workflow::commands::WorkflowCommand;
use crabdance_workflow::context::WorkflowContext;
use crabdance_workflow::future::{
    ActivityFailureInfo, ActivityFailureType, DefaultWorkflowError, WorkflowError,
};
use crabdance_workflow::{
    CommandRecord, CommandResolver, DelayedCallback, DriverOutcome, Resolution, WorkflowDriver,
};

type ActivityClosure = Arc<dyn Fn(Option<Vec<u8>>) -> Result<Vec<u8>, String> + Send + Sync>;
/// A lifecycle listener taking the operation's name (activity/child id).
type NameListener = Arc<dyn Fn(&str) + Send + Sync>;
/// A lifecycle listener taking the name and whether the operation succeeded.
type NameSuccessListener = Arc<dyn Fn(&str, bool) + Send + Sync>;
/// A delayed callback: a relative workflow-time delay and a one-shot closure.
type DelayedEntry = (Duration, Box<dyn FnOnce(&WorkflowContext) + Send>);

/// How a registered activity resolves during a test run.
#[derive(Clone)]
enum ActivityMock {
    /// Always return this fixed result (Ok bytes or Err reason).
    Value(Result<Vec<u8>, String>),
    /// Run this closure against the activity's input bytes.
    Func(ActivityClosure),
    /// Return successive results on successive scheduling (retry simulation); the last
    /// entry repeats once exhausted.
    Seq(Vec<Result<Vec<u8>, String>>),
}

/// Lifecycle listeners fired as the workflow schedules and resolves operations.
/// Closures are `Fn` so they may fire many times across a run.
#[derive(Clone, Default)]
struct Listeners {
    activity_scheduled: Vec<NameListener>,
    activity_completed: Vec<NameSuccessListener>,
    child_scheduled: Vec<NameListener>,
    child_completed: Vec<NameSuccessListener>,
    timer_scheduled: Vec<Arc<dyn Fn() + Send + Sync>>,
}

/// A test environment that drives the real `WorkflowContext` to completion in-process.
#[derive(Default)]
pub struct WorkflowTestEnv {
    activity_mocks: HashMap<String, ActivityMock>,
    child_results: HashMap<String, Result<Vec<u8>, String>>,
    signals: Vec<(String, Vec<u8>)>,
    change_versions: HashMap<String, i32>,
    start_time_nanos: i64,
    workflow_id: String,
    run_id: String,
    /// Expected call counts per activity (cardinality), checked by `assert_expectations`.
    expected_activity_calls: HashMap<String, usize>,
    /// The most recently registered activity, so `.once()` / `.times(n)` apply to it.
    last_registered_activity: Option<String>,
    listeners: Listeners,
    last_completion_result: Option<Vec<u8>>,
    /// Callbacks to fire at workflow-time offsets (drained on `execute`).
    delayed_callbacks: Mutex<Vec<DelayedEntry>>,
}

impl WorkflowTestEnv {
    pub fn new() -> Self {
        Self {
            workflow_id: "test-workflow".to_string(),
            run_id: "test-run".to_string(),
            ..Default::default()
        }
    }

    /// Mock an activity to always return `result` (encoded bytes).
    pub fn on_activity(&mut self, activity_type: &str, result: Vec<u8>) -> &mut Self {
        self.register_activity(activity_type, ActivityMock::Value(Ok(result)))
    }

    /// Mock an activity to always fail with `reason`.
    pub fn on_activity_error(&mut self, activity_type: &str, reason: &str) -> &mut Self {
        self.register_activity(activity_type, ActivityMock::Value(Err(reason.to_string())))
    }

    /// Mock an activity by running `func` against its input bytes (real-or-mocked).
    pub fn on_activity_fn<F>(&mut self, activity_type: &str, func: F) -> &mut Self
    where
        F: Fn(Option<Vec<u8>>) -> Result<Vec<u8>, String> + Send + Sync + 'static,
    {
        self.register_activity(activity_type, ActivityMock::Func(Arc::new(func)))
    }

    /// Mock an activity to return successive results on successive invocations — the
    /// building block for retry simulation (e.g. fail once, then succeed). The last
    /// result repeats once the sequence is exhausted.
    pub fn on_activity_attempts(
        &mut self,
        activity_type: &str,
        results: Vec<Result<Vec<u8>, String>>,
    ) -> &mut Self {
        self.register_activity(activity_type, ActivityMock::Seq(results))
    }

    fn register_activity(&mut self, activity_type: &str, mock: ActivityMock) -> &mut Self {
        self.activity_mocks.insert(activity_type.to_string(), mock);
        self.last_registered_activity = Some(activity_type.to_string());
        self
    }

    /// Assert the most recently registered activity is scheduled exactly once
    /// (Go's `.Once()`). Verified by [`TestRunResult::assert_expectations`].
    pub fn once(&mut self) -> &mut Self {
        self.times(1)
    }

    /// Assert the most recently registered activity is scheduled exactly `n` times
    /// (Go's `.Times(n)`). Verified by [`TestRunResult::assert_expectations`].
    pub fn times(&mut self, n: usize) -> &mut Self {
        if let Some(name) = self.last_registered_activity.clone() {
            self.expected_activity_calls.insert(name, n);
        }
        self
    }

    /// Mock a child workflow (by workflow id) to return `result`.
    pub fn on_child_workflow(&mut self, workflow_id: &str, result: Vec<u8>) -> &mut Self {
        self.child_results
            .insert(workflow_id.to_string(), Ok(result));
        self
    }

    /// Mock a child workflow (by workflow id) to fail with `reason`.
    pub fn on_child_workflow_error(&mut self, workflow_id: &str, reason: &str) -> &mut Self {
        self.child_results
            .insert(workflow_id.to_string(), Err(reason.to_string()));
        self
    }

    /// Queue a signal to be delivered to the workflow before it runs.
    pub fn signal(&mut self, signal_name: &str, payload: Vec<u8>) -> &mut Self {
        self.signals.push((signal_name.to_string(), payload));
        self
    }

    /// Pin a `get_version` result for `change_id` (so version-gated branches are
    /// exercised deterministically).
    pub fn set_version(&mut self, change_id: &str, version: i32) -> &mut Self {
        self.change_versions.insert(change_id.to_string(), version);
        self
    }

    /// Set the deterministic workflow start time (unix-epoch nanoseconds).
    pub fn set_start_time_nanos(&mut self, nanos: i64) -> &mut Self {
        self.start_time_nanos = nanos;
        self
    }

    /// Seed the cron previous-run result returned by `ctx.last_completion_result()`
    /// (Go's `SetLastCompletionResult`).
    pub fn set_last_completion_result(&mut self, result: Vec<u8>) -> &mut Self {
        self.last_completion_result = Some(result);
        self
    }

    /// Register a callback to fire after `delay` of workflow (mock-clock) time, e.g. to
    /// deliver a signal mid-run (Go's `RegisterDelayedCallback`). The callback receives
    /// the live [`WorkflowContext`].
    pub fn register_delayed_callback<F>(&mut self, delay: Duration, callback: F) -> &mut Self
    where
        F: FnOnce(&WorkflowContext) + Send + 'static,
    {
        self.delayed_callbacks
            .lock()
            .unwrap()
            .push((delay, Box::new(callback)));
        self
    }

    /// Listen for each activity scheduling (Go's activity-start listener).
    pub fn on_activity_scheduled<F: Fn(&str) + Send + Sync + 'static>(
        &mut self,
        listener: F,
    ) -> &mut Self {
        self.listeners.activity_scheduled.push(Arc::new(listener));
        self
    }

    /// Listen for each activity completion, with whether it succeeded.
    pub fn on_activity_completed<F: Fn(&str, bool) + Send + Sync + 'static>(
        &mut self,
        listener: F,
    ) -> &mut Self {
        self.listeners.activity_completed.push(Arc::new(listener));
        self
    }

    /// Listen for each child-workflow scheduling.
    pub fn on_child_scheduled<F: Fn(&str) + Send + Sync + 'static>(
        &mut self,
        listener: F,
    ) -> &mut Self {
        self.listeners.child_scheduled.push(Arc::new(listener));
        self
    }

    /// Listen for each child-workflow completion, with whether it succeeded.
    pub fn on_child_completed<F: Fn(&str, bool) + Send + Sync + 'static>(
        &mut self,
        listener: F,
    ) -> &mut Self {
        self.listeners.child_completed.push(Arc::new(listener));
        self
    }

    /// Listen for each timer scheduling.
    pub fn on_timer_scheduled<F: Fn() + Send + Sync + 'static>(
        &mut self,
        listener: F,
    ) -> &mut Self {
        self.listeners.timer_scheduled.push(Arc::new(listener));
        self
    }

    fn workflow_info(&self) -> WorkflowInfo {
        let start_time = chrono::DateTime::from_timestamp(
            self.start_time_nanos.div_euclid(1_000_000_000),
            self.start_time_nanos.rem_euclid(1_000_000_000) as u32,
        )
        .unwrap_or_default();

        WorkflowInfo {
            workflow_execution: WorkflowExecution {
                workflow_id: self.workflow_id.clone(),
                run_id: self.run_id.clone(),
            },
            workflow_type: WorkflowType {
                name: "TestWorkflow".to_string(),
            },
            task_list: "test".to_string(),
            start_time,
            execution_start_to_close_timeout: std::time::Duration::from_secs(3600),
            task_start_to_close_timeout: std::time::Duration::from_secs(10),
            attempt: 1,
            continued_execution_run_id: None,
            parent_workflow_execution: None,
            cron_schedule: None,
            memo: None,
            search_attributes: None,
        }
    }

    /// Run `build`'s workflow (a closure receiving the real [`WorkflowContext`]) to
    /// completion, resolving activities from the registered mocks and auto-firing timers.
    pub fn execute<F, Fut>(&self, build: F) -> TestRunResult
    where
        F: FnOnce(WorkflowContext) -> Fut,
        Fut: Future<Output = Result<Vec<u8>, DefaultWorkflowError>> + Send + 'static,
    {
        let executed = Arc::new(Mutex::new(Vec::new()));
        let task_lists = Arc::new(Mutex::new(HashMap::new()));
        let resolver = Arc::new(MockResolver {
            activity_mocks: self.activity_mocks.clone(),
            child_results: self.child_results.clone(),
            executed_activities: executed.clone(),
            scheduled_task_lists: task_lists.clone(),
            call_indices: Arc::new(Mutex::new(HashMap::new())),
            listeners: self.listeners.clone(),
        });

        let driver = WorkflowDriver::new(self.workflow_info(), resolver);
        let ctx = driver.context();

        for (name, payload) in &self.signals {
            ctx.add_signal(name, payload.clone());
        }
        if !self.change_versions.is_empty() {
            ctx.set_change_versions(self.change_versions.clone());
        }
        if let Some(result) = &self.last_completion_result {
            ctx.set_last_completion_result(Some(result.clone()));
        }

        // Convert delayed callbacks (relative delays) to absolute mock-clock deadlines.
        let callbacks: Vec<DelayedCallback> = self
            .delayed_callbacks
            .lock()
            .unwrap()
            .drain(..)
            .map(|(delay, cb)| (self.start_time_nanos + delay.as_nanos() as i64, cb))
            .collect();

        let outcome = driver.run_with_callbacks(build(ctx), callbacks);

        let executed_activities = executed.lock().unwrap().clone();
        let scheduled_task_lists = task_lists.lock().unwrap().clone();
        let commands = driver.recorded_commands();
        TestRunResult {
            outcome,
            executed_activities,
            scheduled_task_lists,
            commands,
            expected_activity_calls: self.expected_activity_calls.clone(),
        }
    }
}

struct MockResolver {
    activity_mocks: HashMap<String, ActivityMock>,
    child_results: HashMap<String, Result<Vec<u8>, String>>,
    executed_activities: Arc<Mutex<Vec<String>>>,
    /// The task list each activity type was last scheduled on (for session routing).
    scheduled_task_lists: Arc<Mutex<HashMap<String, String>>>,
    /// Per-activity invocation counter, for `Seq` mocks.
    call_indices: Arc<Mutex<HashMap<String, usize>>>,
    listeners: Listeners,
}

/// Build a production-equivalent activity failure (so a workflow that matches on
/// `WorkflowError::ActivityFailed` behaves the same in-test as on a real worker).
fn activity_failure(reason: impl Into<String>) -> DefaultWorkflowError {
    WorkflowError::ActivityFailed(ActivityFailureInfo {
        failure_type: ActivityFailureType::ExecutionFailed,
        message: reason.into(),
        details: None,
        retryable: false,
    })
}

impl MockResolver {
    /// Resolve an activity's raw `Result<bytes, reason>`, advancing the per-activity
    /// call index for sequence mocks.
    fn activity_result(
        &self,
        activity_type: &str,
        args: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, String> {
        match self.activity_mocks.get(activity_type) {
            Some(ActivityMock::Value(v)) => v.clone(),
            Some(ActivityMock::Func(func)) => func(args),
            Some(ActivityMock::Seq(results)) if !results.is_empty() => {
                let mut indices = self.call_indices.lock().unwrap();
                let idx = indices.entry(activity_type.to_string()).or_insert(0);
                let result = results[(*idx).min(results.len() - 1)].clone();
                *idx += 1;
                result
            }
            // No mock registered (or empty sequence) is a harness misconfiguration.
            _ => Err(format!("no mock registered for activity '{activity_type}'")),
        }
    }
}

impl CommandResolver for MockResolver {
    fn resolve(&self, command: &WorkflowCommand) -> Resolution {
        match command {
            WorkflowCommand::ScheduleActivity(c) => {
                self.executed_activities
                    .lock()
                    .unwrap()
                    .push(c.activity_type.clone());
                self.scheduled_task_lists
                    .lock()
                    .unwrap()
                    .insert(c.activity_type.clone(), c.options.task_list.clone());
                for listener in &self.listeners.activity_scheduled {
                    listener(&c.activity_type);
                }

                let result = self.activity_result(&c.activity_type, c.args.clone());
                for listener in &self.listeners.activity_completed {
                    listener(&c.activity_type, result.is_ok());
                }
                match result {
                    Ok(bytes) => Resolution::Done(Ok(bytes)),
                    Err(reason) if self.activity_mocks.contains_key(&c.activity_type) => {
                        Resolution::Done(Err(activity_failure(reason)))
                    }
                    // Distinguish a misconfiguration from a simulated failure.
                    Err(reason) => Resolution::Done(Err(WorkflowError::message(reason))),
                }
            }
            WorkflowCommand::StartChildWorkflow(c) => {
                for listener in &self.listeners.child_scheduled {
                    listener(&c.workflow_id);
                }
                let result = self.child_results.get(&c.workflow_id);
                for listener in &self.listeners.child_completed {
                    listener(&c.workflow_id, matches!(result, Some(Ok(_))));
                }
                match result {
                    Some(Ok(bytes)) => Resolution::Done(Ok(bytes.clone())),
                    Some(Err(reason)) => {
                        Resolution::Done(Err(WorkflowError::child_workflow_failed(reason.clone())))
                    }
                    None => Resolution::Done(Err(WorkflowError::message(format!(
                        "no mock registered for child workflow '{}'",
                        c.workflow_id
                    )))),
                }
            }
            WorkflowCommand::StartTimer(_) => {
                for listener in &self.listeners.timer_scheduled {
                    listener();
                }
                // Timers are resolved by the driver's mock clock, not here.
                Resolution::Blocked
            }
            WorkflowCommand::SignalExternalWorkflow(_)
            | WorkflowCommand::RequestCancelExternalWorkflow(_)
            | WorkflowCommand::ContinueAsNewWorkflow(_) => Resolution::Done(Ok(Vec::new())),
            _ => Resolution::Blocked,
        }
    }
}

/// The outcome of a [`WorkflowTestEnv::execute`] run.
pub struct TestRunResult {
    outcome: DriverOutcome,
    executed_activities: Vec<String>,
    scheduled_task_lists: HashMap<String, String>,
    commands: Vec<CommandRecord>,
    expected_activity_calls: HashMap<String, usize>,
}

impl TestRunResult {
    /// The workflow's result, or `None` if it continued-as-new or blocked on an
    /// undelivered external event.
    pub fn output_result(&self) -> Option<Result<&Vec<u8>, &DefaultWorkflowError>> {
        match &self.outcome {
            DriverOutcome::Completed(Ok(bytes)) => Some(Ok(bytes)),
            DriverOutcome::Completed(Err(e)) => Some(Err(e)),
            DriverOutcome::ContinuedAsNew(_) | DriverOutcome::Blocked => None,
        }
    }

    /// The successful output bytes; panics if the workflow failed, continued-as-new, or blocked.
    pub fn unwrap_output(self) -> Vec<u8> {
        match self.outcome {
            DriverOutcome::Completed(Ok(bytes)) => bytes,
            DriverOutcome::Completed(Err(e)) => panic!("workflow failed: {e}"),
            DriverOutcome::ContinuedAsNew(_) => panic!("workflow continued as new"),
            DriverOutcome::Blocked => panic!("workflow blocked on an external event"),
        }
    }

    /// Whether the workflow completed successfully.
    pub fn is_ok(&self) -> bool {
        matches!(self.outcome, DriverOutcome::Completed(Ok(_)))
    }

    /// Whether the workflow blocked waiting on an external event (e.g. a missing signal).
    pub fn is_blocked(&self) -> bool {
        self.outcome.is_blocked()
    }

    /// The continue-as-new details if the workflow terminated by continuing as new.
    pub fn continued_as_new(&self) -> Option<&crabdance_workflow::ContinuedAsNew> {
        self.outcome.continued_as_new()
    }

    /// The activity types the workflow scheduled, in order.
    pub fn executed_activities(&self) -> &[String] {
        &self.executed_activities
    }

    /// Whether the workflow scheduled an activity of the given type.
    pub fn was_activity_executed(&self, activity_type: &str) -> bool {
        self.executed_activities.iter().any(|a| a == activity_type)
    }

    /// The task list an activity type was scheduled on (e.g. to verify a session
    /// pinned its activities to the resource-specific task list). Empty string means
    /// the workflow's default task list.
    pub fn activity_task_list(&self, activity_type: &str) -> Option<String> {
        self.scheduled_task_lists.get(activity_type).cloned()
    }

    /// How many times the workflow scheduled an activity of the given type.
    pub fn activity_call_count(&self, activity_type: &str) -> usize {
        self.executed_activities
            .iter()
            .filter(|a| a.as_str() == activity_type)
            .count()
    }

    /// The full sequence of commands the workflow emitted.
    pub fn commands(&self) -> &[CommandRecord] {
        &self.commands
    }

    /// Verify every cardinality expectation (`.once()` / `.times(n)`); panics on a
    /// mismatch (Go's `AssertExpectations`).
    pub fn assert_expectations(&self) {
        for (activity, &expected) in &self.expected_activity_calls {
            let actual = self.activity_call_count(activity);
            assert_eq!(
                actual, expected,
                "activity '{activity}' expected {expected} call(s), got {actual}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crabdance_workflow::context::DEFAULT_VERSION;

    use super::*;

    #[test]
    fn runs_real_workflow_with_mocked_activity_by_fn() {
        let mut env = WorkflowTestEnv::new();
        env.on_activity_fn("double", |args| {
            let n: i32 = serde_json::from_slice(&args.unwrap()).unwrap();
            Ok(serde_json::to_vec(&(n * 2)).unwrap())
        });

        let run = env.execute(|ctx| async move {
            let out = ctx
                .execute_activity(
                    "double",
                    Some(serde_json::to_vec(&21).unwrap()),
                    Default::default(),
                )
                .await?;
            Ok(out)
        });

        assert!(run.was_activity_executed("double"));
        let n: i32 = serde_json::from_slice(&run.unwrap_output()).unwrap();
        assert_eq!(n, 42);
    }

    #[test]
    fn mocks_activity_by_value() {
        let mut env = WorkflowTestEnv::new();
        env.on_activity("greet", b"hi".to_vec());

        let run = env.execute(|ctx| async move {
            ctx.execute_activity("greet", None, Default::default())
                .await
        });
        assert_eq!(run.unwrap_output(), b"hi".to_vec());
    }

    #[test]
    fn auto_fires_timers() {
        let mut env = WorkflowTestEnv::new();
        env.set_start_time_nanos(0);

        let run = env.execute(|ctx| async move {
            ctx.sleep(Duration::from_secs(30)).await;
            Ok(ctx.now().timestamp().to_string().into_bytes())
        });

        let secs: i64 = String::from_utf8(run.unwrap_output())
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(secs, 30);
    }

    #[test]
    fn delivers_seeded_signal() {
        let mut env = WorkflowTestEnv::new();
        env.signal("go", b"payload".to_vec());

        let run = env.execute(|ctx| async move {
            let mut channel = ctx.get_signal_channel("go");
            let payload = channel.recv().await.unwrap_or_default();
            Ok(payload)
        });

        assert_eq!(run.unwrap_output(), b"payload".to_vec());
    }

    #[test]
    fn blocks_on_undelivered_signal() {
        let env = WorkflowTestEnv::new();
        let run = env.execute(|ctx| async move {
            let mut channel = ctx.get_signal_channel("never");
            let _ = channel.recv().await;
            Ok(Vec::new())
        });
        assert!(run.is_blocked());
    }

    #[test]
    fn exercises_version_gate_and_side_effects() {
        let mut env = WorkflowTestEnv::new();
        env.set_version("step", 1);
        env.on_activity("new_path", b"v1".to_vec());
        env.on_activity("old_path", b"v0".to_vec());

        let run = env.execute(|ctx| async move {
            // side effect runs for real in execution mode
            let n = ctx.side_effect(|| 7u32).await;
            assert_eq!(n, 7);
            let version = ctx.get_version("step", DEFAULT_VERSION, 1);
            if version >= 1 {
                ctx.execute_activity("new_path", None, Default::default())
                    .await
            } else {
                ctx.execute_activity("old_path", None, Default::default())
                    .await
            }
        });

        assert_eq!(run.unwrap_output(), b"v1".to_vec());
    }

    #[test]
    fn reports_continue_as_new() {
        let mut env = WorkflowTestEnv::new();
        env.on_activity("work", b"ok".to_vec());

        let run = env.execute(|ctx| async move {
            ctx.execute_activity("work", None, Default::default())
                .await?;
            ctx.continue_as_new("again", None, Default::default()).await
        });

        let continued = run.continued_as_new().expect("should continue as new");
        assert_eq!(continued.workflow_type, "again");
        assert!(run.was_activity_executed("work"));
        assert!(!run.is_blocked());
    }

    #[test]
    fn failed_activity_propagates() {
        let mut env = WorkflowTestEnv::new();
        env.on_activity_error("charge", "card declined");

        let run = env.execute(|ctx| async move {
            ctx.execute_activity("charge", None, Default::default())
                .await
        });

        match run.output_result() {
            Some(Err(e)) => assert!(e.to_string().contains("card declined")),
            other => panic!("expected failure, got {other:?}"),
        }
    }

    #[test]
    fn cardinality_once_is_satisfied() {
        let mut env = WorkflowTestEnv::new();
        env.on_activity("ping", b"pong".to_vec()).once();

        let run = env.execute(|ctx| async move {
            ctx.execute_activity("ping", None, Default::default()).await
        });
        run.assert_expectations();
        assert_eq!(run.activity_call_count("ping"), 1);
    }

    #[test]
    #[should_panic(expected = "expected 2 call(s), got 1")]
    fn cardinality_mismatch_panics() {
        let mut env = WorkflowTestEnv::new();
        env.on_activity("ping", b"pong".to_vec()).times(2);

        let run = env.execute(|ctx| async move {
            ctx.execute_activity("ping", None, Default::default()).await
        });
        run.assert_expectations();
    }

    #[test]
    fn lifecycle_listeners_fire() {
        let scheduled = Arc::new(Mutex::new(Vec::new()));
        let completed = Arc::new(Mutex::new(Vec::new()));
        let s = scheduled.clone();
        let c = completed.clone();

        let mut env = WorkflowTestEnv::new();
        env.on_activity("a", b"ok".to_vec());
        env.on_activity_scheduled(move |name| s.lock().unwrap().push(name.to_string()));
        env.on_activity_completed(move |name, ok| c.lock().unwrap().push((name.to_string(), ok)));

        let _ =
            env.execute(
                |ctx| async move { ctx.execute_activity("a", None, Default::default()).await },
            );

        assert_eq!(*scheduled.lock().unwrap(), vec!["a".to_string()]);
        assert_eq!(*completed.lock().unwrap(), vec![("a".to_string(), true)]);
    }

    #[test]
    fn delayed_callback_delivers_signal_at_workflow_time() {
        let mut env = WorkflowTestEnv::new();
        env.set_start_time_nanos(0);
        // At t=10s, deliver the signal the workflow is waiting on.
        env.register_delayed_callback(Duration::from_secs(10), |ctx| {
            ctx.add_signal("go", b"late".to_vec());
        });

        let run = env.execute(|ctx| async move {
            let mut channel = ctx.get_signal_channel("go");
            // Race a long timer against the signal; the delayed callback fires first.
            let payload = channel.recv().await.unwrap_or_default();
            Ok(payload)
        });

        assert_eq!(run.unwrap_output(), b"late".to_vec());
    }

    #[test]
    fn activity_attempts_returns_successive_results() {
        let mut env = WorkflowTestEnv::new();
        env.on_activity_attempts(
            "flaky",
            vec![Err("boom".to_string()), Ok(b"recovered".to_vec())],
        );

        let run = env.execute(|ctx| async move {
            // First attempt fails; the workflow retries and the second succeeds.
            if ctx
                .execute_activity("flaky", None, Default::default())
                .await
                .is_err()
            {
                ctx.execute_activity("flaky", None, Default::default())
                    .await
            } else {
                Ok(Vec::new())
            }
        });

        assert_eq!(run.activity_call_count("flaky"), 2);
        assert_eq!(run.unwrap_output(), b"recovered".to_vec());
    }

    #[test]
    fn session_pins_activities_to_resource_tasklist() {
        let response = serde_json::to_vec(&crabdance_workflow::session::SessionCreationResponse {
            tasklist: "res-7@host-a".to_string(),
            host_name: "host-a".to_string(),
            resource_id: "res-7".to_string(),
        })
        .unwrap();

        let mut env = WorkflowTestEnv::new();
        env.on_activity(crabdance_workflow::SESSION_CREATION_ACTIVITY, response);
        env.on_activity("doWork", b"done".to_vec());
        env.on_activity(crabdance_workflow::SESSION_COMPLETION_ACTIVITY, vec![]);

        let run = env.execute(|ctx| async move {
            let session = ctx.create_session("default-tl", Default::default()).await?;
            assert_eq!(session.tasklist, "res-7@host-a");
            assert!(ctx.get_session_info().is_some());

            let out = ctx
                .execute_activity_in_session(&session, "doWork", None, Default::default())
                .await?;
            ctx.complete_session(&session).await;
            assert!(ctx.get_session_info().is_none());
            Ok(out)
        });

        // The session's activity was pinned to the resource-specific task list.
        assert_eq!(
            run.activity_task_list("doWork").as_deref(),
            Some("res-7@host-a")
        );
        // The creation activity used the derived creation task list.
        assert_eq!(
            run.activity_task_list(crabdance_workflow::SESSION_CREATION_ACTIVITY)
                .as_deref(),
            Some("default-tl__internal_session_creation")
        );
        assert_eq!(run.unwrap_output(), b"done".to_vec());
    }

    #[test]
    fn seeds_last_completion_result() {
        let mut env = WorkflowTestEnv::new();
        env.set_last_completion_result(b"prev-run".to_vec());

        let run = env
            .execute(|ctx| async move { Ok(ctx.get_last_completion_result().unwrap_or_default()) });

        assert_eq!(run.unwrap_output(), b"prev-run".to_vec());
    }
}
