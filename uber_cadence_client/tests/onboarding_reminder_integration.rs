//! Onboarding Reminder Flow Integration Test
//!
//! This test verifies that `ctx.sleep()` works correctly in a workflow.
//! It implements an "onboarding reminder flow" that sends a welcome email,
//! waits 5 seconds, then sends a reminder email.
//!
//! ## Prerequisites
//!
//! Start the Cadence server using Docker Compose:
//! ```bash
//! docker compose up -d
//! ```
//!
//! Wait for the services to be fully ready (2-3 minutes):
//! ```bash
//! docker compose ps
//! ```
//!
//! ## Running Tests
//!
//! Run the integration test (requires running Cadence server):
//! ```bash
//! cargo test --test onboarding_reminder_integration -- --ignored --test-threads=1 --nocapture
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uber_cadence_activity::{activity, ActivityContext};
use uber_cadence_client::error::TransportError;
use uber_cadence_client::GrpcWorkflowServiceClient;
use uber_cadence_core::ActivityOptions;
use uber_cadence_proto::shared::{
    EventAttributes, EventType, HistoryEventFilterType, TaskList, TaskListKind, WorkflowExecution,
    WorkflowType,
};
use uber_cadence_proto::workflow_service::{
    GetWorkflowExecutionHistoryRequest, RegisterDomainRequest, StartWorkflowExecutionRequest,
    WorkflowService,
};
use uber_cadence_worker::registry::{ActivityError, WorkflowError};
use uber_cadence_worker::{CadenceWorker, Worker, WorkerOptions};
use uber_cadence_workflow::{call_activity, workflow, WorkflowContext};
use uuid::Uuid;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
struct EmailRequest {
    to: String,
    subject: String,
    body: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OnboardingInput {
    user_email: String,
    user_name: String,
}

// ============================================================================
// Activity Implementation
// ============================================================================

#[activity(name = "send_email")]
async fn send_email(_ctx: &ActivityContext, input: EmailRequest) -> Result<(), ActivityError> {
    // Stubbed: just log/print the email
    println!(
        "[SEND_EMAIL] To: {} | Subject: {} | Body: {}",
        input.to, input.subject, input.body
    );

    Ok(())
}

// ============================================================================
// Workflow Implementation
// ============================================================================

#[workflow(name = "onboarding_reminder")]
async fn onboarding_reminder(
    ctx: WorkflowContext,
    input: OnboardingInput,
) -> Result<(), WorkflowError> {
    // Activity options
    let options = ActivityOptions {
        task_list: "".to_string(),
        schedule_to_close_timeout: Duration::from_secs(60),
        schedule_to_start_timeout: Duration::from_secs(60),
        start_to_close_timeout: Duration::from_secs(60),
        heartbeat_timeout: Duration::from_secs(60),
        retry_policy: None,
        wait_for_cancellation: false,
        local_activity: false,
    };

    // Step 1: Send welcome email
    let welcome_email = EmailRequest {
        to: input.user_email.clone(),
        subject: "Welcome to our platform!".to_string(),
        body: format!(
            "Hello {}, welcome to our platform! We're excited to have you.",
            input.user_name
        ),
    };

    let _: () = call_activity!(ctx, send_email, welcome_email, options.clone()).await?;

    println!("[WORKFLOW] Welcome email sent, sleeping for 5 seconds...");

    // Step 2: Sleep for 5 seconds
    ctx.sleep(Duration::from_secs(5)).await;

    println!("[WORKFLOW] Sleep completed, sending reminder email...");

    // Step 3: Send reminder email
    let reminder_email = EmailRequest {
        to: input.user_email.clone(),
        subject: "Don't forget to complete your onboarding!".to_string(),
        body: format!(
            "Hi {}, we noticed you haven't completed your onboarding yet. Take a few minutes to finish setting up your account!",
            input.user_name
        ),
    };

    let _: () = call_activity!(ctx, send_email, reminder_email, options.clone()).await?;

    println!("[WORKFLOW] Reminder email sent, workflow complete!");

    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

const CADENCE_GRPC_ENDPOINT: &str = "http://localhost:7833";

async fn create_grpc_client(domain: &str) -> Result<GrpcWorkflowServiceClient, TransportError> {
    GrpcWorkflowServiceClient::connect(CADENCE_GRPC_ENDPOINT, domain, None).await
}

fn generate_test_domain_name() -> String {
    format!("onboarding-test-domain-{}", Uuid::new_v4())
}

fn generate_task_list_name() -> String {
    format!("onboarding-task-list-{}", Uuid::new_v4())
}

fn generate_workflow_id() -> String {
    format!("onboarding-workflow-{}", Uuid::new_v4())
}

async fn register_domain_and_wait(
    client: &GrpcWorkflowServiceClient,
    domain_name: &str,
) -> Result<(), TransportError> {
    let register_request = RegisterDomainRequest {
        name: domain_name.to_string(),
        description: Some("Test domain for onboarding reminder flow".to_string()),
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

    client.register_domain(register_request).await?;

    // Wait for domain to propagate in the cache
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    Ok(())
}

fn create_workflow_request(
    domain: &str,
    task_list: &str,
    workflow_id: &str,
    input: &OnboardingInput,
) -> StartWorkflowExecutionRequest {
    StartWorkflowExecutionRequest {
        domain: domain.to_string(),
        workflow_id: workflow_id.to_string(),
        workflow_type: Some(WorkflowType {
            name: onboarding_reminder_cadence::NAME.to_string(),
        }),
        task_list: Some(TaskList {
            name: task_list.to_string(),
            kind: TaskListKind::Normal,
        }),
        input: Some(serde_json::to_vec(input).unwrap()),
        execution_start_to_close_timeout_seconds: Some(3600),
        task_start_to_close_timeout_seconds: Some(10),
        identity: "onboarding-test".to_string(),
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
    }
}

#[derive(Debug)]
pub enum WorkflowCompletionResult {
    Completed {
        result: Option<Vec<u8>>,
    },
    Failed {
        reason: String,
        details: Option<Vec<u8>>,
    },
    TimedOut,
}

async fn wait_for_workflow_completion(
    client: &GrpcWorkflowServiceClient,
    domain: &str,
    workflow_id: &str,
    run_id: &str,
    timeout: Duration,
) -> Result<WorkflowCompletionResult, TransportError> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            return Ok(WorkflowCompletionResult::TimedOut);
        }

        let history_request = GetWorkflowExecutionHistoryRequest {
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

        match client.get_workflow_execution_history(history_request).await {
            Ok(response) => {
                if let Some(history) = response.history {
                    for event in history.events {
                        match event.event_type {
                            EventType::WorkflowExecutionCompleted => {
                                if let Some(
                                    EventAttributes::WorkflowExecutionCompletedEventAttributes(
                                        attr,
                                    ),
                                ) = event.attributes
                                {
                                    return Ok(WorkflowCompletionResult::Completed {
                                        result: attr.result,
                                    });
                                }
                            }
                            EventType::WorkflowExecutionFailed => {
                                if let Some(
                                    EventAttributes::WorkflowExecutionFailedEventAttributes(attr),
                                ) = event.attributes
                                {
                                    return Ok(WorkflowCompletionResult::Failed {
                                        reason: attr.reason.clone().unwrap_or_default(),
                                        details: attr.details,
                                    });
                                }
                            }
                            _ => continue,
                        }
                    }
                }
            }
            Err(e) => return Err(e),
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn get_workflow_history(
    client: &GrpcWorkflowServiceClient,
    domain: &str,
    workflow_id: &str,
    run_id: &str,
) -> Result<uber_cadence_proto::shared::History, TransportError> {
    let history_request = GetWorkflowExecutionHistoryRequest {
        domain: domain.to_string(),
        execution: Some(WorkflowExecution {
            workflow_id: workflow_id.to_string(),
            run_id: run_id.to_string(),
        }),
        page_size: 1000,
        next_page_token: None,
        wait_for_new_event: false,
        history_event_filter_type: Some(HistoryEventFilterType::AllEvent),
        skip_archival: true,
        query_consistency_level: None,
    };

    let response = client
        .get_workflow_execution_history(history_request)
        .await?;
    Ok(response
        .history
        .unwrap_or_else(|| uber_cadence_proto::shared::History { events: vec![] }))
}

fn assert_activity_executed(
    history: &uber_cadence_proto::shared::History,
    activity_type: &str,
) -> bool {
    history.events.iter().any(|e| {
        if e.event_type == EventType::ActivityTaskScheduled {
            if let Some(EventAttributes::ActivityTaskScheduledEventAttributes(attr)) = &e.attributes
            {
                if let Some(atype) = &attr.activity_type {
                    return atype.name == activity_type;
                }
            }
        }
        false
    })
}

fn assert_timer_fired(history: &uber_cadence_proto::shared::History) -> bool {
    history
        .events
        .iter()
        .any(|e| e.event_type == EventType::TimerFired || e.event_type == EventType::TimerStarted)
}

// ============================================================================
// Test
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_onboarding_reminder_flow() {
    println!("\n=== Testing Onboarding Reminder Flow ===");
    println!("This test verifies that ctx.sleep() works correctly in workflows.\n");

    let domain_name = generate_test_domain_name();
    let task_list = generate_task_list_name();
    let workflow_id = generate_workflow_id();

    // Setup
    let client = create_grpc_client(&domain_name).await.unwrap();
    register_domain_and_wait(&client, &domain_name)
        .await
        .unwrap();

    println!("✓ Domain registered: {}", domain_name);

    let service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync> =
        Arc::new(client.clone());

    // Setup registry
    let registry = Arc::new(uber_cadence_worker::registry::WorkflowRegistry::new());
    onboarding_reminder_cadence::register(registry.as_ref());
    send_email_cadence::register(registry.as_ref());

    println!("✓ Workflow and activity registered");

    // Start worker
    let worker = CadenceWorker::new(
        &domain_name,
        &task_list,
        WorkerOptions {
            identity: "onboarding-worker".to_string(),
            disable_sticky_execution: true,
            ..Default::default()
        },
        registry,
        service.clone(),
    );

    worker.start().expect("Failed to start worker");

    println!("✓ Worker started");

    // Create input
    let onboarding_input = OnboardingInput {
        user_email: "newuser@example.com".to_string(),
        user_name: "Alice".to_string(),
    };

    // Start workflow
    let start_request =
        create_workflow_request(&domain_name, &task_list, &workflow_id, &onboarding_input);
    let response = client
        .start_workflow_execution(start_request)
        .await
        .unwrap();

    println!("✓ Workflow started: run_id={}", response.run_id);

    // Wait for workflow to complete (should take ~5+ seconds due to sleep)
    let start_time = std::time::Instant::now();
    let result = wait_for_workflow_completion(
        &client,
        &domain_name,
        &workflow_id,
        &response.run_id,
        Duration::from_secs(30),
    )
    .await;
    let elapsed = start_time.elapsed();

    match result {
        Ok(WorkflowCompletionResult::Completed { .. }) => {
            println!("✓ Workflow completed successfully in {:?}", elapsed);
        }
        Ok(WorkflowCompletionResult::Failed { reason, .. }) => {
            // Print history on failure
            let history =
                get_workflow_history(&client, &domain_name, &workflow_id, &response.run_id)
                    .await
                    .unwrap();
            println!("FAILED! History events:");
            for event in history.events {
                println!("  - {:?}", event.event_type);
            }
            panic!("Workflow failed: {}", reason);
        }
        Ok(WorkflowCompletionResult::TimedOut) => {
            // Print history on timeout
            let history =
                get_workflow_history(&client, &domain_name, &workflow_id, &response.run_id)
                    .await
                    .unwrap();
            println!("TIMEOUT! History events:");
            for event in history.events {
                println!("  - {:?}", event.event_type);
            }
            panic!("Workflow timed out");
        }
        Err(e) => panic!("Error waiting for workflow: {}", e),
    }

    // Get history and verify
    let history = get_workflow_history(&client, &domain_name, &workflow_id, &response.run_id)
        .await
        .unwrap();

    // Verify welcome email was sent
    assert!(
        assert_activity_executed(&history, "send_email"),
        "Welcome email should be sent"
    );

    // Verify timer was used (either TimerStarted or TimerFired event)
    assert!(
        assert_timer_fired(&history),
        "Timer should have been used for sleep"
    );

    // Verify the workflow took at least 5 seconds (accounting for some variance)
    assert!(
        elapsed >= Duration::from_secs(4),
        "Workflow should have taken at least 4 seconds due to sleep, but took {:?}",
        elapsed
    );

    println!("✓ Sleep/timer functionality verified");
    println!("✓ Activity execution verified");

    // Stop worker
    worker.stop();

    println!("\n=== Test Complete ===\n");
}
