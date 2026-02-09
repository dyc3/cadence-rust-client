//! Spike/burst scenario - alternate between baseline and spike load

use std::time::Duration;

use anyhow::Result;
use crabdance_client::GrpcWorkflowServiceClient;
use crabdance_proto::shared::{TaskList, TaskListKind, WorkflowType};
use crabdance_proto::workflow_service::{StartWorkflowExecutionRequest, WorkflowService};
use tokio::time::{interval, Instant};
use uuid::Uuid;

use crate::cli::{ClientArgs, SpikeArgs};
use crate::metrics::collector::MetricsCollector;
use crate::metrics::reporter;
use crate::workflows::noop::NoopInput;

pub async fn run(client_args: ClientArgs, args: SpikeArgs) -> Result<()> {
    tracing::info!("Starting spike/burst scenario");

    let domain = &client_args.domain;
    let task_list = &client_args.task_list;
    let endpoint = &client_args.endpoint;

    // Create client only (no worker)
    let client = GrpcWorkflowServiceClient::connect(endpoint, domain, None).await?;

    tracing::info!("Client connected");

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

    // Use timestamp to ensure workflow IDs are unique across runs
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    tracing::info!(
        "Starting spike pattern: baseline={}/sec, spike={}/sec",
        args.baseline_rate,
        args.spike_rate
    );

    let mut ticker = interval(Duration::from_secs(1));
    let mut workflow_counter = 0usize;
    let mut workflow_handles = Vec::new();
    let mut elapsed_in_interval = 0u64;
    let mut in_spike = false;

    loop {
        ticker.tick().await;

        // Check if we've exceeded duration
        if start_time.elapsed() >= duration {
            break;
        }

        elapsed_in_interval += 1;

        // Determine if we're in spike or baseline
        let interval_duration = if in_spike {
            args.spike_duration
        } else {
            args.spike_interval
        };

        if elapsed_in_interval >= interval_duration {
            // Toggle between spike and baseline
            in_spike = !in_spike;
            elapsed_in_interval = 0;

            let rate = if in_spike {
                args.spike_rate
            } else {
                args.baseline_rate
            };
            tracing::info!(
                "Switching to {} mode: {}/sec",
                if in_spike { "SPIKE" } else { "baseline" },
                rate
            );
        }

        // Get current rate
        let current_rate = if in_spike {
            args.spike_rate
        } else {
            args.baseline_rate
        };

        // Spawn workflows at current rate
        for _ in 0..(current_rate as usize) {
            workflow_counter += 1;
            let workflow_id = format!("load-test-spike-{}-{}", run_id, workflow_counter);
            let input = NoopInput {
                id: workflow_counter,
            };
            let input_bytes = serde_json::to_vec(&input)?;

            // Record workflow start
            collector.workflow_started();
            let start = Instant::now();

            // Start workflow and track the handle
            let client_clone = client.clone();
            let domain_clone = domain.clone();
            let task_list_clone = task_list.clone();
            let collector_clone = collector.clone();

            let handle = tokio::spawn(async move {
                let start_request = StartWorkflowExecutionRequest {
                    domain: domain_clone.clone(),
                    workflow_id: workflow_id.clone(),
                    workflow_type: Some(WorkflowType {
                        name: "noop_workflow".to_string(),
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
