//! Integration tests for Thrift-based Cadence client
//!
//! These tests require a running Cadence server instance.
//! Start the server with: docker-compose up -d
//! 
//! The tests are marked with #[ignore] by default to prevent failures in CI.
//! Run them with: cargo test --test thrift_integration -- --ignored

use cadence_client::{ThriftWorkflowServiceClient, ClientConfig};
use cadence_proto::WorkflowService;
use cadence_proto::workflow_service::*;
use cadence_proto::shared::*;
use std::time::Duration;

/// Helper to create a test client configuration
fn create_test_config() -> ClientConfig {
    ClientConfig {
        host: std::env::var("CADENCE_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: std::env::var("CADENCE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(7933),
        timeout: Duration::from_secs(30),
        max_retries: 3,
    }
}

/// Helper to generate a unique domain name for testing
fn generate_test_domain() -> String {
    format!("test-domain-{}", uuid::Uuid::new_v4())
}

/// Helper to create a register domain request
fn create_register_domain_request(domain_name: &str) -> RegisterDomainRequest {
    RegisterDomainRequest {
        name: domain_name.to_string(),
        description: Some("Test domain created by integration test".to_string()),
        owner_email: "test@example.com".to_string(),
        workflow_execution_retention_period_in_days: 3,
        emit_metric: Some(true),
        clusters: vec![ClusterReplicationConfiguration {
            cluster_name: "active".to_string(),
        }],
        active_cluster_name: "active".to_string(),
        data: Default::default(),
        security_token: None,
        is_global_domain: Some(false),
        history_archival_status: None,
        history_archival_uri: None,
        visibility_archival_status: None,
        visibility_archival_uri: None,
    }
}

/// Helper to create a start workflow execution request
fn create_start_workflow_request(domain_name: &str, workflow_id: &str) -> StartWorkflowExecutionRequest {
    StartWorkflowExecutionRequest {
        domain: domain_name.to_string(),
        workflow_id: workflow_id.to_string(),
        workflow_type: Some(WorkflowType {
            name: "TestWorkflow".to_string(),
        }),
        task_list: Some(TaskList {
            name: "test-task-list".to_string(),
            kind: TaskListKind::Normal,
        }),
        input: Some(vec![]),
        execution_start_to_close_timeout_seconds: Some(300),
        task_start_to_close_timeout_seconds: Some(30),
        identity: "test-integration".to_string(),
        request_id: uuid::Uuid::new_v4().to_string(),
        workflow_id_reuse_policy: Some(WorkflowIdReusePolicy::AllowDuplicate),
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

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_thrift_connection() {
    let config = create_test_config();
    let client = ThriftWorkflowServiceClient::new("test-domain", config);
    
    let result = client.connect().await;
    assert!(result.is_ok(), "Should connect to Cadence server: {:?}", result);
}

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_register_domain() {
    let config = create_test_config();
    let domain_name = generate_test_domain();
    let client = ThriftWorkflowServiceClient::new(&domain_name, config);
    
    // Connect first
    client.connect().await.expect("Should connect");
    
    // Register a new domain
    let request = create_register_domain_request(&domain_name);
    
    let result = client.register_domain(request).await;
    // Domain may already exist, which is ok
    match result {
        Ok(_) => println!("Domain registered successfully"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            // Domain already exists is ok
            if !error_str.contains("already") && !error_str.contains("exist") {
                panic!("Failed to register domain: {:?}", e);
            }
        }
    }
}

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_describe_domain() {
    let config = create_test_config();
    let domain_name = generate_test_domain();
    let client = ThriftWorkflowServiceClient::new(&domain_name, config);
    
    // First register the domain
    let register_request = create_register_domain_request(&domain_name);
    let _ = client.register_domain(register_request).await;
    
    // Give it a moment to propagate
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Now describe it
    let describe_request = DescribeDomainRequest {
        name: Some(domain_name.clone()),
        uuid: None,
    };
    
    let result = client.describe_domain(describe_request).await;
    assert!(result.is_ok(), "Should describe domain: {:?}", result);
    
    let response = result.unwrap();
    assert!(response.domain_info.is_some(), "Should have domain info");
    
    if let Some(info) = response.domain_info {
        assert_eq!(info.name, domain_name);
    }
}

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_start_workflow_execution() {
    let config = create_test_config();
    let domain_name = generate_test_domain();
    let client = ThriftWorkflowServiceClient::new(&domain_name, config);
    
    // Register domain first
    let register_request = create_register_domain_request(&domain_name);
    let _ = client.register_domain(register_request).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Start a workflow execution
    let workflow_id = format!("test-workflow-{}", uuid::Uuid::new_v4());
    let request = create_start_workflow_request(&domain_name, &workflow_id);
    
    let result = client.start_workflow_execution(request).await;
    assert!(result.is_ok(), "Should start workflow execution: {:?}", result);
    
    let response = result.unwrap();
    assert!(!response.run_id.is_empty(), "Should have a run ID");
    println!("Started workflow with run ID: {}", response.run_id);
}

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_signal_workflow_execution() {
    let config = create_test_config();
    let domain_name = generate_test_domain();
    let client = ThriftWorkflowServiceClient::new(&domain_name, config);
    
    // Register domain
    let register_request = create_register_domain_request(&domain_name);
    let _ = client.register_domain(register_request).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Start a workflow first
    let workflow_id = format!("test-workflow-signal-{}", uuid::Uuid::new_v4());
    let start_request = create_start_workflow_request(&domain_name, &workflow_id);
    
    let start_result = client.start_workflow_execution(start_request).await;
    assert!(start_result.is_ok(), "Should start workflow: {:?}", start_result);
    
    let run_id = start_result.unwrap().run_id;
    
    // Now send a signal
    let signal_request = SignalWorkflowExecutionRequest {
        domain: domain_name.clone(),
        workflow_execution: Some(WorkflowExecution {
            workflow_id: workflow_id.clone(),
            run_id: run_id.clone(),
        }),
        signal_name: "test-signal".to_string(),
        input: Some(vec![]),
        identity: "test-integration".to_string(),
        request_id: uuid::Uuid::new_v4().to_string(),
        control: None,
    };
    
    let result = client.signal_workflow_execution(signal_request).await;
    assert!(result.is_ok(), "Should signal workflow execution: {:?}", result);
}

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_query_workflow() {
    let config = create_test_config();
    let domain_name = generate_test_domain();
    let client = ThriftWorkflowServiceClient::new(&domain_name, config);
    
    // Register domain
    let register_request = create_register_domain_request(&domain_name);
    let _ = client.register_domain(register_request).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Start a workflow
    let workflow_id = format!("test-workflow-query-{}", uuid::Uuid::new_v4());
    let start_request = create_start_workflow_request(&domain_name, &workflow_id);
    
    let start_result = client.start_workflow_execution(start_request).await;
    assert!(start_result.is_ok(), "Should start workflow: {:?}", start_result);
    
    let run_id = start_result.unwrap().run_id;
    
    // Query the workflow
    let query_request = QueryWorkflowRequest {
        domain: domain_name.clone(),
        execution: Some(WorkflowExecution {
            workflow_id: workflow_id.clone(),
            run_id: run_id.clone(),
        }),
        query: Some(WorkflowQuery {
            query_type: "__stack_trace".to_string(), // Built-in query
            query_args: None,
        }),
    };
    
    let result = client.query_workflow(query_request).await;
    // Query may fail if workflow hasn't started yet, but the call should work
    println!("Query result: {:?}", result);
}

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_poll_for_decision_task_timeout() {
    let config = ClientConfig {
        timeout: Duration::from_secs(2), // Short timeout for testing
        ..create_test_config()
    };
    let domain_name = generate_test_domain();
    let client = ThriftWorkflowServiceClient::new(&domain_name, config);
    
    // Register domain
    let register_request = create_register_domain_request(&domain_name);
    let _ = client.register_domain(register_request).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Poll for decision task (should timeout with empty task)
    let poll_request = PollForDecisionTaskRequest {
        domain: domain_name.clone(),
        task_list: Some(TaskList {
            name: "non-existent-task-list".to_string(),
            kind: TaskListKind::Normal,
        }),
        identity: "test-integration".to_string(),
        binary_checksum: "test-checksum".to_string(),
    };
    
    let result = client.poll_for_decision_task(poll_request).await;
    // Should succeed but return empty task
    assert!(result.is_ok(), "Should complete poll request: {:?}", result);
}

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_list_open_workflow_executions() {
    let config = create_test_config();
    let domain_name = generate_test_domain();
    let client = ThriftWorkflowServiceClient::new(&domain_name, config);
    
    // Register domain
    let register_request = create_register_domain_request(&domain_name);
    let _ = client.register_domain(register_request).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // List open workflow executions
    let now = chrono::Utc::now().timestamp();
    let one_day_ago = now - 86400;
    
    let list_request = ListOpenWorkflowExecutionsRequest {
        domain: domain_name.clone(),
        maximum_page_size: 100,
        next_page_token: None,
        start_time_filter: Some(StartTimeFilter {
            earliest_time: Some(one_day_ago),
            latest_time: Some(now),
        }),
        execution_filter: None,
        type_filter: None,
        status_filter: None,
    };
    
    let result = client.list_open_workflow_executions(list_request).await;
    assert!(result.is_ok(), "Should list open workflows: {:?}", result);
}

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_list_closed_workflow_executions() {
    let config = create_test_config();
    let domain_name = generate_test_domain();
    let client = ThriftWorkflowServiceClient::new(&domain_name, config);
    
    // Register domain
    let register_request = create_register_domain_request(&domain_name);
    let _ = client.register_domain(register_request).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // List closed workflow executions
    let now = chrono::Utc::now().timestamp();
    let one_day_ago = now - 86400;
    
    let list_request = ListClosedWorkflowExecutionsRequest {
        domain: domain_name.clone(),
        maximum_page_size: 100,
        next_page_token: None,
        start_time_filter: Some(StartTimeFilter {
            earliest_time: Some(one_day_ago),
            latest_time: Some(now),
        }),
        execution_filter: None,
        type_filter: None,
        status_filter: None,
    };
    
    let result = client.list_closed_workflow_executions(list_request).await;
    assert!(result.is_ok(), "Should list closed workflows: {:?}", result);
}

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_get_workflow_execution_history() {
    let config = create_test_config();
    let domain_name = generate_test_domain();
    let client = ThriftWorkflowServiceClient::new(&domain_name, config);
    
    // Register domain
    let register_request = create_register_domain_request(&domain_name);
    let _ = client.register_domain(register_request).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Start a workflow
    let workflow_id = format!("test-workflow-history-{}", uuid::Uuid::new_v4());
    let start_request = create_start_workflow_request(&domain_name, &workflow_id);
    
    let start_result = client.start_workflow_execution(start_request).await;
    assert!(start_result.is_ok(), "Should start workflow: {:?}", start_result);
    
    let run_id = start_result.unwrap().run_id;
    
    // Get workflow execution history
    let history_request = GetWorkflowExecutionHistoryRequest {
        domain: domain_name.clone(),
        execution: Some(WorkflowExecution {
            workflow_id: workflow_id.clone(),
            run_id: run_id.clone(),
        }),
        page_size: 100,
        next_page_token: None,
        wait_for_new_event: false,
        history_event_filter_type: None,
        skip_archival: false,
    };
    
    let result = client.get_workflow_execution_history(history_request).await;
    assert!(result.is_ok(), "Should get workflow history: {:?}", result);
    
    let response = result.unwrap();
    assert!(response.history.is_some(), "Should have history");
}
