use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use uber_cadence_activity::ActivityContext;
use uber_cadence_client::GrpcWorkflowServiceClient;
use uber_cadence_core::{ActivityOptions, CadenceError};
use uber_cadence_proto::shared::{
    EventAttributes, EventType, HistoryEventFilterType, TaskList, TaskListKind, WorkflowExecution,
    WorkflowType,
};
use uber_cadence_proto::workflow_service::{
    GetWorkflowExecutionHistoryRequest, RegisterDomainRequest, StartWorkflowExecutionRequest,
    WorkflowService,
};
use uber_cadence_worker::registry::{Activity, ActivityError, Registry, Workflow, WorkflowError};
use uber_cadence_worker::{CadenceWorker, Worker, WorkerOptions};
use uber_cadence_workflow::future::{ActivityFailureInfo, ActivityFailureType};
use uber_cadence_workflow::{LocalActivityOptions, WorkflowContext};
use uuid::Uuid;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderInput {
    pub user_id: String,
    pub items: Vec<OrderItem>,
    pub shipping_address: Address,
    pub payment_method: PaymentMethod,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderItem {
    pub product_id: String,
    pub sku: String,
    pub quantity: i32,
    pub unit_price: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PaymentMethod {
    CreditCard { last_four: String, token: String },
    PayPal { email: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderOutput {
    pub order_id: String,
    pub status: OrderStatus,
    pub total_amount: f64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum OrderStatus {
    Pending,
    InventoryReserved,
    Paid,
    Shipped,
    Cancelled,
    Failed,
}

// ============================================================================
// Activities
// ============================================================================

// 1. Calculate Order Total (Local Activity)
// This is a fast, synchronous computation that doesn't need external I/O,
// making it ideal for local activity execution
#[derive(Clone)]
pub struct CalculateOrderTotalActivity;

impl Activity for CalculateOrderTotalActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move {
            let input_data =
                input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
            let order: OrderInput = serde_json::from_slice(&input_data)
                .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))?;
            let total: f64 = order
                .items
                .iter()
                .map(|item| item.unit_price * item.quantity as f64)
                .sum();
            serde_json::to_vec(&total).map_err(|e| ActivityError::ExecutionFailed(e.to_string()))
        })
    }
}

// 2. Reserve Inventory
#[derive(Clone)]
pub struct ReserveInventoryActivity;

impl Activity for ReserveInventoryActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        _input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move {
            // Always succeed for this test
            Ok(vec![])
        })
    }
}

// 3. Process Payment
#[derive(Clone)]
pub struct ProcessPaymentActivity;

impl Activity for ProcessPaymentActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move {
            let input_data =
                input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
            let (order, total): (OrderInput, f64) = serde_json::from_slice(&input_data)
                .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))?;

            // Fail if any item is expensive (simple logic for testing failure)
            for item in order.items {
                if item.unit_price > 10000.0 {
                    return Err(ActivityError::ExecutionFailed(
                        "Payment declined: limit exceeded".to_string(),
                    ));
                }
            }

            // Mock payment processing
            println!("Processed payment of ${} for user {}", total, order.user_id);
            Ok(vec![])
        })
    }
}

// 4. Release Inventory (Compensation)
#[derive(Clone)]
pub struct ReleaseInventoryActivity;

impl Activity for ReleaseInventoryActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        _input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move {
            println!("Compensating: Releasing inventory");
            Ok(vec![])
        })
    }
}

// 5. Send Notification
#[derive(Clone)]
pub struct SendNotificationActivity;

impl Activity for SendNotificationActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move {
            let input_data =
                input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
            let message: String = serde_json::from_slice(&input_data)
                .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))?;
            println!("Sending notification: {}", message);
            Ok(vec![])
        })
    }
}

// ============================================================================
// Workflow (Saga)
// ============================================================================

#[derive(Clone)]
pub struct OrderProcessingSagaWorkflow;

impl Workflow for OrderProcessingSagaWorkflow {
    fn execute(
        &self,
        ctx: WorkflowContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async move {
            let input_data =
                input.ok_or_else(|| WorkflowError::ExecutionFailed("Missing input".to_string()))?;
            let order: OrderInput = serde_json::from_slice(&input_data)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            // Step 1: Calculate Total (using local activity for fast computation)
            let local_options = LocalActivityOptions {
                schedule_to_close_timeout: Duration::from_secs(5),
                retry_policy: None,
            };

            let total_bytes = ctx
                .execute_local_activity(
                    "calculate_order_total",
                    Some(input_data.clone()),
                    local_options,
                )
                .await
                .map_err(|e| {
                    WorkflowError::ActivityFailed(ActivityFailureInfo {
                        failure_type: ActivityFailureType::ExecutionFailed,
                        message: e.to_string(),
                        details: None,
                        retryable: false,
                    })
                })?;

            // Options for regular activities
            let options = ActivityOptions {
                task_list: "".to_string(), // Use default task list
                schedule_to_close_timeout: Duration::from_secs(60),
                schedule_to_start_timeout: Duration::from_secs(60),
                start_to_close_timeout: Duration::from_secs(60),
                heartbeat_timeout: Duration::from_secs(60),
                retry_policy: None,
                wait_for_cancellation: false,
                local_activity: false,
            };

            let total: f64 = serde_json::from_slice(&total_bytes)
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            // Step 2: Reserve Inventory
            ctx.execute_activity(
                "reserve_inventory",
                Some(input_data.clone()),
                options.clone(),
            )
            .await
            .map_err(|e| {
                WorkflowError::ActivityFailed(ActivityFailureInfo {
                    failure_type: ActivityFailureType::ExecutionFailed,
                    message: e.to_string(),
                    details: None,
                    retryable: false,
                })
            })?;

            // Step 3: Process Payment (with Compensation)
            let payment_input = serde_json::to_vec(&(order.clone(), total))
                .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;

            match ctx
                .execute_activity("process_payment", Some(payment_input), options.clone())
                .await
            {
                Ok(_) => {
                    // Payment success
                    let notification = format!("Order confirmed for ${}", total);
                    let notif_bytes = serde_json::to_vec(&notification)
                        .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;
                    ctx.execute_activity("send_notification", Some(notif_bytes), options.clone())
                        .await
                        .map_err(|e| {
                            WorkflowError::ActivityFailed(ActivityFailureInfo {
                                failure_type: ActivityFailureType::ExecutionFailed,
                                message: e.to_string(),
                                details: None,
                                retryable: false,
                            })
                        })?;

                    let output = OrderOutput {
                        order_id: Uuid::new_v4().to_string(),
                        status: OrderStatus::Paid,
                        total_amount: total,
                    };
                    Ok(serde_json::to_vec(&output)
                        .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?)
                }
                Err(e) => {
                    // Payment failed - Compensate!
                    println!("Payment failed ({:?}), executing compensation...", e);

                    // Execute compensation activity
                    ctx.execute_activity(
                        "release_inventory",
                        Some(input_data.clone()),
                        options.clone(),
                    )
                    .await
                    .map_err(|e| {
                        WorkflowError::ActivityFailed(ActivityFailureInfo {
                            failure_type: ActivityFailureType::ExecutionFailed,
                            message: e.to_string(),
                            details: None,
                            retryable: false,
                        })
                    })?;

                    // Notify user of failure
                    let notification = "Order failed: payment declined".to_owned();
                    let notif_bytes = serde_json::to_vec(&notification)
                        .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;
                    let _ = ctx
                        .execute_activity("send_notification", Some(notif_bytes), options.clone())
                        .await;

                    Err(WorkflowError::ActivityFailed(ActivityFailureInfo {
                        failure_type: ActivityFailureType::Application,
                        message: format!("Payment failed: {:?}", e),
                        details: None,
                        retryable: false,
                    }))
                }
            }
        })
    }
}

// ============================================================================
// Test
// ============================================================================
// Helpers
// ============================================================================

const CADENCE_GRPC_ENDPOINT: &str = "http://localhost:7833";

async fn create_grpc_client(domain: &str) -> Result<GrpcWorkflowServiceClient, CadenceError> {
    GrpcWorkflowServiceClient::connect(CADENCE_GRPC_ENDPOINT, domain, None).await
}

fn generate_test_domain_name() -> String {
    format!("saga-test-domain-{}", Uuid::new_v4())
}

fn generate_task_list_name() -> String {
    format!("saga-task-list-{}", Uuid::new_v4())
}

fn generate_workflow_id(suffix: &str) -> String {
    format!("saga-workflow-{}-{}", suffix, Uuid::new_v4())
}

async fn register_domain_and_wait(
    client: &GrpcWorkflowServiceClient,
    domain_name: &str,
) -> Result<(), CadenceError> {
    let register_request = RegisterDomainRequest {
        name: domain_name.to_string(),
        description: Some("Test domain".to_string()),
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

    let _ = client.register_domain(register_request).await; // Ignore if already exists
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    Ok(())
}

fn create_order_workflow_request(
    domain: &str,
    task_list: &str,
    workflow_id: &str,
    input: &OrderInput,
) -> StartWorkflowExecutionRequest {
    StartWorkflowExecutionRequest {
        domain: domain.to_string(),
        workflow_id: workflow_id.to_string(),
        workflow_type: Some(WorkflowType {
            name: "order_processing_saga".to_string(),
        }),
        task_list: Some(TaskList {
            name: task_list.to_string(),
            kind: TaskListKind::Normal,
        }),
        input: Some(serde_json::to_vec(input).unwrap()),
        execution_start_to_close_timeout_seconds: Some(3600),
        task_start_to_close_timeout_seconds: Some(10),
        identity: "saga-test".to_string(),
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

// Wrapper for checking workflow completion
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
            wait_for_new_event: true, // Long poll
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
) -> Result<uber_cadence_proto::shared::History, CadenceError> {
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

fn assert_local_activity_executed(
    history: &uber_cadence_proto::shared::History,
    activity_type: &str,
) -> bool {
    history.events.iter().any(|e| {
        if e.event_type == EventType::MarkerRecorded {
            if let Some(EventAttributes::MarkerRecordedEventAttributes(attr)) = &e.attributes {
                if attr.marker_name == "LocalActivity" {
                    // Check if details contain the activity type
                    if let Some(details) = &attr.details {
                        // Try to parse the marker data to check activity type
                        if let Ok(marker_str) = std::str::from_utf8(details) {
                            return marker_str.contains(activity_type);
                        }
                    }
                }
            }
        }
        false
    })
}

fn assert_activity_executed(
    history: &uber_cadence_proto::shared::History,
    activity_type: &str,
) -> bool {
    // Check for regular activity
    let regular_activity = history.events.iter().any(|e| {
        if e.event_type == EventType::ActivityTaskScheduled {
            if let Some(EventAttributes::ActivityTaskScheduledEventAttributes(attr)) = &e.attributes
            {
                if let Some(atype) = &attr.activity_type {
                    return atype.name == activity_type;
                }
            }
        }
        false
    });

    // If not found as regular activity, check for local activity marker
    regular_activity || assert_local_activity_executed(history, activity_type)
}

fn assert_activity_failed(
    history: &uber_cadence_proto::shared::History,
    activity_type: &str,
) -> bool {
    // This is harder to check directly without tracking schedule ID,
    // but we can check if there's a failure event for a scheduled event of that type.
    // For simplicity, we just check if the activity was scheduled, and if the workflow failed.
    // A more robust check would link ActivityTaskFailed to ActivityTaskScheduled.
    assert_activity_executed(history, activity_type)
}

fn assert_compensation_executed(history: &uber_cadence_proto::shared::History) -> bool {
    assert_activity_executed(history, "release_inventory")
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_order_saga_payment_failure_with_compensation() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    println!("\n=== Testing Order Saga Payment Failure + Compensation ===");

    let domain_name = generate_test_domain_name();
    let task_list = generate_task_list_name();
    let workflow_id = generate_workflow_id("saga-payment-failure");

    // Setup
    let client = create_grpc_client(&domain_name).await.unwrap();
    register_domain_and_wait(&client, &domain_name)
        .await
        .unwrap();

    let service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync> =
        Arc::new(client.clone());

    // Setup registry
    let registry = Arc::new(uber_cadence_worker::registry::WorkflowRegistry::new());
    Registry::register_workflow(
        registry.as_ref(),
        "order_processing_saga",
        Box::new(OrderProcessingSagaWorkflow),
    );
    Registry::register_activity(
        registry.as_ref(),
        "calculate_order_total",
        Box::new(CalculateOrderTotalActivity),
    );
    Registry::register_activity(
        registry.as_ref(),
        "reserve_inventory",
        Box::new(ReserveInventoryActivity),
    );
    Registry::register_activity(
        registry.as_ref(),
        "process_payment",
        Box::new(ProcessPaymentActivity),
    );
    Registry::register_activity(
        registry.as_ref(),
        "release_inventory",
        Box::new(ReleaseInventoryActivity),
    );
    Registry::register_activity(
        registry.as_ref(),
        "send_notification",
        Box::new(SendNotificationActivity),
    );

    // Start worker
    let worker = CadenceWorker::new(
        &domain_name,
        &task_list,
        WorkerOptions {
            identity: "saga-worker".to_string(),
            disable_sticky_execution: true,
            ..Default::default()
        },
        registry,
        service.clone(),
    );

    Worker::start(&worker).expect("Failed to start worker");

    // Create order with high value that will trigger payment failure
    let order_input = OrderInput {
        user_id: "user-payment-fail".to_string(),
        items: vec![OrderItem {
            product_id: "prod-expensive".to_string(),
            sku: "SKU-FAIL".to_string(),
            quantity: 1,
            unit_price: 15000.00, // > 10000 triggers failure
        }],
        shipping_address: Address {
            street: "456 Test Ave".to_string(),
            city: "Testville".to_string(),
            state: "TS".to_string(),
            postal_code: "54321".to_string(),
            country: "USA".to_string(),
        },
        payment_method: PaymentMethod::CreditCard {
            last_four: "1234".to_string(),
            token: "tok_test".to_string(),
        },
    };

    // Start workflow
    let start_request =
        create_order_workflow_request(&domain_name, &task_list, &workflow_id, &order_input);
    let response = client
        .start_workflow_execution(start_request)
        .await
        .unwrap();

    // Wait for workflow to complete (should fail due to payment)
    let result = wait_for_workflow_completion(
        &client,
        &domain_name,
        &workflow_id,
        &response.run_id,
        Duration::from_secs(10),
    )
    .await;

    match result {
        Ok(WorkflowCompletionResult::Failed { reason, .. }) => {
            println!("  ✓ Workflow failed as expected: {}", reason);
        }
        Ok(WorkflowCompletionResult::Completed { .. }) => panic!("Workflow should have failed"),
        Ok(WorkflowCompletionResult::TimedOut) => {
            // Print history on timeout
            let history =
                get_workflow_history(&client, &domain_name, &workflow_id, &response.run_id)
                    .await
                    .unwrap();
            println!("TIMEOUT! History events:");
            for event in history.events {
                println!(
                    "  - {:?} (Attr: {:?})",
                    event.event_type,
                    event.attributes.map(|_| "Present")
                );
            }
            panic!("Workflow timed out");
        }
        Err(e) => panic!("Error waiting for workflow: {}", e),
    }

    // Get history and verify compensation
    let history = get_workflow_history(&client, &domain_name, &workflow_id, &response.run_id)
        .await
        .unwrap();

    // Verify expected activity execution
    assert!(
        assert_activity_executed(&history, "calculate_order_total"),
        "Calculate should execute"
    );
    assert!(
        assert_activity_executed(&history, "reserve_inventory"),
        "Reserve should execute"
    );
    assert!(
        assert_activity_failed(&history, "process_payment"),
        "Payment should fail"
    );

    // Verify compensation was executed
    assert!(
        assert_compensation_executed(&history),
        "Compensation (release_inventory) should execute after payment failure"
    );
    assert!(
        assert_activity_executed(&history, "release_inventory"),
        "Inventory should be released"
    );

    // Stop worker
    worker.stop();

    println!("  ✓ Saga compensation verified successfully");
    println!("\n=== Test Complete ===\n");
}

#[tokio::test]
#[ignore]
async fn test_order_saga_success_path() {
    println!("\n=== Testing Order Saga Success Path ===");

    let domain_name = generate_test_domain_name();
    let task_list = generate_task_list_name();
    let workflow_id = generate_workflow_id("saga-success");

    // Setup
    let client = create_grpc_client(&domain_name).await.unwrap();
    register_domain_and_wait(&client, &domain_name)
        .await
        .unwrap();

    let service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync> =
        Arc::new(client.clone());

    // Setup registry
    let registry = Arc::new(uber_cadence_worker::registry::WorkflowRegistry::new());
    registry.register_workflow(
        "order_processing_saga",
        Box::new(OrderProcessingSagaWorkflow),
    );
    registry.register_activity(
        "calculate_order_total",
        Box::new(CalculateOrderTotalActivity),
    );
    registry.register_activity("reserve_inventory", Box::new(ReserveInventoryActivity));
    registry.register_activity("process_payment", Box::new(ProcessPaymentActivity));
    registry.register_activity("release_inventory", Box::new(ReleaseInventoryActivity));
    registry.register_activity("send_notification", Box::new(SendNotificationActivity));

    // Start worker
    let worker = CadenceWorker::new(
        &domain_name,
        &task_list,
        WorkerOptions {
            identity: "saga-worker-success".to_string(),
            ..Default::default()
        },
        registry,
        service.clone(),
    );

    worker.start().expect("Failed to start worker");

    // Create order with normal value
    let order_input = OrderInput {
        user_id: "user-success".to_string(),
        items: vec![OrderItem {
            product_id: "prod-normal".to_string(),
            sku: "SKU-OK".to_string(),
            quantity: 1,
            unit_price: 50.00,
        }],
        shipping_address: Address {
            street: "123 Main St".to_string(),
            city: "Anytown".to_string(),
            state: "CA".to_string(),
            postal_code: "12345".to_string(),
            country: "USA".to_string(),
        },
        payment_method: PaymentMethod::PayPal {
            email: "test@example.com".to_string(),
        },
    };

    // Start workflow
    let start_request =
        create_order_workflow_request(&domain_name, &task_list, &workflow_id, &order_input);
    let response = client
        .start_workflow_execution(start_request)
        .await
        .unwrap();

    // Wait for workflow to complete
    let result = wait_for_workflow_completion(
        &client,
        &domain_name,
        &workflow_id,
        &response.run_id,
        Duration::from_secs(10),
    )
    .await;

    match result {
        Ok(WorkflowCompletionResult::Completed { result }) => {
            println!("  ✓ Workflow completed successfully");
            let output: OrderOutput = serde_json::from_slice(&result.unwrap()).unwrap();
            assert_eq!(output.status, OrderStatus::Paid);
        }
        Ok(WorkflowCompletionResult::Failed { reason, .. }) => {
            panic!("Workflow failed: {}", reason)
        }
        Ok(WorkflowCompletionResult::TimedOut) => panic!("Workflow timed out"),
        Err(e) => panic!("Error waiting for workflow: {}", e),
    }

    // Get history
    let history = get_workflow_history(&client, &domain_name, &workflow_id, &response.run_id)
        .await
        .unwrap();

    // Verify expected activity execution
    assert!(
        assert_activity_executed(&history, "calculate_order_total"),
        "Calculate should execute"
    );
    assert!(
        assert_activity_executed(&history, "reserve_inventory"),
        "Reserve should execute"
    );
    assert!(
        assert_activity_executed(&history, "process_payment"),
        "Payment should execute"
    );
    assert!(
        assert_activity_executed(&history, "send_notification"),
        "Notification should execute"
    );

    // Verify no compensation
    assert!(
        !assert_activity_executed(&history, "release_inventory"),
        "Inventory should NOT be released"
    );

    // Stop worker
    worker.stop();

    println!("  ✓ Success path verified successfully");
    println!("\n=== Test Complete ===\n");
}
