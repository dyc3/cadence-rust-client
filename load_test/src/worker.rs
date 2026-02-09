//! Worker module - standalone worker for load testing
//!
//! This module provides a standalone worker that can be started independently
//! from the load test client, allowing for realistic testing scenarios where
//! the worker runs continuously and handles requests from multiple clients.

use std::sync::Arc;

use anyhow::Result;
use crabdance_client::GrpcWorkflowServiceClient;
use crabdance_core::TransportError;
use crabdance_proto::workflow_service::{RegisterDomainRequest, WorkflowService};
use crabdance_worker::registry::Registry;
use crabdance_worker::{CadenceWorker, Worker, WorkerOptions};
use uuid::Uuid;

use crate::activities::cpu_intensive::CpuIntensiveActivity;
use crate::activities::failing::FailingActivity;
use crate::activities::fast::FastActivity;
use crate::activities::io_simulated::IoSimulatedActivity;
use crate::config;
use crate::workflows::cpu_bound::CpuBoundWorkflow;
use crate::workflows::failing::FailingWorkflow;
use crate::workflows::io_bound::IoBoundWorkflow;
use crate::workflows::noop::NoopWorkflow;

/// Run the load test worker
///
/// This function creates and starts a Cadence worker with all load test
/// workflows and activities registered. The worker will run until Ctrl+C is pressed.
pub async fn run_worker(
    endpoint: String,
    domain: String,
    task_list: String,
    worker_profile: String,
    verbose: bool,
) -> Result<()> {
    // Create client
    let client = GrpcWorkflowServiceClient::connect(&endpoint, &domain, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect: {}", e))?;
    let service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync> =
        Arc::new(client.clone());

    // Register domain if needed
    register_domain_if_needed(&client, &domain).await?;

    // Create registry
    let registry = Arc::new(crabdance_worker::registry::WorkflowRegistry::new());

    // Register ALL load test workflows
    registry.register_workflow("noop_workflow", Box::new(NoopWorkflow));
    registry.register_workflow("cpu_bound_workflow", Box::new(CpuBoundWorkflow));
    registry.register_workflow("io_bound_workflow", Box::new(IoBoundWorkflow));
    registry.register_workflow("failing_workflow", Box::new(FailingWorkflow));

    // Register ALL activities
    registry.register_activity("fast_activity", Box::new(FastActivity));
    registry.register_activity("cpu_intensive", Box::new(CpuIntensiveActivity));
    registry.register_activity("io_simulated", Box::new(IoSimulatedActivity));
    registry.register_activity("failing", Box::new(FailingActivity));

    // Get worker configuration
    let worker_opts = config::get_worker_profile(&worker_profile);
    let worker_opts = WorkerOptions {
        identity: format!("load-test-worker-{}", Uuid::new_v4()),
        disable_sticky_execution: true, // Simplify for load testing
        ..worker_opts
    };

    // Print startup info
    tracing::info!("Starting Cadence Load Test Worker");
    tracing::info!("  Endpoint: {}", endpoint);
    tracing::info!("  Domain: {}", domain);
    tracing::info!("  Task List: {}", task_list);
    tracing::info!("  Profile: {}", worker_profile);
    tracing::info!(
        "  Max Concurrent Activities: {}",
        worker_opts.max_concurrent_activity_execution_size
    );
    tracing::info!(
        "  Max Concurrent Decisions: {}",
        worker_opts.max_concurrent_decision_task_execution_size
    );
    tracing::info!(
        "  Decision Pollers: {}",
        worker_opts.max_concurrent_decision_task_pollers
    );
    tracing::info!(
        "  Activity Pollers: {}",
        worker_opts.max_concurrent_activity_task_pollers
    );

    if verbose {
        tracing::debug!("Verbose logging enabled");
    }

    tracing::info!("Worker ready. Press Ctrl+C to stop.");

    // Create and run worker (blocks until Ctrl+C)
    let worker = CadenceWorker::new(&domain, &task_list, worker_opts, registry, service);
    worker.run()?;

    tracing::info!("Worker stopped");
    Ok(())
}

/// Helper function to register domain if it doesn't exist
async fn register_domain_if_needed(
    client: &GrpcWorkflowServiceClient,
    domain_name: &str,
) -> Result<(), TransportError> {
    let register_request = RegisterDomainRequest {
        name: domain_name.to_string(),
        description: Some("Load test domain".to_string()),
        owner_email: "loadtest@example.com".to_string(),
        workflow_execution_retention_period_in_days: 1,
        emit_metric: None,
        data: std::collections::HashMap::new(),
        security_token: None,
        is_global_domain: None,
        active_cluster_name: String::new(),
        clusters: vec![],
        history_archival_status: None,
        history_archival_uri: None,
        visibility_archival_status: None,
        visibility_archival_uri: None,
    };

    // Ignore error if domain already exists
    let _ = client.register_domain(register_request).await;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    Ok(())
}
