use crabdance_worker::worker::{NonDeterministicWorkflowPolicy, WorkerOptions};
use std::time::Duration;

/// Get worker options by profile name
pub fn get_worker_profile(profile: &str) -> WorkerOptions {
    match profile {
        "dev" => development_profile(),
        "high-perf" => high_performance_profile(),
        "stress" => stress_test_profile(),
        _ => {
            eprintln!("Unknown profile '{}', using 'high-perf' profile", profile);
            high_performance_profile()
        }
    }
}

/// High-performance profile for load testing
///
/// Optimized for maximum throughput and concurrency:
/// - 2000 concurrent activities
/// - 2000 concurrent decision tasks
/// - 500 activities/sec, 100 decision tasks/sec
/// - 4 pollers for each type
/// - Sticky execution enabled
/// - 1000 cached workflows
pub fn high_performance_profile() -> WorkerOptions {
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
        max_cached_workflows: 1000,
        non_deterministic_workflow_policy: NonDeterministicWorkflowPolicy::FailWorkflow,
        identity: "loadtest-high-perf-worker".to_string(),
        deadlock_detection_timeout: Duration::from_secs(60),
    }
}

/// Development profile for testing and debugging
///
/// Lower limits for easier debugging:
/// - 100 concurrent activities
/// - 100 concurrent decision tasks
/// - 50 activities/sec, 20 decision tasks/sec
/// - 1 poller for each type
/// - Sticky execution disabled for simpler debugging
/// - 100 cached workflows
pub fn development_profile() -> WorkerOptions {
    WorkerOptions {
        max_concurrent_activity_execution_size: 100,
        max_concurrent_local_activity_execution_size: 100,
        max_concurrent_decision_task_execution_size: 100,
        worker_activities_per_second: 50.0,
        worker_local_activities_per_second: 50.0,
        worker_decision_tasks_per_second: 20.0,
        max_concurrent_decision_task_pollers: 1,
        max_concurrent_activity_task_pollers: 1,
        disable_sticky_execution: true,
        sticky_schedule_to_start_timeout: Duration::from_secs(5),
        worker_stop_timeout: Duration::from_secs(5),
        enable_session_worker: false,
        max_concurrent_session_execution_size: 100,
        max_cached_workflows: 100,
        non_deterministic_workflow_policy: NonDeterministicWorkflowPolicy::BlockWorkflow,
        identity: "loadtest-dev-worker".to_string(),
        deadlock_detection_timeout: Duration::from_secs(30),
    }
}

/// Stress test profile for extreme load testing
///
/// Maximum limits for stress testing:
/// - 5000 concurrent activities
/// - 5000 concurrent decision tasks
/// - 1000 activities/sec, 200 decision tasks/sec
/// - 8 pollers for each type
/// - Sticky execution enabled
/// - 5000 cached workflows
pub fn stress_test_profile() -> WorkerOptions {
    WorkerOptions {
        max_concurrent_activity_execution_size: 5000,
        max_concurrent_local_activity_execution_size: 5000,
        max_concurrent_decision_task_execution_size: 5000,
        worker_activities_per_second: 1000.0,
        worker_local_activities_per_second: 1000.0,
        worker_decision_tasks_per_second: 200.0,
        max_concurrent_decision_task_pollers: 8,
        max_concurrent_activity_task_pollers: 8,
        disable_sticky_execution: false,
        sticky_schedule_to_start_timeout: Duration::from_secs(15),
        worker_stop_timeout: Duration::from_secs(60),
        enable_session_worker: false,
        max_concurrent_session_execution_size: 5000,
        max_cached_workflows: 5000,
        non_deterministic_workflow_policy: NonDeterministicWorkflowPolicy::FailWorkflow,
        identity: "loadtest-stress-worker".to_string(),
        deadlock_detection_timeout: Duration::from_secs(120),
    }
}
