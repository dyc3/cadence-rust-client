//! Channels and Spawn Integration Test
//!
//! This test verifies that workflow channels and spawn work correctly with a real Cadence server.
//! It requires a running Cadence server instance with gRPC enabled.
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
//! Run the channels and spawn integration test:
//! ```bash
//! cargo test --test channels_spawn_integration -- --ignored --nocapture
//! ```
//!
//! ## gRPC Endpoint
//!
//! The test connects to `http://localhost:7833` (gRPC port).

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
use uber_cadence_worker::registry::{
    Activity, ActivityError, Registry, Workflow, WorkflowError, WorkflowRegistry,
};
use uber_cadence_worker::{CadenceWorker, Worker, WorkerOptions};
use uber_cadence_workflow::WorkflowContext;
use uuid::Uuid;

/// gRPC endpoint for Cadence server
const CADENCE_GRPC_ENDPOINT: &str = "http://localhost:7833";

/// Task list for workers (use a fixed name, not generated)
const TASK_LIST: &str = "channels-spawn-task-list";

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
    id: u32,
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobResult {
    job_id: u32,
    result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SplitMergeInput {
    jobs: Vec<Job>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SplitMergeOutput {
    results: Vec<JobResult>,
}

// ============================================================================
// Activities
// ============================================================================

#[derive(Clone)]
struct ProcessJobActivity;

impl Activity for ProcessJobActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move {
            let input_bytes =
                input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
            let job: Job = serde_json::from_slice(&input_bytes).map_err(|e| {
                ActivityError::ExecutionFailed(format!("Failed to deserialize job: {}", e))
            })?;

            tracing::info!("Processing job {}: {}", job.id, job.data);

            // Simulate some processing - now we can use async/await!
            // In a real implementation, this could be an async API call

            let result = JobResult {
                job_id: job.id,
                result: format!("Processed: {}", job.data),
            };

            serde_json::to_vec(&result).map_err(|e| {
                ActivityError::ExecutionFailed(format!("Failed to serialize result: {}", e))
            })
        })
    }
}

#[derive(Clone)]
struct FastProcessActivity;

impl Activity for FastProcessActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move {
            let input_bytes =
                input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
            let value: u32 = serde_json::from_slice(&input_bytes).map_err(|e| {
                ActivityError::ExecutionFailed(format!("Failed to deserialize value: {}", e))
            })?;

            tracing::info!("Fast processing value: {}", value);

            let result = value * 2;

            serde_json::to_vec(&result).map_err(|e| {
                ActivityError::ExecutionFailed(format!("Failed to serialize result: {}", e))
            })
        })
    }
}

// ============================================================================
// Workflow Implementations
// ============================================================================
// Workflows
// ============================================================================

#[derive(Clone)]
struct SplitMergeWorkflow;

impl Workflow for SplitMergeWorkflow {
    fn execute(
        &self,
        ctx: WorkflowContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async move {
            let input_bytes =
                input.ok_or_else(|| WorkflowError::ExecutionFailed("Missing input".to_string()))?;
            let input_data: SplitMergeInput =
                serde_json::from_slice(&input_bytes).map_err(|e| {
                    WorkflowError::ExecutionFailed(format!("Failed to deserialize input: {}", e))
                })?;

            let job_count = input_data.jobs.len();
            tracing::info!("Starting split-merge workflow with {} jobs", job_count);

            // Create a channel to collect results
            let (tx, rx) = ctx.new_channel(job_count);

            // Fan-out: spawn a task for each job
            for job in input_data.jobs {
                let ctx_clone = ctx.clone();
                let tx_clone = tx.clone();

                ctx.spawn(async move {
                    tracing::info!("Spawned task for job {}", job.id);

                    // Execute activity
                    let options = ActivityOptions {
                        task_list: TASK_LIST.to_string(),
                        schedule_to_close_timeout: Duration::from_secs(60),
                        schedule_to_start_timeout: Duration::from_secs(10),
                        start_to_close_timeout: Duration::from_secs(30),
                        heartbeat_timeout: Duration::from_secs(10),
                        retry_policy: None,
                        local_activity: false,
                        wait_for_cancellation: false,
                    };

                    let job_bytes = serde_json::to_vec(&job).map_err(|e| {
                        uber_cadence_workflow::WorkflowError::Generic(e.to_string())
                    })?;
                    let result_bytes = ctx_clone
                        .execute_activity("process_job", Some(job_bytes), options)
                        .await?;

                    let result: JobResult = serde_json::from_slice(&result_bytes).map_err(|e| {
                        uber_cadence_workflow::WorkflowError::Generic(e.to_string())
                    })?;
                    tracing::info!("Job {} completed: {:?}", job.id, result);

                    // Send result through channel
                    let _ = tx_clone.send(result).await;

                    Ok::<_, uber_cadence_workflow::WorkflowError>(())
                });
            }

            // Drop the original sender so the channel closes when all spawned tasks are done
            drop(tx);

            // Fan-in: collect all results from the channel
            let mut results = Vec::new();
            while let Some(result) = rx.recv().await {
                tracing::info!("Received result for job {}", result.job_id);
                results.push(result);
            }

            tracing::info!(
                "Split-merge workflow completed with {} results",
                results.len()
            );

            let output = SplitMergeOutput { results };
            serde_json::to_vec(&output).map_err(|e| {
                WorkflowError::ExecutionFailed(format!("Failed to serialize output: {}", e))
            })
        })
    }
}

#[derive(Clone)]
struct ParallelWorkflow;

impl Workflow for ParallelWorkflow {
    fn execute(
        &self,
        ctx: WorkflowContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async move {
            let input_bytes =
                input.ok_or_else(|| WorkflowError::ExecutionFailed("Missing input".to_string()))?;
            let values: Vec<u32> = serde_json::from_slice(&input_bytes).map_err(|e| {
                WorkflowError::ExecutionFailed(format!("Failed to deserialize input: {}", e))
            })?;

            tracing::info!("Starting parallel workflow with {} values", values.len());

            // Spawn tasks for each value
            let mut handles = Vec::new();
            for value in values {
                let ctx_clone = ctx.clone();

                let handle = ctx.spawn(async move {
                    let options = ActivityOptions {
                        task_list: TASK_LIST.to_string(),
                        schedule_to_close_timeout: Duration::from_secs(60),
                        schedule_to_start_timeout: Duration::from_secs(10),
                        start_to_close_timeout: Duration::from_secs(30),
                        heartbeat_timeout: Duration::from_secs(10),
                        retry_policy: None,
                        local_activity: false,
                        wait_for_cancellation: false,
                    };

                    let value_bytes = serde_json::to_vec(&value).map_err(|e| {
                        uber_cadence_workflow::WorkflowError::Generic(e.to_string())
                    })?;
                    match ctx_clone
                        .execute_activity("fast_process", Some(value_bytes), options)
                        .await
                    {
                        Ok(result_bytes) => {
                            let result: u32 =
                                serde_json::from_slice(&result_bytes).map_err(|e| {
                                    uber_cadence_workflow::WorkflowError::Generic(e.to_string())
                                })?;
                            Ok(result)
                        }
                        Err(e) => Err(e),
                    }
                });

                handles.push(handle);
            }

            // Wait for all tasks to complete
            let mut results = Vec::new();
            for handle in handles {
                let value = handle
                    .join()
                    .await
                    .map_err(|e| WorkflowError::ExecutionFailed(e.to_string()))?;
                results.push(value.map_err(|e| WorkflowError::Generic(format!("{:?}", e)))?);
            }

            tracing::info!("Parallel workflow completed with {} results", results.len());

            serde_json::to_vec(&results).map_err(|e| {
                WorkflowError::ExecutionFailed(format!("Failed to serialize output: {}", e))
            })
        })
    }
}

// ============================================================================
// Helpers
// ============================================================================

async fn create_grpc_client(domain: &str) -> Result<GrpcWorkflowServiceClient, CadenceError> {
    GrpcWorkflowServiceClient::connect(CADENCE_GRPC_ENDPOINT, domain, None).await
}

fn generate_test_domain_name() -> String {
    format!("channels-spawn-test-{}", Uuid::new_v4())
}

#[expect(dead_code)]
fn generate_task_list_name() -> String {
    format!("channels-spawn-task-list-{}", Uuid::new_v4())
}

fn generate_workflow_id(suffix: &str) -> String {
    format!("channels-spawn-workflow-{}-{}", suffix, Uuid::new_v4())
}

async fn register_domain_and_wait(
    client: &GrpcWorkflowServiceClient,
    domain_name: &str,
) -> Result<(), CadenceError> {
    let register_request = RegisterDomainRequest {
        name: domain_name.to_string(),
        description: Some("Test domain for channels and spawn".to_string()),
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

    // Try to register domain, ignore if it already exists
    match client.register_domain(register_request).await {
        Ok(_) => {
            tracing::info!("Registered domain: {}", domain_name);
            // Wait for domain to propagate
            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        }
        Err(e) => {
            tracing::warn!("Domain registration error (may already exist): {:?}", e);
        }
    }

    Ok(())
}

fn create_start_workflow_request(
    domain: &str,
    task_list: &str,
    workflow_id: &str,
    workflow_type: &str,
    input: &impl Serialize,
) -> StartWorkflowExecutionRequest {
    StartWorkflowExecutionRequest {
        domain: domain.to_string(),
        workflow_id: workflow_id.to_string(),
        workflow_type: Some(WorkflowType {
            name: workflow_type.to_string(),
        }),
        task_list: Some(TaskList {
            name: task_list.to_string(),
            kind: TaskListKind::Normal,
        }),
        input: Some(serde_json::to_vec(input).unwrap()),
        execution_start_to_close_timeout_seconds: Some(120),
        task_start_to_close_timeout_seconds: Some(10),
        identity: "channels-spawn-test".to_string(),
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
enum WorkflowCompletionResult {
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
                            EventType::WorkflowExecutionTimedOut => {
                                return Ok(WorkflowCompletionResult::TimedOut);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Error fetching workflow history: {:?}", e);
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_split_merge_workflow_with_channels_and_spawn() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,uber_cadence_worker=debug,uber_cadence_workflow=debug")
        .with_test_writer()
        .try_init()
        .ok();

    tracing::info!("=== Starting split-merge workflow test with channels and spawn ===");

    // Generate unique names
    let domain = generate_test_domain_name();
    let task_list = TASK_LIST; // Use fixed task list name
    let workflow_id = generate_workflow_id("split-merge");

    tracing::info!("Domain: {}", domain);
    tracing::info!("Task list: {}", task_list);
    tracing::info!("Workflow ID: {}", workflow_id);

    // Create client and register domain
    let client = create_grpc_client(&domain)
        .await
        .expect("Failed to create gRPC client");

    register_domain_and_wait(&client, &domain)
        .await
        .expect("Failed to register domain");

    // Create registry
    let registry: Arc<dyn Registry> = Arc::new(WorkflowRegistry::new());
    Registry::register_workflow(
        registry.as_ref(),
        "split_merge_workflow",
        Box::new(SplitMergeWorkflow),
    );
    Registry::register_activity(
        registry.as_ref(),
        "process_job",
        Box::new(ProcessJobActivity),
    );

    // Create service Arc
    let service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync> =
        Arc::new(client.clone());

    // Create and start worker
    let worker = CadenceWorker::new(
        domain.clone(),
        task_list.to_string(),
        WorkerOptions::default(),
        registry,
        service,
    );

    let worker_handle = tokio::spawn(async move {
        let _ = worker.start();
    });

    // Give worker time to start polling
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Start workflow
    let input = SplitMergeInput {
        jobs: vec![
            Job {
                id: 1,
                data: "First job".to_string(),
            },
            Job {
                id: 2,
                data: "Second job".to_string(),
            },
            Job {
                id: 3,
                data: "Third job".to_string(),
            },
        ],
    };

    let start_request = create_start_workflow_request(
        &domain,
        task_list,
        &workflow_id,
        "split_merge_workflow",
        &input,
    );

    tracing::info!("Starting workflow...");
    let start_response = client
        .start_workflow_execution(start_request)
        .await
        .expect("Failed to start workflow");

    tracing::info!("Workflow started with run_id: {}", start_response.run_id);

    // Wait for workflow to complete
    tracing::info!("Waiting for workflow to complete...");
    let completion_result = wait_for_workflow_completion(
        &client,
        &domain,
        &workflow_id,
        &start_response.run_id,
        Duration::from_secs(60),
    )
    .await
    .expect("Failed to wait for workflow completion");

    match completion_result {
        WorkflowCompletionResult::Completed { result } => {
            tracing::info!("Workflow completed successfully!");

            let result_bytes = result.expect("Expected workflow result");
            let output: SplitMergeOutput = serde_json::from_slice(&result_bytes)
                .expect("Failed to deserialize workflow output");

            tracing::info!("Workflow output: {:?}", output);

            // Verify results
            assert_eq!(output.results.len(), 3, "Should have 3 results");

            // Check that all job IDs are present
            let job_ids: Vec<u32> = output.results.iter().map(|r| r.job_id).collect();
            assert!(job_ids.contains(&1), "Should have result for job 1");
            assert!(job_ids.contains(&2), "Should have result for job 2");
            assert!(job_ids.contains(&3), "Should have result for job 3");

            // Check that all results contain "Processed:"
            for result in &output.results {
                assert!(
                    result.result.starts_with("Processed:"),
                    "Result should start with 'Processed:', got: {}",
                    result.result
                );
            }

            tracing::info!("✓ Split-merge workflow test passed!");
        }
        WorkflowCompletionResult::Failed { reason, details } => {
            panic!(
                "Workflow failed: {}, details: {:?}",
                reason,
                details.map(|d| String::from_utf8_lossy(&d).to_string())
            );
        }
        WorkflowCompletionResult::TimedOut => {
            panic!("Workflow timed out");
        }
    }

    // Stop worker
    worker_handle.abort();
}

#[tokio::test]
#[ignore] // Requires running Cadence server
async fn test_parallel_workflow_with_spawn() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,uber_cadence_worker=debug,uber_cadence_workflow=debug")
        .with_test_writer()
        .try_init()
        .ok();

    tracing::info!("=== Starting parallel workflow test with spawn ===");

    // Generate unique names
    let domain = generate_test_domain_name();
    let task_list = TASK_LIST; // Use fixed task list name
    let workflow_id = generate_workflow_id("parallel");

    tracing::info!("Domain: {}", domain);
    tracing::info!("Task list: {}", task_list);
    tracing::info!("Workflow ID: {}", workflow_id);

    // Create client and register domain
    let client = create_grpc_client(&domain)
        .await
        .expect("Failed to create gRPC client");

    register_domain_and_wait(&client, &domain)
        .await
        .expect("Failed to register domain");

    // Create registry
    let registry: Arc<dyn Registry> = Arc::new(WorkflowRegistry::new());
    Registry::register_workflow(
        registry.as_ref(),
        "parallel_workflow",
        Box::new(ParallelWorkflow),
    );
    Registry::register_activity(
        registry.as_ref(),
        "fast_process",
        Box::new(FastProcessActivity),
    );

    // Create service Arc
    let service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync> =
        Arc::new(client.clone());

    // Create and start worker
    let worker = CadenceWorker::new(
        domain.clone(),
        task_list.to_string(),
        WorkerOptions::default(),
        registry,
        service,
    );

    let worker_handle = tokio::spawn(async move {
        let _ = worker.start();
    });

    // Give worker time to start polling
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Start workflow
    let input = vec![1u32, 2, 3, 4, 5];

    let start_request = create_start_workflow_request(
        &domain,
        task_list,
        &workflow_id,
        "parallel_workflow",
        &input,
    );

    tracing::info!("Starting workflow...");
    let start_response = client
        .start_workflow_execution(start_request)
        .await
        .expect("Failed to start workflow");

    tracing::info!("Workflow started with run_id: {}", start_response.run_id);

    // Wait for workflow to complete
    tracing::info!("Waiting for workflow to complete...");
    let completion_result = wait_for_workflow_completion(
        &client,
        &domain,
        &workflow_id,
        &start_response.run_id,
        Duration::from_secs(60),
    )
    .await
    .expect("Failed to wait for workflow completion");

    match completion_result {
        WorkflowCompletionResult::Completed { result } => {
            tracing::info!("Workflow completed successfully!");

            let result_bytes = result.expect("Expected workflow result");
            let output: Vec<u32> = serde_json::from_slice(&result_bytes)
                .expect("Failed to deserialize workflow output");

            tracing::info!("Workflow output: {:?}", output);

            // Verify results (should be doubled)
            assert_eq!(output.len(), 5, "Should have 5 results");
            assert_eq!(output, vec![2, 4, 6, 8, 10], "Values should be doubled");

            tracing::info!("✓ Parallel workflow test passed!");
        }
        WorkflowCompletionResult::Failed { reason, details } => {
            panic!(
                "Workflow failed: {}, details: {:?}",
                reason,
                details.map(|d| String::from_utf8_lossy(&d).to_string())
            );
        }
        WorkflowCompletionResult::TimedOut => {
            panic!("Workflow timed out");
        }
    }

    // Stop worker
    worker_handle.abort();
}
