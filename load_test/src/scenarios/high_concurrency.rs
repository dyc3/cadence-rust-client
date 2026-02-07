//! High concurrency scenario - maintain N concurrent workflows

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::time::{interval, Instant};
use uber_cadence_client::GrpcWorkflowServiceClient;
use uber_cadence_proto::shared::{TaskList, TaskListKind, WorkflowType};
use uber_cadence_proto::workflow_service::{StartWorkflowExecutionRequest, WorkflowService};
use uuid::Uuid;

use crate::cli::{ClientArgs, HighConcurrencyArgs};
use crate::metrics::collector::MetricsCollector;
use crate::metrics::reporter;
use crate::workflows::io_bound::IoWorkflowInput;

pub async fn run(client_args: ClientArgs, args: HighConcurrencyArgs) -> Result<()> {
    tracing::info!("Starting high concurrency scenario");

    let domain = &client_args.domain;
    let task_list = &client_args.task_list;
    let endpoint = &client_args.endpoint;

    // Create client only (no worker)
    let client = GrpcWorkflowServiceClient::connect(endpoint, domain, None).await?;

    tracing::info!("Client connected, waiting for warmup period");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Setup metrics collector
    let collector = MetricsCollector::new();
    let collector_clone = collector.clone();

    // Start periodic metrics reporter
    tokio::spawn(async move {
        reporter::start_periodic_reporter(collector_clone, 2).await;
    });

    // Track in-flight workflows
    let in_flight = Arc::new(AtomicUsize::new(0));

    // Run load test
    let start_time = Instant::now();
    let duration = Duration::from_secs(client_args.duration);
    let target_count = args.target_count;

    // Use timestamp to ensure workflow IDs are unique across runs
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    tracing::info!("Maintaining {} concurrent workflows", target_count);

    let mut workflow_counter = 0usize;
    let mut ticker = interval(Duration::from_millis(100)); // Check every 100ms
    let mut workflow_handles = Vec::new();

    loop {
        ticker.tick().await;

        // Check if we've exceeded duration
        if start_time.elapsed() >= duration {
            break;
        }

        // Spawn workflows to reach target concurrency
        let current_count = in_flight.load(Ordering::Relaxed);

        if current_count < target_count {
            // Spawn multiple workflows at once to reach target faster
            let to_spawn = (target_count - current_count).min(10);

            for _ in 0..to_spawn {
                workflow_counter += 1;
                let workflow_id = format!("load-test-hc-{}-{}", run_id, workflow_counter);
                let input = IoWorkflowInput {
                    id: workflow_counter,
                    delay_ms: args.workflow_duration * 1000, // Convert seconds to milliseconds
                };
                let input_bytes = serde_json::to_vec(&input)?;

                // Record workflow start
                collector.workflow_started();
                in_flight.fetch_add(1, Ordering::Relaxed);
                let start = Instant::now();

                // Start workflow and track the handle
                let client_clone = client.clone();
                let domain_clone = domain.clone();
                let task_list_clone = task_list.clone();
                let collector_clone = collector.clone();
                let in_flight_clone = in_flight.clone();

                let handle = tokio::spawn(async move {
                    let start_request = StartWorkflowExecutionRequest {
                        domain: domain_clone.clone(),
                        workflow_id: workflow_id.clone(),
                        workflow_type: Some(WorkflowType {
                            name: "io_bound_workflow".to_string(),
                        }),
                        task_list: Some(TaskList {
                            name: task_list_clone.clone(),
                            kind: TaskListKind::Normal,
                        }),
                        input: Some(input_bytes.clone()),
                        execution_start_to_close_timeout_seconds: Some(300),
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

                    // Decrement in-flight counter
                    in_flight_clone.fetch_sub(1, Ordering::Relaxed);
                });

                workflow_handles.push(handle);
            }
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

        // Log progress every 50 workflows
        if (idx + 1) % 50 == 0 {
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
