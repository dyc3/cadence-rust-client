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
//! ```ignore
//! let mut env = WorkflowTestEnv::new();
//! env.on_activity_fn("double", |args| {
//!     let n: i32 = serde_json::from_slice(args.unwrap()).unwrap();
//!     Ok(serde_json::to_vec(&(n * 2)).unwrap())
//! });
//!
//! let run = env.execute(|ctx| async move {
//!     let out = ctx.execute_activity("double", Some(serde_json::to_vec(&21).unwrap()), Default::default()).await?;
//!     Ok(out)
//! });
//!
//! assert!(run.was_activity_executed("double"));
//! let n: i32 = serde_json::from_slice(&run.unwrap_output()).unwrap();
//! assert_eq!(n, 42);
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use crabdance_core::{WorkflowExecution, WorkflowInfo, WorkflowType};
use crabdance_workflow::commands::WorkflowCommand;
use crabdance_workflow::context::WorkflowContext;
use crabdance_workflow::future::{DefaultWorkflowError, WorkflowError};
use crabdance_workflow::{CommandRecord, CommandResolver, DriverOutcome, Resolution, WorkflowDriver};

type ActivityClosure = Arc<dyn Fn(Option<Vec<u8>>) -> Result<Vec<u8>, String> + Send + Sync>;

/// How a registered activity resolves during a test run.
#[derive(Clone)]
enum ActivityMock {
    /// Always return this fixed result (Ok bytes or Err reason).
    Value(Result<Vec<u8>, String>),
    /// Run this closure against the activity's input bytes.
    Func(ActivityClosure),
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
        self.activity_mocks
            .insert(activity_type.to_string(), ActivityMock::Value(Ok(result)));
        self
    }

    /// Mock an activity to always fail with `reason`.
    pub fn on_activity_error(&mut self, activity_type: &str, reason: &str) -> &mut Self {
        self.activity_mocks.insert(
            activity_type.to_string(),
            ActivityMock::Value(Err(reason.to_string())),
        );
        self
    }

    /// Mock an activity by running `func` against its input bytes (real-or-mocked).
    pub fn on_activity_fn<F>(&mut self, activity_type: &str, func: F) -> &mut Self
    where
        F: Fn(Option<Vec<u8>>) -> Result<Vec<u8>, String> + Send + Sync + 'static,
    {
        self.activity_mocks.insert(
            activity_type.to_string(),
            ActivityMock::Func(Arc::new(func)),
        );
        self
    }

    /// Mock a child workflow (by workflow id) to return `result`.
    pub fn on_child_workflow(&mut self, workflow_id: &str, result: Vec<u8>) -> &mut Self {
        self.child_results
            .insert(workflow_id.to_string(), Ok(result));
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
        let resolver = Arc::new(MockResolver {
            activity_mocks: self.activity_mocks.clone(),
            child_results: self.child_results.clone(),
            executed_activities: executed.clone(),
        });

        let driver = WorkflowDriver::new(self.workflow_info(), resolver);
        let ctx = driver.context();

        for (name, payload) in &self.signals {
            ctx.add_signal(name, payload.clone());
        }
        if !self.change_versions.is_empty() {
            ctx.set_change_versions(self.change_versions.clone());
        }

        let outcome = driver.run(build(ctx));

        let executed_activities = executed.lock().unwrap().clone();
        let commands = driver.recorded_commands();
        TestRunResult {
            outcome,
            executed_activities,
            commands,
        }
    }
}

struct MockResolver {
    activity_mocks: HashMap<String, ActivityMock>,
    child_results: HashMap<String, Result<Vec<u8>, String>>,
    executed_activities: Arc<Mutex<Vec<String>>>,
}

impl CommandResolver for MockResolver {
    fn resolve(&self, command: &WorkflowCommand) -> Resolution {
        match command {
            WorkflowCommand::ScheduleActivity(c) => {
                self.executed_activities
                    .lock()
                    .unwrap()
                    .push(c.activity_type.clone());
                match self.activity_mocks.get(&c.activity_type) {
                    Some(ActivityMock::Value(Ok(bytes))) => Resolution::Done(Ok(bytes.clone())),
                    Some(ActivityMock::Value(Err(reason))) => {
                        Resolution::Done(Err(WorkflowError::message(reason.clone())))
                    }
                    Some(ActivityMock::Func(func)) => match func(c.args.clone()) {
                        Ok(bytes) => Resolution::Done(Ok(bytes)),
                        Err(reason) => Resolution::Done(Err(WorkflowError::message(reason))),
                    },
                    None => Resolution::Done(Err(WorkflowError::message(format!(
                        "no mock registered for activity '{}'",
                        c.activity_type
                    )))),
                }
            }
            WorkflowCommand::StartChildWorkflow(c) => match self.child_results.get(&c.workflow_id) {
                Some(Ok(bytes)) => Resolution::Done(Ok(bytes.clone())),
                Some(Err(reason)) => Resolution::Done(Err(WorkflowError::message(reason.clone()))),
                None => Resolution::Done(Err(WorkflowError::message(format!(
                    "no mock registered for child workflow '{}'",
                    c.workflow_id
                )))),
            },
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
    commands: Vec<CommandRecord>,
}

impl TestRunResult {
    /// The workflow's result, or `None` if it blocked on an undelivered external event.
    pub fn output_result(&self) -> Option<Result<&Vec<u8>, &DefaultWorkflowError>> {
        match &self.outcome {
            DriverOutcome::Completed(Ok(bytes)) => Some(Ok(bytes)),
            DriverOutcome::Completed(Err(e)) => Some(Err(e)),
            DriverOutcome::Blocked => None,
        }
    }

    /// The successful output bytes; panics if the workflow failed or blocked.
    pub fn unwrap_output(self) -> Vec<u8> {
        match self.outcome {
            DriverOutcome::Completed(Ok(bytes)) => bytes,
            DriverOutcome::Completed(Err(e)) => panic!("workflow failed: {e}"),
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

    /// The activity types the workflow scheduled, in order.
    pub fn executed_activities(&self) -> &[String] {
        &self.executed_activities
    }

    /// Whether the workflow scheduled an activity of the given type.
    pub fn was_activity_executed(&self, activity_type: &str) -> bool {
        self.executed_activities
            .iter()
            .any(|a| a == activity_type)
    }

    /// The full sequence of commands the workflow emitted.
    pub fn commands(&self) -> &[CommandRecord] {
        &self.commands
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
                .execute_activity("double", Some(serde_json::to_vec(&21).unwrap()), Default::default())
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
            ctx.execute_activity("greet", None, Default::default()).await
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

        let secs: i64 = String::from_utf8(run.unwrap_output()).unwrap().parse().unwrap();
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
                ctx.execute_activity("new_path", None, Default::default()).await
            } else {
                ctx.execute_activity("old_path", None, Default::default()).await
            }
        });

        assert_eq!(run.unwrap_output(), b"v1".to_vec());
    }

    #[test]
    fn failed_activity_propagates() {
        let mut env = WorkflowTestEnv::new();
        env.on_activity_error("charge", "card declined");

        let run = env.execute(|ctx| async move {
            ctx.execute_activity("charge", None, Default::default()).await
        });

        match run.output_result() {
            Some(Err(e)) => assert!(e.to_string().contains("card declined")),
            other => panic!("expected failure, got {other:?}"),
        }
    }
}
