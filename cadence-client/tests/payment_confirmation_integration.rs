//! Payment Confirmation Flow Integration Test
//!
//! This test verifies signal handling in workflows by implementing a payment confirmation flow.
//! The workflow starts a payment, waits for a webhook signal confirming the payment status,
//! then processes the result. This demonstrates real-world signal usage patterns including
//! waiting for external events.
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
//! cargo test --test payment_confirmation_integration -- --ignored --test-threads=1 --nocapture
//! ```

use cadence_activity::ActivityContext;
use cadence_client::GrpcWorkflowServiceClient;
use cadence_core::{ActivityOptions, CadenceError};
use cadence_proto::shared::{
    EventAttributes, EventType, HistoryEventFilterType, TaskList, TaskListKind, WorkflowExecution,
    WorkflowType,
};
use cadence_proto::workflow_service::{
    GetWorkflowExecutionHistoryRequest, RegisterDomainRequest, SignalWorkflowExecutionRequest,
    StartWorkflowExecutionRequest, WorkflowService,
};
use cadence_worker::registry::{Activity, ActivityError, Registry, Workflow, WorkflowError};
use cadence_worker::{CadenceWorker, Worker, WorkerOptions};
use cadence_workflow::WorkflowContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PaymentInput {
    order_id: String,
    amount: f64,
    customer_email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PaymentIntent {
    payment_intent_id: String,
    status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PaymentConfirmedSignal {
    payment_intent_id: String,
    amount: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PaymentFailedSignal {
    payment_intent_id: String,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OrderResult {
    order_id: String,
    status: OrderStatus,
    message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum OrderStatus {
    Confirmed,
    Failed,
    Timeout,
}

// ============================================================================
// Activities
// ============================================================================

#[derive(Clone)]
struct InitiatePaymentActivity;

impl Activity for InitiatePaymentActivity {
    fn execute(
        &self,
        _ctx: &mut ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, ActivityError> {
        let input_data =
            input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
        let payment_input: PaymentInput = serde_json::from_slice(&input_data)
            .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))?;

        // Stubbed: generate a mock payment intent ID
        let payment_intent = PaymentIntent {
            payment_intent_id: format!("pi_{}", Uuid::new_v4().to_string().replace("-", "")),
            status: "requires_confirmation".to_string(),
        };

        println!(
            "[INITIATE_PAYMENT] Order: {} | Amount: ${:.2} | Payment Intent: {}",
            payment_input.order_id, payment_input.amount, payment_intent.payment_intent_id
        );

        serde_json::to_vec(&payment_intent)
            .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))
    }
}

#[derive(Clone)]
struct SendReceiptEmailActivity;

impl Activity for SendReceiptEmailActivity {
    fn execute(
        &self,
        _ctx: &mut ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, ActivityError> {
        let input_data =
            input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
        let (order_id, amount, email): (String, f64, String) = serde_json::from_slice(&input_data)
            .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))?;

        println!(
            "[SEND_RECEIPT] To: {} | Order: {} | Amount: ${:.2}",
            email, order_id, amount
        );

        Ok(vec![])
    }
}

#[derive(Clone)]
struct SendFailureNotificationActivity;

impl Activity for SendFailureNotificationActivity {
    fn execute(
        &self,
        _ctx: &mut ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, ActivityError> {
        let input_data =
            input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
        let (order_id, reason, email): (String, String, String) =
            serde_json::from_slice(&input_data)
                .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))?;

        println!(
            "[SEND_FAILURE_NOTIFICATION] To: {} | Order: {} | Reason: {}",
            email, order_id, reason
        );

        Ok(vec![])
    }
}

// ============================================================================
// Workflow
// ============================================================================

#[derive(Clone)]
struct PaymentConfirmationWorkflow;

impl Workflow for PaymentConfirmationWorkflow {
    fn execute(
        &self,
        ctx: WorkflowContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async move {
            let input_data =
                input.ok_or_else(|| WorkflowError::ExecutionFailed("Missing input".to_string()))?;
            let payment_input: PaymentInput = serde_json::from_slice(&input_data)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            println!(
                "[WORKFLOW] Starting payment confirmation for order: {}",
                payment_input.order_id
            );

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

            // Step 1: Initiate payment
            let payment_intent_bytes = ctx
                .execute_activity(
                    "initiate_payment",
                    Some(input_data.clone()),
                    options.clone(),
                )
                .await
                .map_err(|e| WorkflowError::ActivityFailed(e.to_string()))?;

            let payment_intent: PaymentIntent = serde_json::from_slice(&payment_intent_bytes)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            println!(
                "[WORKFLOW] Payment initiated: {} | Waiting for confirmation signal...",
                payment_intent.payment_intent_id
            );

            // Step 2: Create signal channels
            let mut confirmed_channel = ctx.get_signal_channel("payment_confirmed");
            let mut failed_channel = ctx.get_signal_channel("payment_failed");

            // Step 3: Wait for signal with timeout (30 seconds)
            let timeout_duration = Duration::from_secs(30);
            let signal_timeout = ctx.sleep(timeout_duration);

            tokio::select! {
                // Handle payment confirmed signal
                signal_data = async { confirmed_channel.recv().await } => {
                    if let Some(data) = signal_data {
                        if let Ok(signal) = serde_json::from_slice::<PaymentConfirmedSignal>(&data) {
                            println!(
                                "[WORKFLOW] Payment confirmed signal received: {}",
                                signal.payment_intent_id
                            );

                            // Step 4a: Send receipt email
                            let receipt_input = (
                                payment_input.order_id.clone(),
                                signal.amount,
                                payment_input.customer_email.clone(),
                            );
                            let receipt_bytes = serde_json::to_vec(&receipt_input)
                                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

                            ctx.execute_activity("send_receipt_email", Some(receipt_bytes), options.clone())
                                .await
                                .map_err(|e| WorkflowError::ActivityFailed(e.to_string()))?;

                            println!("[WORKFLOW] Receipt sent, order confirmed");

                            // Return success result
                            let result = OrderResult {
                                order_id: payment_input.order_id,
                                status: OrderStatus::Confirmed,
                                message: format!("Payment confirmed for ${:.2}", signal.amount),
                            };

                            return serde_json::to_vec(&result)
                                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()));
                        }
                    }
                    Err(WorkflowError::ExecutionFailed("Invalid signal data".to_string()))
                }

                // Handle payment failed signal
                signal_data = async { failed_channel.recv().await } => {
                    if let Some(data) = signal_data {
                        if let Ok(signal) = serde_json::from_slice::<PaymentFailedSignal>(&data) {
                            println!(
                                "[WORKFLOW] Payment failed signal received: {} | Reason: {}",
                                signal.payment_intent_id, signal.reason
                            );

                            // Step 4b: Send failure notification
                            let failure_input = (
                                payment_input.order_id.clone(),
                                signal.reason.clone(),
                                payment_input.customer_email.clone(),
                            );
                            let failure_bytes = serde_json::to_vec(&failure_input)
                                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

                            ctx.execute_activity("send_failure_notification", Some(failure_bytes), options.clone())
                                .await
                                .map_err(|e| WorkflowError::ActivityFailed(e.to_string()))?;

                            println!("[WORKFLOW] Failure notification sent");

                            // Return failure result
                            let result = OrderResult {
                                order_id: payment_input.order_id,
                                status: OrderStatus::Failed,
                                message: format!("Payment failed: {}", signal.reason),
                            };

                            return serde_json::to_vec(&result)
                                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()));
                        }
                    }
                    Err(WorkflowError::ExecutionFailed("Invalid signal data".to_string()))
                }

                // Handle timeout
                _ = signal_timeout => {
                    println!("[WORKFLOW] Payment confirmation timed out");

                    // Step 4c: Send failure notification for timeout
                    let failure_input = (
                        payment_input.order_id.clone(),
                        "Payment confirmation timeout".to_string(),
                        payment_input.customer_email.clone(),
                    );
                    let failure_bytes = serde_json::to_vec(&failure_input)
                        .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

                    ctx.execute_activity("send_failure_notification", Some(failure_bytes), options.clone())
                        .await
                        .map_err(|e| WorkflowError::ActivityFailed(e.to_string()))?;

                    println!("[WORKFLOW] Timeout notification sent");

                    // Return timeout result
                    let result = OrderResult {
                        order_id: payment_input.order_id,
                        status: OrderStatus::Timeout,
                        message: "Payment confirmation timed out".to_string(),
                    };

                    serde_json::to_vec(&result)
                        .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))
                }
            }
        })
    }
}

// ============================================================================
// Helpers
// ============================================================================

const CADENCE_GRPC_ENDPOINT: &str = "http://localhost:7833";

async fn create_grpc_client(domain: &str) -> Result<GrpcWorkflowServiceClient, CadenceError> {
    GrpcWorkflowServiceClient::connect(CADENCE_GRPC_ENDPOINT, domain, None).await
}

fn generate_test_domain_name() -> String {
    format!("payment-test-domain-{}", Uuid::new_v4())
}

fn generate_task_list_name() -> String {
    format!("payment-task-list-{}", Uuid::new_v4())
}

fn generate_workflow_id() -> String {
    format!("payment-workflow-{}", Uuid::new_v4())
}

async fn register_domain_and_wait(
    client: &GrpcWorkflowServiceClient,
    domain_name: &str,
) -> Result<(), CadenceError> {
    let register_request = RegisterDomainRequest {
        name: domain_name.to_string(),
        description: Some("Test domain for payment confirmation flow".to_string()),
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

    let _ = client.register_domain(register_request).await;

    // Wait for domain to propagate in the cache
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    Ok(())
}

fn create_payment_workflow_request(
    domain: &str,
    task_list: &str,
    workflow_id: &str,
    input: &PaymentInput,
) -> StartWorkflowExecutionRequest {
    StartWorkflowExecutionRequest {
        domain: domain.to_string(),
        workflow_id: workflow_id.to_string(),
        workflow_type: Some(WorkflowType {
            name: "payment_confirmation".to_string(),
        }),
        task_list: Some(TaskList {
            name: task_list.to_string(),
            kind: TaskListKind::Normal,
        }),
        input: Some(serde_json::to_vec(input).unwrap()),
        execution_start_to_close_timeout_seconds: Some(3600),
        task_start_to_close_timeout_seconds: Some(10),
        identity: "payment-test".to_string(),
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
) -> Result<WorkflowCompletionResult, CadenceError> {
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
) -> Result<cadence_proto::shared::History, CadenceError> {
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
        .unwrap_or_else(|| cadence_proto::shared::History { events: vec![] }))
}

fn assert_activity_executed(history: &cadence_proto::shared::History, activity_type: &str) -> bool {
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

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_payment_confirmation_success() {
    println!("\n=== Testing Payment Confirmation Success ===");
    println!("This test verifies signal handling for payment confirmation flow.\n");

    let domain_name = generate_test_domain_name();
    let task_list = generate_task_list_name();
    let workflow_id = generate_workflow_id();

    // Setup
    let client = create_grpc_client(&domain_name).await.unwrap();
    register_domain_and_wait(&client, &domain_name)
        .await
        .unwrap();

    println!("✓ Domain registered: {}", domain_name);

    let service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync> =
        Arc::new(client.clone());

    // Setup registry
    let registry = Arc::new(cadence_worker::registry::WorkflowRegistry::new());
    registry.register_workflow(
        "payment_confirmation",
        Box::new(PaymentConfirmationWorkflow),
    );
    registry.register_activity("initiate_payment", Box::new(InitiatePaymentActivity));
    registry.register_activity("send_receipt_email", Box::new(SendReceiptEmailActivity));
    registry.register_activity(
        "send_failure_notification",
        Box::new(SendFailureNotificationActivity),
    );

    println!("✓ Workflow and activities registered");

    // Start worker
    let worker = CadenceWorker::new(
        &domain_name,
        &task_list,
        WorkerOptions {
            identity: "payment-worker".to_string(),
            disable_sticky_execution: true,
            ..Default::default()
        },
        registry,
        service.clone(),
    );

    worker.start().expect("Failed to start worker");

    println!("✓ Worker started");

    // Create payment input
    let payment_input = PaymentInput {
        order_id: format!("order-{}", Uuid::new_v4()),
        amount: 99.99,
        customer_email: "customer@example.com".to_string(),
    };

    // Start workflow
    let start_request =
        create_payment_workflow_request(&domain_name, &task_list, &workflow_id, &payment_input);
    let response = client
        .start_workflow_execution(start_request)
        .await
        .unwrap();

    println!("✓ Workflow started: run_id={}", response.run_id);

    // Wait briefly for workflow to initiate payment
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Send payment_confirmed signal
    let confirmed_signal = PaymentConfirmedSignal {
        payment_intent_id: "pi_test_123".to_string(),
        amount: 99.99,
    };

    let signal_request = SignalWorkflowExecutionRequest {
        domain: domain_name.clone(),
        workflow_execution: Some(WorkflowExecution {
            workflow_id: workflow_id.clone(),
            run_id: response.run_id.clone(),
        }),
        signal_name: "payment_confirmed".to_string(),
        input: Some(serde_json::to_vec(&confirmed_signal).unwrap()),
        identity: "payment-test".to_string(),
        request_id: Uuid::new_v4().to_string(),
        control: None,
    };

    client
        .signal_workflow_execution(signal_request)
        .await
        .expect("Failed to send payment_confirmed signal");

    println!("✓ Payment confirmation signal sent");

    // Wait for workflow to complete
    let result = wait_for_workflow_completion(
        &client,
        &domain_name,
        &workflow_id,
        &response.run_id,
        Duration::from_secs(30),
    )
    .await;

    match result {
        Ok(WorkflowCompletionResult::Completed { result }) => {
            let order_result: OrderResult =
                serde_json::from_slice(&result.unwrap()).expect("Failed to parse result");
            println!("✓ Workflow completed successfully");
            println!("  Status: {:?}", order_result.status);
            println!("  Message: {}", order_result.message);
            assert_eq!(order_result.status, OrderStatus::Confirmed);
        }
        Ok(WorkflowCompletionResult::Failed { reason, .. }) => {
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

    // Verify activities executed
    assert!(
        assert_activity_executed(&history, "initiate_payment"),
        "Initiate payment activity should execute"
    );
    assert!(
        assert_activity_executed(&history, "send_receipt_email"),
        "Send receipt email activity should execute"
    );

    // Verify failure notification was NOT sent
    assert!(
        !assert_activity_executed(&history, "send_failure_notification"),
        "Send failure notification should NOT execute for successful payment"
    );

    println!("✓ Payment confirmation flow verified successfully");
    println!("✓ Signal handling and activity execution verified");

    // Stop worker
    worker.stop();

    println!("\n=== Test Complete ===\n");
}

#[tokio::test]
#[ignore]
async fn test_payment_confirmation_failure() {
    println!("\n=== Testing Payment Confirmation Failure ===");
    println!("This test verifies signal handling for payment failure flow.\n");

    let domain_name = generate_test_domain_name();
    let task_list = generate_task_list_name();
    let workflow_id = generate_workflow_id();

    // Setup
    let client = create_grpc_client(&domain_name).await.unwrap();
    register_domain_and_wait(&client, &domain_name)
        .await
        .unwrap();

    println!("✓ Domain registered: {}", domain_name);

    let service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync> =
        Arc::new(client.clone());

    // Setup registry
    let registry = Arc::new(cadence_worker::registry::WorkflowRegistry::new());
    registry.register_workflow(
        "payment_confirmation",
        Box::new(PaymentConfirmationWorkflow),
    );
    registry.register_activity("initiate_payment", Box::new(InitiatePaymentActivity));
    registry.register_activity("send_receipt_email", Box::new(SendReceiptEmailActivity));
    registry.register_activity(
        "send_failure_notification",
        Box::new(SendFailureNotificationActivity),
    );

    println!("✓ Workflow and activities registered");

    // Start worker
    let worker = CadenceWorker::new(
        &domain_name,
        &task_list,
        WorkerOptions {
            identity: "payment-worker-failure".to_string(),
            disable_sticky_execution: true,
            ..Default::default()
        },
        registry,
        service.clone(),
    );

    worker.start().expect("Failed to start worker");

    println!("✓ Worker started");

    // Create payment input
    let payment_input = PaymentInput {
        order_id: format!("order-{}", Uuid::new_v4()),
        amount: 149.99,
        customer_email: "customer@example.com".to_string(),
    };

    // Start workflow
    let start_request =
        create_payment_workflow_request(&domain_name, &task_list, &workflow_id, &payment_input);
    let response = client
        .start_workflow_execution(start_request)
        .await
        .unwrap();

    println!("✓ Workflow started: run_id={}", response.run_id);

    // Wait briefly for workflow to initiate payment
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Send payment_failed signal
    let failed_signal = PaymentFailedSignal {
        payment_intent_id: "pi_test_456".to_string(),
        reason: "Insufficient funds".to_string(),
    };

    let signal_request = SignalWorkflowExecutionRequest {
        domain: domain_name.clone(),
        workflow_execution: Some(WorkflowExecution {
            workflow_id: workflow_id.clone(),
            run_id: response.run_id.clone(),
        }),
        signal_name: "payment_failed".to_string(),
        input: Some(serde_json::to_vec(&failed_signal).unwrap()),
        identity: "payment-test".to_string(),
        request_id: Uuid::new_v4().to_string(),
        control: None,
    };

    client
        .signal_workflow_execution(signal_request)
        .await
        .expect("Failed to send payment_failed signal");

    println!("✓ Payment failure signal sent");

    // Wait for workflow to complete
    let result = wait_for_workflow_completion(
        &client,
        &domain_name,
        &workflow_id,
        &response.run_id,
        Duration::from_secs(30),
    )
    .await;

    match result {
        Ok(WorkflowCompletionResult::Completed { result }) => {
            let order_result: OrderResult =
                serde_json::from_slice(&result.unwrap()).expect("Failed to parse result");
            println!("✓ Workflow completed");
            println!("  Status: {:?}", order_result.status);
            println!("  Message: {}", order_result.message);
            assert_eq!(order_result.status, OrderStatus::Failed);
        }
        Ok(WorkflowCompletionResult::Failed { reason, .. }) => {
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

    // Verify activities executed
    assert!(
        assert_activity_executed(&history, "initiate_payment"),
        "Initiate payment activity should execute"
    );
    assert!(
        assert_activity_executed(&history, "send_failure_notification"),
        "Send failure notification activity should execute"
    );

    // Verify receipt email was NOT sent
    assert!(
        !assert_activity_executed(&history, "send_receipt_email"),
        "Send receipt email should NOT execute for failed payment"
    );

    println!("✓ Payment failure flow verified successfully");
    println!("✓ Signal handling and activity execution verified");

    // Stop worker
    worker.stop();

    println!("\n=== Test Complete ===\n");
}
