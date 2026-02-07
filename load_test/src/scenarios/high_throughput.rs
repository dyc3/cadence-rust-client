//! High throughput scenario - spawn workflows as fast as possible up to a target rate

use std::time::Duration;

use anyhow::Result;
use tokio::time::{interval, Instant};
use uber_cadence_client::GrpcWorkflowServiceClient;
use uber_cadence_proto::shared::{TaskList, TaskListKind, WorkflowType};
use uber_cadence_proto::workflow_service::{StartWorkflowExecutionRequest, WorkflowService};
use uuid::Uuid;

use crate::cli::{ClientArgs, HighThroughputArgs};
use crate::metrics::collector::MetricsCollector;
use crate::metrics::reporter;
use crate::workflows::cpu_bound::CpuWorkflowInput;
use crate::workflows::failing::FailingWorkflowInput;
use crate::workflows::io_bound::IoWorkflowInput;
use crate::workflows::noop::NoopInput;

/// Helper function to create workflow input based on type
fn create_workflow_input(
    workflow_type: &str,
    id: usize,
    args: &HighThroughputArgs,
) -> Result<Vec<u8>> {
    let input_bytes = match workflow_type {
        "noop_workflow" => {
            let input = NoopInput { id };
            serde_json::to_vec(&input)?
        }
        "cpu_bound_workflow" => {
            let input = CpuWorkflowInput {
                id,
                iterations: args.cpu_iterations,
            };
            serde_json::to_vec(&input)?
        }
        "io_bound_workflow" => {
            let input = IoWorkflowInput {
                id,
                delay_ms: args.io_delay_ms,
            };
            serde_json::to_vec(&input)?
        }
        "failing_workflow" => {
            let input = FailingWorkflowInput {
                id,
                failure_rate: args.failure_rate,
                max_retries: args.max_retries,
            };
            serde_json::to_vec(&input)?
        }
        _ => anyhow::bail!("Unknown workflow type: {}", workflow_type),
    };
    Ok(input_bytes)
}

pub async fn run(client_args: ClientArgs, args: HighThroughputArgs) -> Result<()> {
    tracing::info!("Starting high throughput scenario");

    let domain = &client_args.domain;
    let task_list = &client_args.task_list;
    let endpoint = &client_args.endpoint;

    // Create client only (no worker)
    let client = GrpcWorkflowServiceClient::connect(endpoint, domain, None).await?;

    tracing::info!("Client connected, waiting for warmup period");

    // Warmup period
    if args.warmup > 0 {
        tokio::time::sleep(Duration::from_secs(args.warmup)).await;
    }

    // Setup metrics collector
    let collector = MetricsCollector::new();
    let collector_clone = collector.clone();

    // Start periodic metrics reporter
    tokio::spawn(async move {
        reporter::start_periodic_reporter(collector_clone, 2).await;
    });

    // Run load test
    let start_time = Instant::now();
    let duration = Duration::from_secs(client_args.duration);
    let target_rate = args.target_rate;

    // Use timestamp to ensure workflow IDs are unique across runs
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    tracing::info!(
        "Starting workflow spawning at {} workflows/sec",
        target_rate
    );

    let mut ticker = interval(Duration::from_secs(1));
    let mut workflow_counter = 0usize;
    let mut workflow_handles = Vec::new();

    loop {
        ticker.tick().await;

        // Check if we've exceeded duration
        if start_time.elapsed() >= duration {
            break;
        }

        // Spawn target_rate workflows this second
        for _ in 0..(target_rate as usize) {
            workflow_counter += 1;
            let workflow_id = format!("load-test-ht-{}-{}", run_id, workflow_counter);
            let input_bytes = create_workflow_input(&args.workflow_type, workflow_counter, &args)?;

            // Record workflow start
            collector.workflow_started();
            let start = Instant::now();

            // Start workflow and track the handle
            let client_clone = client.clone();
            let domain_clone = domain.clone();
            let task_list_clone = task_list.clone();
            let collector_clone = collector.clone();
            let workflow_type_clone = args.workflow_type.clone();

            let handle = tokio::spawn(async move {
                let start_request = StartWorkflowExecutionRequest {
                    domain: domain_clone.clone(),
                    workflow_id: workflow_id.clone(),
                    workflow_type: Some(WorkflowType {
                        name: workflow_type_clone,
                    }),
                    task_list: Some(TaskList {
                        name: task_list_clone.clone(),
                        kind: TaskListKind::Normal,
                    }),
                    input: Some(input_bytes.clone()),
                    execution_start_to_close_timeout_seconds: Some(60),
                    task_start_to_close_timeout_seconds: Some(10),
                    identity: "load-test".to_string(),
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

                match client_clone.start_workflow_execution(start_request).await {
                    Ok(_) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        collector_clone.workflow_completed(duration_ms);
                    }
                    Err(e) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        tracing::error!("Failed to start workflow {}: {}", workflow_id, e);
                        collector_clone.workflow_failed(duration_ms);
                    }
                }
            });

            workflow_handles.push(handle);
        }
    }

    tracing::info!(
        "Load test duration completed, waiting for {} in-flight workflows...",
        workflow_handles.len()
    );

    // Wait for all spawned workflows to complete
    for (idx, handle) in workflow_handles.into_iter().enumerate() {
        if let Err(e) = handle.await {
            tracing::error!("Workflow task {} panicked: {}", idx, e);
        }

        // Log progress every 100 workflows
        if (idx + 1) % 100 == 0 {
            tracing::info!(
                "Waited for {}/{} workflows to complete",
                idx + 1,
                workflow_counter
            );
        }
    }

    tracing::info!("All workflows completed");

    // Print final report
    reporter::print_final_report(&collector);

    Ok(())
}
