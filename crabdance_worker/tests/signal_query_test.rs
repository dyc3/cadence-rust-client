use crabdance_proto::shared::{
    EventAttributes, EventType, History, HistoryEvent, WorkflowExecution,
    WorkflowExecutionSignaledEventAttributes, WorkflowExecutionStartedEventAttributes,
    WorkflowType,
};
use crabdance_proto::workflow_service::PollForDecisionTaskResponse;
use crabdance_proto::WorkflowQuery;
use crabdance_worker::executor::cache::WorkflowCache;
use crabdance_worker::executor::workflow::WorkflowExecutor;
use crabdance_worker::registry::{Registry, Workflow, WorkflowError, WorkflowRegistry};
use crabdance_worker::WorkerOptions;
use crabdance_workflow::context::WorkflowContext;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone)]
struct ClosureWorkflow<F>(F)
where
    F: Fn(
            WorkflowContext,
            Option<Vec<u8>>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>>
        + Send
        + Sync
        + Clone;

impl<F> Workflow for ClosureWorkflow<F>
where
    F: Fn(
            WorkflowContext,
            Option<Vec<u8>>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>>
        + Send
        + Sync
        + Clone
        + 'static,
{
    fn execute(
        &self,
        ctx: WorkflowContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        (self.0)(ctx, input)
    }
}

fn default_event(id: i64, event_type: EventType) -> HistoryEvent {
    HistoryEvent {
        event_id: id,
        timestamp: 0,
        event_type,
        version: 0,
        task_id: 0,
        attributes: None,
    }
}

fn default_started_attrs(name: &str) -> WorkflowExecutionStartedEventAttributes {
    WorkflowExecutionStartedEventAttributes {
        workflow_type: Some(WorkflowType {
            name: name.to_string(),
        }),
        parent_workflow_execution: None,
        task_list: None,
        input: vec![],
        execution_start_to_close_timeout_seconds: 10,
        task_start_to_close_timeout_seconds: 10,
        identity: "test".to_string(),
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
    }
}

fn default_signaled_attrs(name: &str, input: &[u8]) -> WorkflowExecutionSignaledEventAttributes {
    WorkflowExecutionSignaledEventAttributes {
        signal_name: name.to_string(),
        input: Some(input.to_vec()),
        identity: "test".to_string(),
    }
}

#[tokio::test]
async fn test_signal_handling() {
    // 1. Define workflow
    fn signal_workflow(
        ctx: WorkflowContext,
        _: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async move {
            let mut channel = ctx.get_signal_channel("test-signal");
            let val = channel.recv().await.unwrap();
            Ok(val)
        })
    }

    // 2. Setup Registry
    let registry = WorkflowRegistry::new();
    registry.register_workflow("SignalWorkflow", Box::new(ClosureWorkflow(signal_workflow)));
    let registry = Arc::new(registry);

    // 3. Setup Executor
    let cache = Arc::new(WorkflowCache::new(100));
    let executor = WorkflowExecutor::new(
        registry,
        cache,
        WorkerOptions::default(),
        "test-task-list".to_string(),
        crabdance_worker::local_activity_queue::LocalActivityQueue::new(),
    );

    // 4. Create History
    let mut event1 = default_event(1, EventType::WorkflowExecutionStarted);
    event1.attributes = Some(EventAttributes::WorkflowExecutionStartedEventAttributes(
        Box::new(default_started_attrs("SignalWorkflow")),
    ));

    let mut event2 = default_event(2, EventType::WorkflowExecutionSignaled);
    event2.attributes = Some(EventAttributes::WorkflowExecutionSignaledEventAttributes(
        Box::new(default_signaled_attrs("test-signal", b"signal-data")),
    ));

    let event3 = default_event(3, EventType::DecisionTaskScheduled);
    let event4 = default_event(4, EventType::DecisionTaskStarted);

    let events = vec![event1, event2, event3, event4];

    let task = PollForDecisionTaskResponse {
        task_token: b"token".to_vec(),
        workflow_execution: Some(WorkflowExecution {
            workflow_id: "w1".to_string(),
            run_id: "r1".to_string(),
        }),
        workflow_type: Some(WorkflowType {
            name: "SignalWorkflow".to_string(),
        }),
        history: Some(History { events }),
        previous_started_event_id: 0,
        started_event_id: 4,
        attempt: 0,
        backlog_count_hint: 0,
        next_page_token: None,
        query: None,
        workflow_execution_task_list: None,
        scheduled_timestamp: None,
        started_timestamp: None,
        queries: None,
    };

    // 5. Execute
    let (decisions, _) = executor.execute_decision_task(task).await.unwrap();

    // 6. Verify result
    // Should contain CompleteWorkflowExecution decision
    assert!(!decisions.is_empty(), "Expected decisions");
    // We can inspect decisions but checking they exist is a good start.
    // The first decision should be CompleteWorkflowExecution because workflow finished.
    let decision = &decisions[0];
    match decision.decision_type {
        crabdance_proto::shared::DecisionType::CompleteWorkflowExecution => {
            // success
        }
        dt => panic!("Expected CompleteWorkflowExecution, got {:?}", dt),
    }
}

#[tokio::test]
async fn test_query_handling() {
    // 1. Define workflow
    fn query_workflow(
        ctx: WorkflowContext,
        _: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async move {
            ctx.set_query_handler("get_state", |args| {
                // Echo args
                args
            });

            // Wait forever (blocked)
            let mut channel = ctx.get_signal_channel("non-existent");
            channel.recv().await;
            Ok(vec![])
        })
    }

    // 2. Setup Registry
    let registry = WorkflowRegistry::new();
    registry.register_workflow("QueryWorkflow", Box::new(ClosureWorkflow(query_workflow)));
    let registry = Arc::new(registry);

    // 3. Setup Executor
    let cache = Arc::new(WorkflowCache::new(100));
    let executor = WorkflowExecutor::new(
        registry,
        cache,
        WorkerOptions::default(),
        "test-task-list".to_string(),
        crabdance_worker::local_activity_queue::LocalActivityQueue::new(),
    );

    // 4. Create History
    let mut event1 = default_event(1, EventType::WorkflowExecutionStarted);
    event1.attributes = Some(EventAttributes::WorkflowExecutionStartedEventAttributes(
        Box::new(default_started_attrs("QueryWorkflow")),
    ));

    let event2 = default_event(2, EventType::DecisionTaskScheduled);
    let event3 = default_event(3, EventType::DecisionTaskStarted);

    let events = vec![event1, event2, event3];

    // 5. Create Query
    let mut queries = HashMap::new();
    queries.insert(
        "q1".to_string(),
        WorkflowQuery {
            query_type: "get_state".to_string(),
            query_args: Some(b"query-input".to_vec()),
        },
    );

    let task = PollForDecisionTaskResponse {
        task_token: b"token".to_vec(),
        workflow_execution: Some(WorkflowExecution {
            workflow_id: "w1".to_string(),
            run_id: "r1".to_string(),
        }),
        workflow_type: Some(WorkflowType {
            name: "QueryWorkflow".to_string(),
        }),
        history: Some(History { events }),
        queries: Some(queries),
        previous_started_event_id: 0,
        started_event_id: 3,
        attempt: 0,
        backlog_count_hint: 0,
        next_page_token: None,
        query: None,
        workflow_execution_task_list: None,
        scheduled_timestamp: None,
        started_timestamp: None,
    };

    // 6. Execute
    let (_, query_results) = executor.execute_decision_task(task).await.unwrap();

    // 7. Verify Query Result
    assert!(query_results.contains_key("q1"));
    let result = query_results.get("q1").unwrap();
    assert_eq!(result.answer, Some(b"query-input".to_vec()));
}
