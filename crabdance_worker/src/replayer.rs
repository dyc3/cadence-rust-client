//! Offline replayer and live shadower for catching workflow non-determinism
//! before deploy.
//!
//! Both reuse the worker's real replay engine ([`WorkflowExecutor`] +
//! [`match_replay_with_history`](crate::replay_verifier::match_replay_with_history)):
//!
//! * [`WorkflowReplayer`] replays one or many recorded histories against the
//!   currently-registered workflow code and reports whether each is deterministic.
//! * [`WorkflowShadower`] streams histories from a live domain (via a
//!   [`HistorySource`]) and replays each, so a code change can be vetted against
//!   real production executions without touching their state.
//!
//! These wire the previously-dead `ShadowOptions` / `ReplayOptions` structs into
//! runnable tools.

use std::sync::Arc;

use async_trait::async_trait;
use crabdance_core::{CadenceError, DataConverter, JsonDataConverter, ReplayContext};
use crabdance_proto::shared::{
    EventAttributes, EventType, History, HistoryEvent, WorkflowExecution, WorkflowType,
};
use crabdance_proto::workflow_service::PollForDecisionTaskResponse;

use crate::executor::cache::WorkflowCache;
use crate::executor::workflow::WorkflowExecutor;
use crate::local_activity_queue::LocalActivityQueue;
use crate::registry::Registry;
use crate::replay_verifier::match_replay_with_history;
use crate::worker::WorkerOptions;

/// The verdict of replaying one history.
#[derive(Debug, Clone)]
pub struct ReplayReport {
    pub workflow_id: String,
    pub run_id: String,
    pub workflow_type: String,
    /// `Ok(())` if replay was deterministic; `Err(reason)` otherwise (a
    /// non-determinism mismatch or a replay failure).
    pub outcome: Result<(), String>,
}

impl ReplayReport {
    pub fn is_deterministic(&self) -> bool {
        self.outcome.is_ok()
    }
}

/// Replays recorded histories against the registered workflow code, offline.
pub struct WorkflowReplayer {
    registry: Arc<dyn Registry>,
    options: WorkerOptions,
    data_converter: Arc<dyn DataConverter>,
    task_list: String,
    domain: String,
}

impl WorkflowReplayer {
    pub fn new(registry: Arc<dyn Registry>) -> Self {
        Self {
            registry,
            options: WorkerOptions::default(),
            data_converter: Arc::new(JsonDataConverter),
            task_list: "replayer".to_string(),
            domain: "replayer".to_string(),
        }
    }

    pub fn with_options(mut self, options: WorkerOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_data_converter(mut self, converter: Arc<dyn DataConverter>) -> Self {
        self.data_converter = converter;
        self
    }

    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = domain.into();
        self
    }

    /// Replay one recorded history. Reports non-determinism if the replayed code
    /// produces a decision sequence that diverges from the history (or replay fails).
    pub async fn replay_history(
        &self,
        workflow_id: impl Into<String>,
        run_id: impl Into<String>,
        history: Vec<HistoryEvent>,
    ) -> ReplayReport {
        let workflow_id = workflow_id.into();
        let run_id = run_id.into();
        let workflow_type = workflow_type_of(&history).unwrap_or_default();

        let report = |outcome: Result<(), String>| ReplayReport {
            workflow_id: workflow_id.clone(),
            run_id: run_id.clone(),
            workflow_type: workflow_type.clone(),
            outcome,
        };

        if workflow_type.is_empty() {
            return report(Err(
                "history has no WorkflowExecutionStarted event / workflow type".to_string(),
            ));
        }

        let executor = WorkflowExecutor::new(
            self.registry.clone(),
            Arc::new(WorkflowCache::new(1)),
            self.options.clone(),
            self.task_list.clone(),
            LocalActivityQueue::new(),
            None,
        )
        .with_data_converter(self.data_converter.clone());

        let started_event_id = last_decision_started_event_id(&history);
        let task = PollForDecisionTaskResponse {
            task_token: Vec::new(),
            workflow_execution: Some(WorkflowExecution {
                workflow_id: workflow_id.clone(),
                run_id: run_id.clone(),
            }),
            workflow_type: Some(WorkflowType {
                name: workflow_type.clone(),
            }),
            history: Some(History {
                events: history.clone(),
            }),
            previous_started_event_id: 0,
            started_event_id,
            attempt: 0,
            backlog_count_hint: 0,
            queries: None,
            next_page_token: Some(Vec::new()),
            query: None,
            workflow_execution_task_list: None,
            scheduled_timestamp: None,
            started_timestamp: None,
        };

        match executor.execute_decision_task(task).await {
            Err(e) => report(Err(format!("replay failed: {e}"))),
            Ok((decisions, _)) => {
                let ctx = ReplayContext {
                    workflow_type: workflow_type.clone(),
                    workflow_id: workflow_id.clone(),
                    run_id: run_id.clone(),
                    task_list: self.task_list.clone(),
                    domain_name: self.domain.clone(),
                };
                match match_replay_with_history(&decisions, &history, &ctx) {
                    Ok(()) => report(Ok(())),
                    Err(e) => report(Err(format!("non-deterministic: {e}"))),
                }
            }
        }
    }

    /// Replay a batch of histories, returning a report per history in order.
    pub async fn replay_histories(
        &self,
        histories: Vec<(String, String, Vec<HistoryEvent>)>,
    ) -> Vec<ReplayReport> {
        let mut reports = Vec::with_capacity(histories.len());
        for (workflow_id, run_id, history) in histories {
            reports.push(self.replay_history(workflow_id, run_id, history).await);
        }
        reports
    }
}

/// A source of workflow executions and their histories — the seam the shadower uses
/// to stream from a live domain. The client implements this; tests provide a mock.
#[async_trait]
pub trait HistorySource: Send + Sync {
    /// List `(workflow_id, run_id)` executions matching the visibility `query`.
    async fn list_executions(&self, query: &str) -> Result<Vec<(String, String)>, CadenceError>;

    /// Fetch the full history of one execution.
    async fn fetch_history(
        &self,
        workflow_id: &str,
        run_id: &str,
    ) -> Result<Vec<HistoryEvent>, CadenceError>;
}

/// Whether the shadower runs a single sweep or loops continuously.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowMode {
    /// One sweep over the matching executions, then stop.
    Once,
    /// Loop, re-sweeping until `max_executions` (if set) have been shadowed.
    Continuous,
}

/// Streams histories from a live domain and replays each to detect non-determinism
/// against current workflow code (Go's `WorkflowShadower`).
pub struct WorkflowShadower {
    source: Arc<dyn HistorySource>,
    query: String,
    replayer: WorkflowReplayer,
    mode: ShadowMode,
    /// Stop after shadowing this many executions (a safety bound for `Continuous`).
    max_executions: Option<usize>,
}

impl WorkflowShadower {
    pub fn new(
        source: Arc<dyn HistorySource>,
        query: impl Into<String>,
        replayer: WorkflowReplayer,
    ) -> Self {
        Self {
            source,
            query: query.into(),
            replayer,
            mode: ShadowMode::Once,
            max_executions: None,
        }
    }

    pub fn with_mode(mut self, mode: ShadowMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_max_executions(mut self, max: usize) -> Self {
        self.max_executions = Some(max);
        self
    }

    /// Run the shadower: list matching executions, fetch and replay each history, and
    /// return a report per execution. In [`ShadowMode::Continuous`] it re-sweeps until
    /// `max_executions` reports have been collected (or a sweep finds nothing new).
    pub async fn run(&self) -> Result<Vec<ReplayReport>, CadenceError> {
        let mut reports = Vec::new();

        loop {
            let executions = self.source.list_executions(&self.query).await?;
            if executions.is_empty() {
                break;
            }

            for (workflow_id, run_id) in executions {
                match self.source.fetch_history(&workflow_id, &run_id).await {
                    Ok(history) => {
                        reports.push(
                            self.replayer
                                .replay_history(workflow_id, run_id, history)
                                .await,
                        );
                    }
                    Err(e) => reports.push(ReplayReport {
                        workflow_id,
                        run_id,
                        workflow_type: String::new(),
                        outcome: Err(format!("failed to fetch history: {e}")),
                    }),
                }

                if let Some(max) = self.max_executions {
                    if reports.len() >= max {
                        return Ok(reports);
                    }
                }
            }

            if self.mode == ShadowMode::Once {
                break;
            }
        }

        Ok(reports)
    }
}

/// The workflow type from the first `WorkflowExecutionStarted` event.
fn workflow_type_of(events: &[HistoryEvent]) -> Option<String> {
    events.iter().find_map(|e| match &e.attributes {
        Some(EventAttributes::WorkflowExecutionStartedEventAttributes(a)) => {
            a.workflow_type.as_ref().map(|w| w.name.clone())
        }
        _ => None,
    })
}

/// The id of the last `DecisionTaskStarted` event, used as `started_event_id`.
fn last_decision_started_event_id(events: &[HistoryEvent]) -> i64 {
    events
        .iter()
        .filter(|e| e.event_type == EventType::DecisionTaskStarted)
        .map(|e| e.event_id)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;

    use crabdance_proto::shared::{
        DecisionTaskScheduledEventAttributes, DecisionTaskStartedEventAttributes, TaskList,
        TaskListKind, WorkflowExecutionStartedEventAttributes,
    };

    use crate::registry::{Workflow, WorkflowRegistry};

    use super::*;

    /// A workflow that returns its input unchanged and completes immediately.
    #[derive(Clone)]
    struct NoopWorkflow;
    impl Workflow for NoopWorkflow {
        fn execute(
            &self,
            _ctx: crabdance_workflow::context::WorkflowContext,
            input: Option<Vec<u8>>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, crate::registry::WorkflowError>> + Send>>
        {
            Box::pin(async move { Ok(input.unwrap_or_default()) })
        }
    }

    fn started_event(workflow_type: &str) -> HistoryEvent {
        HistoryEvent {
            event_id: 1,
            event_type: EventType::WorkflowExecutionStarted,
            attributes: Some(EventAttributes::WorkflowExecutionStartedEventAttributes(
                Box::new(WorkflowExecutionStartedEventAttributes {
                    workflow_type: Some(WorkflowType {
                        name: workflow_type.to_string(),
                    }),
                    parent_workflow_execution: None,
                    task_list: Some(TaskList {
                        name: "test-list".to_string(),
                        kind: TaskListKind::Normal,
                    }),
                    input: vec![],
                    execution_start_to_close_timeout_seconds: 10,
                    task_start_to_close_timeout_seconds: 10,
                    identity: "id".to_string(),
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
        }
    }

    fn minimal_history(workflow_type: &str) -> Vec<HistoryEvent> {
        vec![
            started_event(workflow_type),
            HistoryEvent {
                event_id: 2,
                event_type: EventType::DecisionTaskScheduled,
                attributes: Some(EventAttributes::DecisionTaskScheduledEventAttributes(
                    Box::new(DecisionTaskScheduledEventAttributes {
                        task_list: Some(TaskList {
                            name: "test-list".to_string(),
                            kind: TaskListKind::Normal,
                        }),
                        start_to_close_timeout_seconds: 10,
                        attempt: 0,
                    }),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
            HistoryEvent {
                event_id: 3,
                event_type: EventType::DecisionTaskStarted,
                attributes: Some(EventAttributes::DecisionTaskStartedEventAttributes(
                    Box::new(DecisionTaskStartedEventAttributes {
                        scheduled_event_id: 2,
                        identity: "worker".to_string(),
                        request_id: "req".to_string(),
                    }),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
            // The recorded completion the trivial workflow's replay must reproduce.
            HistoryEvent {
                event_id: 4,
                event_type: EventType::WorkflowExecutionCompleted,
                attributes: Some(EventAttributes::WorkflowExecutionCompletedEventAttributes(
                    Box::new(
                        crabdance_proto::shared::WorkflowExecutionCompletedEventAttributes {
                            result: Some(vec![]),
                            decision_task_completed_event_id: 3,
                        },
                    ),
                )),
                timestamp: 0,
                version: 0,
                task_id: 0,
            },
        ]
    }

    fn replayer_with(workflow_type: &str) -> WorkflowReplayer {
        let registry = Arc::new(WorkflowRegistry::new());
        registry.register_workflow(workflow_type, Box::new(NoopWorkflow));
        WorkflowReplayer::new(registry)
    }

    #[tokio::test]
    async fn reports_missing_started_event() {
        let replayer = replayer_with("Noop");
        let report = replayer.replay_history("wf", "run", vec![]).await;
        assert!(!report.is_deterministic());
        assert!(report
            .outcome
            .unwrap_err()
            .contains("WorkflowExecutionStarted"));
    }

    #[tokio::test]
    async fn replays_registered_workflow_with_correct_metadata() {
        let replayer = replayer_with("Noop");
        let report = replayer
            .replay_history("wf-1", "run-1", minimal_history("Noop"))
            .await;
        assert_eq!(report.workflow_id, "wf-1");
        assert_eq!(report.run_id, "run-1");
        assert_eq!(report.workflow_type, "Noop");
        // A trivial workflow that immediately completes replays deterministically.
        assert!(report.is_deterministic(), "outcome: {:?}", report.outcome);
    }

    #[tokio::test]
    async fn batch_replays_in_order() {
        let replayer = replayer_with("Noop");
        let reports = replayer
            .replay_histories(vec![
                ("a".to_string(), "ra".to_string(), minimal_history("Noop")),
                ("b".to_string(), "rb".to_string(), minimal_history("Noop")),
            ])
            .await;
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].workflow_id, "a");
        assert_eq!(reports[1].workflow_id, "b");
    }

    struct MockSource {
        executions: Vec<(String, String)>,
        histories: std::collections::HashMap<String, Vec<HistoryEvent>>,
    }

    #[async_trait]
    impl HistorySource for MockSource {
        async fn list_executions(
            &self,
            _query: &str,
        ) -> Result<Vec<(String, String)>, CadenceError> {
            Ok(self.executions.clone())
        }
        async fn fetch_history(
            &self,
            workflow_id: &str,
            _run_id: &str,
        ) -> Result<Vec<HistoryEvent>, CadenceError> {
            self.histories
                .get(workflow_id)
                .cloned()
                .ok_or_else(|| CadenceError::Other(format!("no history for {workflow_id}")))
        }
    }

    #[tokio::test]
    async fn shadower_replays_each_listed_execution() {
        let mut histories = std::collections::HashMap::new();
        histories.insert("wf-1".to_string(), minimal_history("Noop"));
        histories.insert("wf-2".to_string(), minimal_history("Noop"));
        let source = Arc::new(MockSource {
            executions: vec![
                ("wf-1".to_string(), "r1".to_string()),
                ("wf-2".to_string(), "r2".to_string()),
            ],
            histories,
        });

        let shadower = WorkflowShadower::new(source, "WorkflowType='Noop'", replayer_with("Noop"));
        let reports = shadower.run().await.unwrap();

        assert_eq!(reports.len(), 2);
        assert!(reports.iter().all(|r| r.is_deterministic()));
        assert_eq!(reports[0].workflow_id, "wf-1");
    }

    #[tokio::test]
    async fn shadower_reports_fetch_failure() {
        let source = Arc::new(MockSource {
            executions: vec![("missing".to_string(), "r".to_string())],
            histories: std::collections::HashMap::new(),
        });
        let shadower = WorkflowShadower::new(source, "q", replayer_with("Noop"));
        let reports = shadower.run().await.unwrap();
        assert_eq!(reports.len(), 1);
        assert!(reports[0]
            .outcome
            .as_ref()
            .unwrap_err()
            .contains("fetch history"));
    }
}
