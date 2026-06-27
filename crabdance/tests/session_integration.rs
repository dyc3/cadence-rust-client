#![cfg(feature = "integration")]

//! Session integration test.
//!
//! Verifies a session end-to-end against a live Cadence server: a workflow creates
//! a session, runs an activity inside it (pinned to the session worker's
//! resource-specific task list), and completes the session.
//!
//! ## Prerequisites
//!
//! ```bash
//! docker compose up -d   # then wait ~2-3 min for readiness
//! ```
//!
//! ## Running
//!
//! ```bash
//! cargo test -p crabdance --features integration --test session_integration -- --test-threads=1 --nocapture
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crabdance::activity::{activity, ActivityContext};
use crabdance::client::error::TransportError;
use crabdance::client::GrpcWorkflowServiceClient;
use crabdance::core::ActivityOptions;
use crabdance::proto::shared::{
    EventAttributes, EventType, HistoryEventFilterType, TaskList, TaskListKind, WorkflowExecution,
    WorkflowType,
};
use crabdance::proto::workflow_service::{
    GetWorkflowExecutionHistoryRequest, RegisterDomainRequest, StartWorkflowExecutionRequest,
    WorkflowService,
};
use crabdance::worker::registry::{ActivityError, WorkflowError};
use crabdance::worker::{CadenceWorker, Worker, WorkerOptions};
use crabdance::workflow::{workflow, SessionOptions, WorkflowContext};

const CADENCE_GRPC_ENDPOINT: &str = "http://localhost:7833";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionInput {
    payload: String,
}

#[activity(name = "process_in_session")]
async fn process_in_session(
    _ctx: &ActivityContext,
    input: SessionInput,
) -> Result<String, ActivityError> {
    Ok(format!("processed: {}", input.payload))
}

#[workflow(name = "session_workflow")]
async fn session_workflow(
    ctx: WorkflowContext,
    input: SessionInput,
) -> Result<String, WorkflowError> {
    // Create a session — its activities run on the same worker.
    let session = ctx
        .create_session("", SessionOptions::default())
        .await
        .map_err(|e| WorkflowError::message(format!("create_session failed: {e}")))?;

    let options = ActivityOptions {
        start_to_close_timeout: Duration::from_secs(30),
        schedule_to_start_timeout: Duration::from_secs(30),
        ..Default::default()
    };

    let result_bytes = ctx
        .execute_activity_in_session(
            &session,
            "process_in_session",
            Some(serde_json::to_vec(&input).unwrap()),
            options,
        )
        .await
        .map_err(|e| WorkflowError::message(format!("session activity failed: {e}")))?;

    ctx.complete_session(&session).await;

    let result: String = serde_json::from_slice(&result_bytes).unwrap_or_default();
    Ok(result)
}

async fn create_grpc_client(domain: &str) -> Result<GrpcWorkflowServiceClient, TransportError> {
    GrpcWorkflowServiceClient::connect(CADENCE_GRPC_ENDPOINT, domain, None).await
}

async fn register_domain_and_wait(
    client: &GrpcWorkflowServiceClient,
    domain_name: &str,
) -> Result<(), TransportError> {
    let request = RegisterDomainRequest {
        name: domain_name.to_string(),
        description: Some("Session integration test domain".to_string()),
        owner_email: "test@example.com".to_string(),
        workflow_execution_retention_period_in_days: 1,
        emit_metric: None,
        data: HashMap::new(),
        security_token: None,
        is_global_domain: None,
        active_cluster_name: String::new(),
        clusters: vec![],
        history_archival_status: None,
        history_archival_uri: None,
        visibility_archival_status: None,
        visibility_archival_uri: None,
    };
    client.register_domain(request).await?;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    Ok(())
}

async fn wait_for_completion(
    client: &GrpcWorkflowServiceClient,
    domain: &str,
    workflow_id: &str,
    run_id: &str,
    timeout: Duration,
) -> Option<Vec<u8>> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            return None;
        }
        let request = GetWorkflowExecutionHistoryRequest {
            domain: domain.to_string(),
            execution: Some(WorkflowExecution {
                workflow_id: workflow_id.to_string(),
                run_id: run_id.to_string(),
            }),
            page_size: 1000,
            next_page_token: None,
            wait_for_new_event: true,
            history_event_filter_type: Some(HistoryEventFilterType::AllEvent),
            skip_archival: true,
            query_consistency_level: None,
        };
        if let Ok(response) = client.get_workflow_execution_history(request).await {
            if let Some(history) = response.history {
                for event in history.events {
                    if event.event_type == EventType::WorkflowExecutionCompleted {
                        if let Some(EventAttributes::WorkflowExecutionCompletedEventAttributes(
                            attr,
                        )) = event.attributes
                        {
                            return Some(attr.result.unwrap_or_default());
                        }
                    }
                    if event.event_type == EventType::WorkflowExecutionFailed {
                        panic!("session workflow failed");
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::test]
async fn test_session_across_activities() {
    let domain_name = format!("session-test-domain-{}", Uuid::new_v4());
    let task_list = format!("session-task-list-{}", Uuid::new_v4());
    let workflow_id = format!("session-workflow-{}", Uuid::new_v4());

    let client = create_grpc_client(&domain_name).await.unwrap();
    register_domain_and_wait(&client, &domain_name)
        .await
        .unwrap();

    let service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync> =
        Arc::new(client.clone());

    let registry = Arc::new(crabdance::worker::registry::WorkflowRegistry::new());
    session_workflow_cadence::register(registry.as_ref());
    process_in_session_cadence::register(registry.as_ref());

    // Enable the session worker so the internal creation/completion activities and the
    // resource-specific task list are served.
    let worker = CadenceWorker::new(
        &domain_name,
        &task_list,
        WorkerOptions {
            identity: "session-worker".to_string(),
            disable_sticky_execution: true,
            enable_session_worker: true,
            max_concurrent_session_execution_size: 4,
            ..Default::default()
        },
        registry,
        service.clone(),
    );
    worker.start().expect("failed to start worker");

    let input = SessionInput {
        payload: "hello".to_string(),
    };
    let start_request = StartWorkflowExecutionRequest {
        domain: domain_name.clone(),
        workflow_id: workflow_id.clone(),
        workflow_type: Some(WorkflowType {
            name: session_workflow_cadence::NAME.to_string(),
        }),
        task_list: Some(TaskList {
            name: task_list.clone(),
            kind: TaskListKind::Normal,
        }),
        input: Some(serde_json::to_vec(&input).unwrap()),
        execution_start_to_close_timeout_seconds: Some(3600),
        task_start_to_close_timeout_seconds: Some(10),
        identity: "session-test".to_string(),
        request_id: Uuid::new_v4().to_string(),
        workflow_id_reuse_policy: None,
        retry_policy: None,
        cron_schedule: None,
        memo: None,
        search_attributes: None,
        header: None,
        delay_start_seconds: None,
        jitter_start_seconds: None,
        first_execution_run_id: None,
        first_decision_task_backoff_seconds: None,
        partition_config: None,
    };

    let response = client
        .start_workflow_execution(start_request)
        .await
        .unwrap();

    let result = wait_for_completion(
        &client,
        &domain_name,
        &workflow_id,
        &response.run_id,
        Duration::from_secs(60),
    )
    .await
    .expect("session workflow timed out");

    let output: String = serde_json::from_slice(&result).unwrap();
    assert_eq!(output, "processed: hello");
}
