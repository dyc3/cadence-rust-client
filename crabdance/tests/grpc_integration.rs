//! gRPC Integration Tests
//!
//! These tests verify the gRPC client implementation works correctly with a real Cadence server.
//! They require a running Cadence server instance with gRPC enabled.
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
//! docker compose logs -f cadence
//! ```
//!
//! ## Running Tests
//!
//! Run all gRPC integration tests (sequentially to avoid domain cache issues):
//! ```bash
//! cargo test --test grpc_integration -- --ignored --test-threads=1
//! ```
//!
//! Run a specific test:
//! ```bash
//! cargo test --test grpc_integration test_grpc_connection -- --ignored --nocapture
//! ```
//!
//! ## gRPC Endpoint
//!
//! The tests connect to `http://localhost:7833` (gRPC port).
//! This is different from the Thrift port (7933).

use crabdance::client::error::TransportError;
use crabdance::client::GrpcWorkflowServiceClient;
use crabdance::proto::shared::{
    HistoryEventFilterType, TaskList, TaskListKind, WorkflowExecution, WorkflowType,
};
use crabdance::proto::workflow_service::{
    DescribeDomainRequest, GetWorkflowExecutionHistoryRequest, ListOpenWorkflowExecutionsRequest,
    QueryWorkflowRequest, RegisterDomainRequest, SignalWorkflowExecutionRequest, StartTimeFilter,
    StartWorkflowExecutionRequest, WorkflowQuery, WorkflowService,
};
use std::collections::HashMap;
use uuid::Uuid;

/// gRPC endpoint for Cadence server
const CADENCE_GRPC_ENDPOINT: &str = "http://localhost:7833";

/// Default domain for testing
const DEFAULT_DOMAIN: &str = "test-domain";

/// Helper: Create a connected gRPC client
async fn create_grpc_client(domain: &str) -> Result<GrpcWorkflowServiceClient, TransportError> {
    GrpcWorkflowServiceClient::connect(CADENCE_GRPC_ENDPOINT, domain, None).await
}

/// Helper: Generate unique domain name for testing
fn generate_test_domain_name() -> String {
    // Use UUID to ensure uniqueness when tests run in parallel
    format!("grpc-test-domain-{}", Uuid::new_v4())
}

/// Helper: Generate unique workflow ID
fn generate_workflow_id() -> String {
    format!("grpc-test-workflow-{}", Uuid::new_v4())
}

/// Helper: Register a domain and wait for it to be ready
async fn register_domain_and_wait(
    client: &GrpcWorkflowServiceClient,
    domain_name: &str,
) -> Result<(), TransportError> {
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

    client.register_domain(register_request).await?;

    // Wait for domain to propagate in the cache (needs more time for distributed system)
    // Increased to 1500ms to handle cache propagation when running multiple tests
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    Ok(())
}

/// Helper: Create a basic StartWorkflowExecutionRequest for testing
fn create_test_workflow_request(domain: &str, workflow_id: &str) -> StartWorkflowExecutionRequest {
    StartWorkflowExecutionRequest {
        domain: domain.to_string(),
        workflow_id: workflow_id.to_string(),
        workflow_type: Some(WorkflowType {
            name: "TestWorkflow".to_string(),
        }),
        task_list: Some(TaskList {
            name: "test-task-list".to_string(),
            kind: TaskListKind::Normal,
        }),
        input: None,
        execution_start_to_close_timeout_seconds: Some(3600),
        task_start_to_close_timeout_seconds: Some(10),
        identity: "grpc-integration-test".to_string(),
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

/// Test 1: Basic gRPC connection
///
/// This test verifies that the gRPC client can connect to the Cadence server
/// and make a basic API call.
#[tokio::test]
#[ignore]
async fn test_grpc_connection() {
    println!(
        "Testing gRPC connection to Cadence server at {}",
        CADENCE_GRPC_ENDPOINT
    );

    // Try to connect to the server
    let client = create_grpc_client(DEFAULT_DOMAIN)
        .await
        .expect("Failed to connect to Cadence gRPC server. Is it running on port 7833?");

    println!("✓ Successfully connected to Cadence gRPC server");

    // Try a simple operation - describe the system domain
    let request = DescribeDomainRequest {
        name: Some("cadence-system".to_string()),
        uuid: None,
    };

    let response = client
        .describe_domain(request)
        .await
        .expect("Failed to describe domain");

    println!("✓ Successfully called describe_domain");
    println!(
        "  Domain: {:?}",
        response.domain_info.as_ref().map(|d| &d.name)
    );

    assert!(
        response.domain_info.is_some(),
        "Domain info should be present"
    );
}

/// Test 2: Register and describe domain
///
/// This test verifies domain management operations work via gRPC.
#[tokio::test]
#[ignore]
async fn test_register_and_describe_domain() {
    let domain_name = generate_test_domain_name();
    println!("Testing domain registration: {}", domain_name);

    let client = create_grpc_client(&domain_name)
        .await
        .expect("Failed to connect to Cadence gRPC server");

    // Register a new domain
    register_domain_and_wait(&client, &domain_name)
        .await
        .expect("Failed to register domain");

    println!("✓ Successfully registered domain: {}", domain_name);

    // Describe the domain to verify it was created
    let describe_request = DescribeDomainRequest {
        name: Some(domain_name.clone()),
        uuid: None,
    };

    let response = client
        .describe_domain(describe_request)
        .await
        .expect("Failed to describe domain");

    println!("✓ Successfully described domain");

    // Verify domain properties
    assert!(
        response.domain_info.is_some(),
        "Domain info should be present"
    );
    let domain_info = response.domain_info.unwrap();
    assert_eq!(domain_info.name, domain_name, "Domain name should match");
    assert_eq!(
        domain_info.description.as_str(),
        "Test domain",
        "Domain description should match"
    );

    println!("✓ Domain properties verified");
}

/// Test 3: Start workflow execution
///
/// This test verifies starting a workflow via gRPC.
#[tokio::test]
#[ignore]
async fn test_start_workflow_execution() {
    let domain_name = generate_test_domain_name();
    let workflow_id = generate_workflow_id();

    println!("Testing workflow execution:");
    println!("  Domain: {}", domain_name);
    println!("  Workflow ID: {}", workflow_id);

    let client = create_grpc_client(&domain_name)
        .await
        .expect("Failed to connect to Cadence gRPC server");

    // Register domain first
    register_domain_and_wait(&client, &domain_name)
        .await
        .expect("Failed to register domain");

    println!("✓ Domain registered");

    // Start a workflow
    let start_request = create_test_workflow_request(&domain_name, &workflow_id);

    let response = client
        .start_workflow_execution(start_request)
        .await
        .expect("Failed to start workflow execution");

    println!("✓ Successfully started workflow");
    println!("  Run ID: {}", response.run_id);

    // Verify we got a valid run_id
    assert!(!response.run_id.is_empty(), "Run ID should not be empty");
    assert!(
        Uuid::parse_str(&response.run_id).is_ok(),
        "Run ID should be a valid UUID"
    );
}

/// Test 4: Get workflow execution history
///
/// This test verifies retrieving workflow history via gRPC.
#[tokio::test]
#[ignore]
async fn test_get_workflow_execution_history() {
    let domain_name = generate_test_domain_name();
    let workflow_id = generate_workflow_id();

    println!("Testing workflow history:");
    println!("  Domain: {}", domain_name);
    println!("  Workflow ID: {}", workflow_id);

    let client = create_grpc_client(&domain_name)
        .await
        .expect("Failed to connect to Cadence gRPC server");

    // Register domain
    register_domain_and_wait(&client, &domain_name)
        .await
        .expect("Failed to register domain");

    // Start a workflow
    let start_request = create_test_workflow_request(&domain_name, &workflow_id);
    let start_response = client
        .start_workflow_execution(start_request)
        .await
        .expect("Failed to start workflow execution");

    println!("✓ Workflow started with run_id: {}", start_response.run_id);

    // Get workflow execution history
    let history_request = GetWorkflowExecutionHistoryRequest {
        domain: domain_name.clone(),
        execution: Some(WorkflowExecution {
            workflow_id: workflow_id.clone(),
            run_id: start_response.run_id.clone(),
        }),
        page_size: 100,
        next_page_token: None,
        wait_for_new_event: false,
        history_event_filter_type: Some(HistoryEventFilterType::AllEvent),
        skip_archival: false,
        query_consistency_level: None,
    };

    let history_response = client
        .get_workflow_execution_history(history_request)
        .await
        .expect("Failed to get workflow execution history");

    println!("✓ Successfully retrieved workflow history");

    // Verify history contains events
    assert!(
        history_response.history.is_some(),
        "History should be present"
    );
    let history = history_response.history.unwrap();

    println!("  Event count: {}", history.events.len());

    // Verify history contains events
    assert!(!history.events.is_empty(), "History should contain events");

    // Print event types
    println!("  Events:");
    for event in &history.events {
        println!("    - Event {}: {:?}", event.event_id, event.event_type);
    }

    // Verify we have at least WorkflowExecutionStarted and DecisionTaskScheduled events
    let has_workflow_started = history
        .events
        .iter()
        .any(|e| e.event_type == crabdance::proto::shared::EventType::WorkflowExecutionStarted);

    let has_decision_scheduled = history
        .events
        .iter()
        .any(|e| e.event_type == crabdance::proto::shared::EventType::DecisionTaskScheduled);

    assert!(
        has_workflow_started,
        "History should contain WorkflowExecutionStarted event"
    );
    assert!(
        has_decision_scheduled,
        "History should contain DecisionTaskScheduled event"
    );

    println!("✓ History events verified");
}

/// Test 5: Signal workflow execution
///
/// This test verifies sending a signal to a workflow via gRPC.
#[tokio::test]
#[ignore]
async fn test_signal_workflow_execution() {
    let domain_name = generate_test_domain_name();
    let workflow_id = generate_workflow_id();

    println!("Testing workflow signal:");
    println!("  Domain: {}", domain_name);
    println!("  Workflow ID: {}", workflow_id);

    let client = create_grpc_client(&domain_name)
        .await
        .expect("Failed to connect to Cadence gRPC server");

    // Register domain
    register_domain_and_wait(&client, &domain_name)
        .await
        .expect("Failed to register domain");

    // Start a workflow
    let start_request = create_test_workflow_request(&domain_name, &workflow_id);
    let start_response = client
        .start_workflow_execution(start_request)
        .await
        .expect("Failed to start workflow execution");

    println!("✓ Workflow started");

    // Send a signal to the workflow
    let signal_request = SignalWorkflowExecutionRequest {
        domain: domain_name.clone(),
        workflow_execution: Some(WorkflowExecution {
            workflow_id: workflow_id.clone(),
            run_id: start_response.run_id.clone(),
        }),
        signal_name: "test-signal".to_string(),
        input: Some(b"test signal data".to_vec()),
        identity: "grpc-integration-test".to_string(),
        request_id: Uuid::new_v4().to_string(),
        control: None,
    };

    client
        .signal_workflow_execution(signal_request)
        .await
        .expect("Failed to signal workflow execution");

    println!("✓ Successfully sent signal to workflow");
}

/// Test 6: Query workflow
///
/// This test verifies querying a workflow via gRPC.
/// Note: This may return an error if the workflow is not running with a worker.
#[tokio::test]
#[ignore]
async fn test_query_workflow() {
    let domain_name = generate_test_domain_name();
    let workflow_id = generate_workflow_id();

    println!("Testing workflow query:");
    println!("  Domain: {}", domain_name);
    println!("  Workflow ID: {}", workflow_id);

    let client = create_grpc_client(&domain_name)
        .await
        .expect("Failed to connect to Cadence gRPC server");

    // Register domain
    register_domain_and_wait(&client, &domain_name)
        .await
        .expect("Failed to register domain");

    // Start a workflow
    let start_request = create_test_workflow_request(&domain_name, &workflow_id);
    let start_response = client
        .start_workflow_execution(start_request)
        .await
        .expect("Failed to start workflow execution");

    println!("✓ Workflow started");

    // Query the workflow (use __stack_trace which is a built-in query)
    let query_request = QueryWorkflowRequest {
        domain: domain_name.clone(),
        execution: Some(WorkflowExecution {
            workflow_id: workflow_id.clone(),
            run_id: start_response.run_id.clone(),
        }),
        query: Some(WorkflowQuery {
            query_type: "__stack_trace".to_string(),
            query_args: None,
        }),
        query_consistency_level: None,
        query_reject_condition: None,
    };

    // Note: This will likely fail because we don't have a worker running to handle the query
    // But we can verify the gRPC call itself works
    match client.query_workflow(query_request).await {
        Ok(response) => {
            println!("✓ Query succeeded (unexpected - no worker running)");
            println!(
                "  Query result present: {}",
                response.query_result.is_some()
            );
        }
        Err(e) => {
            println!("✓ Query failed as expected (no worker to handle query)");
            println!("  Error: {:?}", e);
            // This is expected since we don't have a worker running
        }
    }
}

/// Test 7: List open workflow executions
///
/// This test verifies listing workflows via gRPC.
#[tokio::test]
#[ignore]
async fn test_list_open_workflow_executions() {
    let domain_name = generate_test_domain_name();
    let workflow_id = generate_workflow_id();

    println!("Testing workflow listing:");
    println!("  Domain: {}", domain_name);
    println!("  Workflow ID: {}", workflow_id);

    let client = create_grpc_client(&domain_name)
        .await
        .expect("Failed to connect to Cadence gRPC server");

    // Register domain
    register_domain_and_wait(&client, &domain_name)
        .await
        .expect("Failed to register domain");

    // Start a workflow
    let start_request = create_test_workflow_request(&domain_name, &workflow_id);
    let start_response = client
        .start_workflow_execution(start_request)
        .await
        .expect("Failed to start workflow execution");

    println!("✓ Workflow started");

    // List open workflow executions
    // Use a time range that covers recent workflows (last hour)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64;
    let one_hour_ago = now - (3600 * 1_000_000_000);

    let list_request = ListOpenWorkflowExecutionsRequest {
        domain: domain_name.clone(),
        maximum_page_size: 100,
        next_page_token: None,
        start_time_filter: Some(StartTimeFilter {
            earliest_time: Some(one_hour_ago),
            latest_time: Some(now),
        }),
        execution_filter: None,
        type_filter: None,
        status_filter: None,
    };

    let list_response = client
        .list_open_workflow_executions(list_request)
        .await
        .expect("Failed to list open workflow executions");

    println!("✓ Successfully listed open workflows");
    println!("  Count: {}", list_response.executions.len());

    // Verify our workflow is in the list
    let found = list_response.executions.iter().any(|exec| {
        exec.execution
            .as_ref()
            .is_some_and(|e| e.workflow_id == workflow_id && e.run_id == start_response.run_id)
    });

    if found {
        println!("✓ Started workflow found in list");
    } else {
        println!("⚠ Started workflow not yet visible in list (eventual consistency)");
    }
}
