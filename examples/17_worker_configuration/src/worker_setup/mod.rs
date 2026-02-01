//! Worker setup and configuration for worker configuration example.
//!
//! This module demonstrates how to configure and set up workers with
//! various options and patterns.

use cadence_worker::registry::{Registry, WorkflowRegistry};
use cadence_worker::worker::{CadenceWorker, NonDeterministicWorkflowPolicy, WorkerOptions};
use cadence_worker::ActivityError;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// Creates worker options with performance tuning
pub fn create_high_performance_options() -> WorkerOptions {
    WorkerOptions {
        max_concurrent_activity_execution_size: 2000,
        max_concurrent_local_activity_execution_size: 2000,
        max_concurrent_decision_task_execution_size: 2000,
        worker_activities_per_second: 500.0,
        worker_local_activities_per_second: 500.0,
        worker_decision_tasks_per_second: 100.0,
        max_concurrent_decision_task_pollers: 4,
        max_concurrent_activity_task_pollers: 4,
        disable_sticky_execution: false,
        sticky_schedule_to_start_timeout: Duration::from_secs(10),
        worker_stop_timeout: Duration::from_secs(30),
        enable_session_worker: false,
        max_concurrent_session_execution_size: 1000,
        non_deterministic_workflow_policy: NonDeterministicWorkflowPolicy::FailWorkflow,
        identity: "high-performance-worker".to_string(),
        deadlock_detection_timeout: Duration::from_secs(60),
    }
}

/// Creates worker options for development/testing
pub fn create_development_options() -> WorkerOptions {
    WorkerOptions {
        max_concurrent_activity_execution_size: 100,
        max_concurrent_local_activity_execution_size: 100,
        max_concurrent_decision_task_execution_size: 100,
        worker_activities_per_second: 50.0,
        worker_local_activities_per_second: 50.0,
        worker_decision_tasks_per_second: 20.0,
        max_concurrent_decision_task_pollers: 1,
        max_concurrent_activity_task_pollers: 1,
        disable_sticky_execution: true, // Disable for simpler debugging
        sticky_schedule_to_start_timeout: Duration::from_secs(5),
        worker_stop_timeout: Duration::from_secs(5),
        enable_session_worker: false,
        max_concurrent_session_execution_size: 100,
        non_deterministic_workflow_policy: NonDeterministicWorkflowPolicy::BlockWorkflow,
        identity: "dev-worker".to_string(),
        deadlock_detection_timeout: Duration::from_secs(30),
    }
}

/// Creates worker options for memory-constrained environments
pub fn create_limited_resources_options() -> WorkerOptions {
    WorkerOptions {
        max_concurrent_activity_execution_size: 50,
        max_concurrent_local_activity_execution_size: 50,
        max_concurrent_decision_task_execution_size: 50,
        worker_activities_per_second: 10.0,
        worker_local_activities_per_second: 10.0,
        worker_decision_tasks_per_second: 10.0,
        max_concurrent_decision_task_pollers: 1,
        max_concurrent_activity_task_pollers: 1,
        disable_sticky_execution: false,
        sticky_schedule_to_start_timeout: Duration::from_secs(5),
        worker_stop_timeout: Duration::from_secs(10),
        enable_session_worker: false,
        max_concurrent_session_execution_size: 50,
        non_deterministic_workflow_policy: NonDeterministicWorkflowPolicy::BlockWorkflow,
        identity: "resource-limited-worker".to_string(),
        deadlock_detection_timeout: Duration::from_secs(0),
    }
}

/// Example function to set up a worker with registry
pub fn setup_worker(domain: &str, task_list: &str, options: WorkerOptions) -> CadenceWorker {
    info!(
        "Setting up worker for domain: {}, task_list: {}",
        domain, task_list
    );

    // Create a registry
    let registry = Arc::new(WorkflowRegistry::new());

    // Create the worker
    let worker = CadenceWorker::new(domain, task_list, options, registry);

    info!("Worker setup complete");
    worker
}

/// Example: Register workflows and activities with a registry
pub fn register_workflows_and_activities(registry: &WorkflowRegistry) {
    info!("Registering workflows and activities...");

    // Note: In a real implementation, you would wrap async functions
    // in structs that implement the Activity and Workflow traits.
    // For this example, we demonstrate the registration API:

    // Register activities (conceptual - actual implementation requires trait wrappers)
    // registry.register_activity("compute", Box::new(ComputeActivity));
    // registry.register_activity("process_data", Box::new(ProcessDataActivity));

    // Get registered activities info
    let activities = registry.get_registered_activities();
    info!("Registered {} activities:", activities.len());
    for activity in &activities {
        info!("  - {} (type: {})", activity.name, activity.type_name);
    }

    // Register workflows (conceptual - actual implementation requires trait wrappers)
    // registry.register_workflow("data_processing", Box::new(DataProcessingWorkflow));
    // registry.register_workflow("simple_compute", Box::new(SimpleComputeWorkflow));

    // Get registered workflows info
    let workflows = registry.get_registered_workflows();
    info!("Registered {} workflows:", workflows.len());
    for workflow in &workflows {
        info!("  - {} (type: {})", workflow.name, workflow.type_name);
    }
}

/// Example: Print worker configuration
pub fn print_worker_config(options: &WorkerOptions) {
    info!("Worker Configuration:");
    info!("  Identity: {}", options.identity);
    info!(
        "  Max Concurrent Activities: {}",
        options.max_concurrent_activity_execution_size
    );
    info!(
        "  Max Concurrent Local Activities: {}",
        options.max_concurrent_local_activity_execution_size
    );
    info!(
        "  Max Concurrent Decision Tasks: {}",
        options.max_concurrent_decision_task_execution_size
    );
    info!(
        "  Activities Per Second: {}",
        options.worker_activities_per_second
    );
    info!(
        "  Decision Tasks Per Second: {}",
        options.worker_decision_tasks_per_second
    );
    info!(
        "  Decision Task Pollers: {}",
        options.max_concurrent_decision_task_pollers
    );
    info!(
        "  Activity Task Pollers: {}",
        options.max_concurrent_activity_task_pollers
    );
    info!("  Sticky Execution: {}", !options.disable_sticky_execution);
    info!("  Worker Stop Timeout: {:?}", options.worker_stop_timeout);
    info!(
        "  Deadlock Detection: {:?}",
        options.deadlock_detection_timeout
    );
    info!(
        "  Non-Deterministic Policy: {:?}",
        options.non_deterministic_workflow_policy
    );
}
