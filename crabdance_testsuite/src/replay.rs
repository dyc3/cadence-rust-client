//! Replay harness: re-run a workflow against a recorded history and assert it
//! reproduces the same sequence of commands (decisions).
//!
//! This is the workflow-code determinism gate. It builds on the in-memory
//! [`WorkflowDriver`](crabdance_workflow::WorkflowDriver): a [`ReplayResolver`]
//! resolves activity/child-workflow commands from recorded results, deterministic
//! markers (side effects, versions) are seeded into the context's replay caches, and
//! timers auto-fire on the mock clock. Running the workflow in replay mode must emit
//! the exact recorded sequence of *schedulable* commands — any divergence is
//! non-determinism.
//!
//! In replay mode the workflow's deterministic side-channels (side effects, mutable
//! side effects, versions, local activities) are served from the seeded caches and do
//! **not** re-emit marker commands, so the asserted command sequence contains only the
//! schedulable commands: activities, timers, child workflows, and external
//! signal/cancel requests. This mirrors how the production worker reconciles replay.
//!
//! The same `CommandSink` seam powers the production replay path (the worker's
//! `ReplayCommandSink`, which resolves from real Cadence history) and the unit-test
//! suite (which resolves by running or mocking activities). This harness is the
//! lightweight, in-process form used to gate determinism in CI.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use crabdance_core::{WorkflowExecution, WorkflowInfo, WorkflowType};
use crabdance_workflow::commands::WorkflowCommand;
use crabdance_workflow::context::WorkflowContext;
use crabdance_workflow::future::{DefaultWorkflowError, WorkflowError};
use crabdance_workflow::{CommandRecord, CommandResolver, DriverOutcome, Resolution, WorkflowDriver};
use serde::{Deserialize, Serialize};

/// A recorded workflow history sufficient to replay a workflow and assert its
/// determinism. Serializable so golden fixtures can be checked into the repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedHistory {
    /// Deterministic workflow start time (unix-epoch nanoseconds) the original run saw.
    #[serde(default)]
    pub start_time_nanos: i64,
    /// Results the workflow observed during the original run, seeded so the replay can
    /// make the same progress.
    pub events: Vec<RecordedEvent>,
    /// The exact sequence of schedulable commands the workflow must emit on replay.
    pub expected_commands: Vec<CommandRecord>,
}

/// A single recorded outcome the workflow observed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordedEvent {
    /// An activity (by deterministic id) completed with this result.
    ActivityCompleted { activity_id: String, result: Vec<u8> },
    /// An activity failed with this reason.
    ActivityFailed { activity_id: String, reason: String },
    /// A child workflow (by id) completed with this result.
    ChildWorkflowCompleted { workflow_id: String, result: Vec<u8> },
    /// A child workflow failed with this reason.
    ChildWorkflowFailed { workflow_id: String, reason: String },
    /// A recorded side-effect result (replayed from cache).
    SideEffect { side_effect_id: u64, result: Vec<u8> },
    /// A recorded version marker (replayed from cache).
    Version { change_id: String, version: i32 },
}

/// An error from replaying a workflow against a recorded history.
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    /// The workflow emitted a different command than history recorded — non-determinism.
    #[error(
        "non-deterministic workflow at command {index}: expected {expected:?}, got {actual:?}"
    )]
    NonDeterministic {
        index: usize,
        expected: Option<CommandRecord>,
        actual: Option<CommandRecord>,
    },
    /// The workflow blocked during replay: a recorded result was missing or the
    /// workflow diverged onto a path with no recorded outcome.
    #[error("workflow blocked during replay (missing recorded result or divergent path)")]
    BlockedDuringReplay,
}

impl ReplayError {
    /// Convert to a workflow [`WorkflowError::NonDeterministic`] for surfacing through
    /// the normal workflow-error channel (used by the worker's replay path / CI gate).
    pub fn into_workflow_error(self) -> DefaultWorkflowError {
        WorkflowError::NonDeterministic(self.to_string())
    }
}

/// Resolves activity and child-workflow commands from recorded results.
struct ReplayResolver {
    activity_results: HashMap<String, Result<Vec<u8>, String>>,
    child_results: HashMap<String, Result<Vec<u8>, String>>,
}

impl CommandResolver for ReplayResolver {
    fn resolve(&self, command: &WorkflowCommand) -> Resolution {
        match command {
            WorkflowCommand::ScheduleActivity(c) => match self.activity_results.get(&c.activity_id) {
                Some(Ok(bytes)) => Resolution::Done(Ok(bytes.clone())),
                Some(Err(reason)) => Resolution::Done(Err(WorkflowError::message(reason.clone()))),
                None => Resolution::Blocked,
            },
            WorkflowCommand::StartChildWorkflow(c) => {
                match self.child_results.get(&c.workflow_id) {
                    Some(Ok(bytes)) => Resolution::Done(Ok(bytes.clone())),
                    Some(Err(reason)) => {
                        Resolution::Done(Err(WorkflowError::message(reason.clone())))
                    }
                    None => Resolution::Blocked,
                }
            }
            // External signal / cancel are fire-and-forget acknowledgements.
            WorkflowCommand::SignalExternalWorkflow(_)
            | WorkflowCommand::RequestCancelExternalWorkflow(_)
            | WorkflowCommand::ContinueAsNewWorkflow(_) => Resolution::Done(Ok(Vec::new())),
            _ => Resolution::Blocked,
        }
    }
}

fn workflow_info(name: &str, start_time_nanos: i64) -> WorkflowInfo {
    let start_time = chrono::DateTime::from_timestamp(
        start_time_nanos.div_euclid(1_000_000_000),
        start_time_nanos.rem_euclid(1_000_000_000) as u32,
    )
    .unwrap_or_default();

    WorkflowInfo {
        workflow_execution: WorkflowExecution {
            workflow_id: format!("replay-{name}"),
            run_id: "replay-run".to_string(),
        },
        workflow_type: WorkflowType {
            name: name.to_string(),
        },
        task_list: "replay".to_string(),
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

/// Replay `build`'s workflow against `history` and assert it reproduces the recorded
/// command sequence deterministically.
///
/// `build` receives a fresh, replay-configured [`WorkflowContext`] and returns the
/// workflow's root future (typically `your_workflow(ctx, args)`).
#[expect(
    clippy::result_large_err,
    reason = "ReplayError carries the diverging commands for diagnostics; this is a \
              cold test-harness path where the ergonomic by-value API is preferable to \
              boxing every command record"
)]
pub fn replay_workflow<F, Fut>(
    history: &RecordedHistory,
    workflow_name: &str,
    build: F,
) -> Result<(), ReplayError>
where
    F: FnOnce(WorkflowContext) -> Fut,
    Fut: Future<Output = Result<Vec<u8>, DefaultWorkflowError>> + Send + 'static,
{
    // Build the resolver and the seedable caches from recorded events.
    let mut activity_results = HashMap::new();
    let mut child_results = HashMap::new();
    let mut side_effect_results: HashMap<u64, Vec<u8>> = HashMap::new();
    let mut change_versions: HashMap<String, i32> = HashMap::new();

    for event in &history.events {
        match event {
            RecordedEvent::ActivityCompleted {
                activity_id,
                result,
            } => {
                activity_results.insert(activity_id.clone(), Ok(result.clone()));
            }
            RecordedEvent::ActivityFailed {
                activity_id,
                reason,
            } => {
                activity_results.insert(activity_id.clone(), Err(reason.clone()));
            }
            RecordedEvent::ChildWorkflowCompleted {
                workflow_id,
                result,
            } => {
                child_results.insert(workflow_id.clone(), Ok(result.clone()));
            }
            RecordedEvent::ChildWorkflowFailed {
                workflow_id,
                reason,
            } => {
                child_results.insert(workflow_id.clone(), Err(reason.clone()));
            }
            RecordedEvent::SideEffect {
                side_effect_id,
                result,
            } => {
                side_effect_results.insert(*side_effect_id, result.clone());
            }
            RecordedEvent::Version { change_id, version } => {
                change_versions.insert(change_id.clone(), *version);
            }
        }
    }

    let resolver = Arc::new(ReplayResolver {
        activity_results,
        child_results,
    });

    let driver = WorkflowDriver::new(workflow_info(workflow_name, history.start_time_nanos), resolver);
    let ctx = driver.context();

    // Configure the context for replay: seed deterministic caches and flip replay mode.
    ctx.set_replay_mode(true);
    ctx.set_current_time_nanos(history.start_time_nanos);
    ctx.set_side_effect_results(side_effect_results);
    ctx.set_change_versions(change_versions);

    let outcome = driver.run(build(ctx));

    // Assert the emitted schedulable-command sequence matches history. A divergence is
    // non-determinism even if the workflow then blocked, so check this first. When the
    // workflow blocked, only compare the prefix it actually emitted: a clean prefix
    // that simply ran out of recorded results is BlockedDuringReplay, not divergence.
    let actual = driver.recorded_commands();
    let expected = &history.expected_commands;
    let blocked = matches!(outcome, DriverOutcome::Blocked);
    let compare_len = if blocked {
        actual.len()
    } else {
        actual.len().max(expected.len())
    };
    for index in 0..compare_len {
        let a = actual.get(index);
        let e = expected.get(index);
        if a != e {
            return Err(ReplayError::NonDeterministic {
                index,
                expected: e.cloned(),
                actual: a.cloned(),
            });
        }
    }

    if blocked {
        return Err(ReplayError::BlockedDuringReplay);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crabdance_workflow::context::DEFAULT_VERSION;

    use super::*;

    /// The golden workflow: version-gate, timer, then an activity. In replay the
    /// version and any side effects come from the seeded caches; the timer and
    /// activity are the schedulable commands asserted against history.
    async fn order_workflow(ctx: WorkflowContext) -> Result<Vec<u8>, DefaultWorkflowError> {
        let _version = ctx.get_version("charge-v1", DEFAULT_VERSION, 1);
        ctx.sleep(Duration::from_secs(60)).await;
        let charged = ctx
            .execute_activity("charge", None, Default::default())
            .await?;
        Ok(charged)
    }

    fn golden_history() -> RecordedHistory {
        RecordedHistory {
            start_time_nanos: 0,
            events: vec![
                RecordedEvent::Version {
                    change_id: "charge-v1".to_string(),
                    version: 1,
                },
                RecordedEvent::ActivityCompleted {
                    activity_id: "1".to_string(),
                    result: b"charged".to_vec(),
                },
            ],
            expected_commands: vec![
                CommandRecord::StartTimer {
                    timer_id: "0".to_string(),
                    duration_millis: 60_000,
                },
                CommandRecord::ScheduleActivity {
                    activity_id: "1".to_string(),
                    activity_type: "charge".to_string(),
                },
            ],
        }
    }

    #[test]
    fn replays_deterministically() {
        let history = golden_history();
        replay_workflow(&history, "order_workflow", order_workflow)
            .expect("workflow should replay deterministically");
    }

    #[test]
    fn golden_fixture_json_round_trips_and_replays() {
        let json = include_str!("../fixtures/replay/order_workflow.json");
        let history: RecordedHistory =
            serde_json::from_str(json).expect("golden fixture should deserialize");
        replay_workflow(&history, "order_workflow", order_workflow)
            .expect("golden fixture should replay deterministically");
    }

    #[test]
    fn detects_nondeterminism() {
        // History expects a timer then an activity, but this workflow skips the timer
        // and schedules a *different* activity — a non-deterministic change.
        async fn diverged(ctx: WorkflowContext) -> Result<Vec<u8>, DefaultWorkflowError> {
            ctx.execute_activity("refund", None, Default::default()).await
        }

        let history = golden_history();
        let err = replay_workflow(&history, "order_workflow", diverged)
            .expect_err("divergent workflow must be detected as non-deterministic");
        match err {
            ReplayError::NonDeterministic {
                index, expected, ..
            } => {
                assert_eq!(index, 0);
                assert!(matches!(
                    expected,
                    Some(CommandRecord::StartTimer { .. })
                ));
            }
            other => panic!("expected NonDeterministic, got {other:?}"),
        }
    }

    #[test]
    fn blocks_when_recorded_result_missing() {
        // History records no result for the activity, so replay cannot make progress.
        let history = RecordedHistory {
            start_time_nanos: 0,
            events: vec![],
            expected_commands: vec![CommandRecord::ScheduleActivity {
                activity_id: "0".to_string(),
                activity_type: "charge".to_string(),
            }],
        };

        async fn just_activity(ctx: WorkflowContext) -> Result<Vec<u8>, DefaultWorkflowError> {
            ctx.execute_activity("charge", None, Default::default()).await
        }

        let err = replay_workflow(&history, "order_workflow", just_activity)
            .expect_err("missing recorded result should block replay");
        assert!(matches!(err, ReplayError::BlockedDuringReplay));
    }
}
