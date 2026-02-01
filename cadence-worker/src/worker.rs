//! Worker implementation for hosting workflows and activities.
//!
//! This module provides the worker for polling tasks from the Cadence server
//! and executing workflow and activity implementations.

use crate::registry::Registry;
use std::sync::Arc;
use std::time::Duration;

/// Worker trait for hosting workflows and activities
pub trait Worker: Send + Sync {
    /// Start the worker in non-blocking mode
    fn start(&self) -> Result<(), WorkerError>;

    /// Run the worker (blocking)
    fn run(&self) -> Result<(), WorkerError>;

    /// Stop the worker
    fn stop(&self);
}

/// Worker errors
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    #[error("Worker already started")]
    AlreadyStarted,
    #[error("Failed to connect to Cadence server: {0}")]
    ConnectionFailed(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("Worker is shutting down")]
    ShuttingDown,
}

/// Worker options for configuration
#[derive(Clone)]
pub struct WorkerOptions {
    /// Maximum concurrent activity executions
    pub max_concurrent_activity_execution_size: usize,
    /// Maximum concurrent local activity executions
    pub max_concurrent_local_activity_execution_size: usize,
    /// Maximum concurrent decision task executions
    pub max_concurrent_decision_task_execution_size: usize,
    /// Worker activities per second (rate limit)
    pub worker_activities_per_second: f64,
    /// Worker local activities per second (rate limit)
    pub worker_local_activities_per_second: f64,
    /// Worker decision tasks per second (rate limit)
    pub worker_decision_tasks_per_second: f64,
    /// Maximum concurrent decision task pollers
    pub max_concurrent_decision_task_pollers: usize,
    /// Maximum concurrent activity task pollers
    pub max_concurrent_activity_task_pollers: usize,
    /// Disable sticky execution
    pub disable_sticky_execution: bool,
    /// Sticky schedule to start timeout
    pub sticky_schedule_to_start_timeout: Duration,
    /// Worker stop timeout
    pub worker_stop_timeout: Duration,
    /// Enable session worker
    pub enable_session_worker: bool,
    /// Max concurrent session execution size
    pub max_concurrent_session_execution_size: usize,
    /// Non-deterministic workflow policy
    pub non_deterministic_workflow_policy: NonDeterministicWorkflowPolicy,
    /// Identity
    pub identity: String,
    /// Deadlock detection timeout
    pub deadlock_detection_timeout: Duration,
}

impl Default for WorkerOptions {
    fn default() -> Self {
        Self {
            max_concurrent_activity_execution_size: 1000,
            max_concurrent_local_activity_execution_size: 1000,
            max_concurrent_decision_task_execution_size: 1000,
            worker_activities_per_second: 100_000.0, // Unlimited
            worker_local_activities_per_second: 100_000.0, // Unlimited
            worker_decision_tasks_per_second: 100_000.0, // Unlimited
            max_concurrent_decision_task_pollers: 2,
            max_concurrent_activity_task_pollers: 2,
            disable_sticky_execution: false,
            sticky_schedule_to_start_timeout: Duration::from_secs(5),
            worker_stop_timeout: Duration::from_secs(10),
            enable_session_worker: false,
            max_concurrent_session_execution_size: 1000,
            non_deterministic_workflow_policy: NonDeterministicWorkflowPolicy::BlockWorkflow,
            identity: format!(
                "cadence-rust-worker@{}-pid-{}",
                std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()),
                std::process::id()
            ),
            deadlock_detection_timeout: Duration::from_secs(0), // Disabled by default
        }
    }
}

impl std::fmt::Debug for WorkerOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerOptions")
            .field(
                "max_concurrent_activity_execution_size",
                &self.max_concurrent_activity_execution_size,
            )
            .field(
                "max_concurrent_decision_task_execution_size",
                &self.max_concurrent_decision_task_execution_size,
            )
            .field("disable_sticky_execution", &self.disable_sticky_execution)
            .field("identity", &self.identity)
            .finish()
    }
}

/// Non-deterministic workflow policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonDeterministicWorkflowPolicy {
    /// Block the workflow (default)
    BlockWorkflow,
    /// Fail the workflow
    FailWorkflow,
}

/// Shadow options for workflow shadower
#[derive(Debug, Clone)]
pub struct ShadowOptions {
    pub workflow_query: String,
    pub shadow_mode: ShadowMode,
    pub exit_condition: Option<ShadowExitCondition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowMode {
    Normal,
    Continuous,
}

#[derive(Debug, Clone)]
pub struct ShadowExitCondition {
    pub expiration_interval: Duration,
}

/// Replay options
#[derive(Debug, Clone)]
pub struct ReplayOptions {
    pub max_concurrent_decision_task_pollers: usize,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self {
            max_concurrent_decision_task_pollers: 1,
        }
    }
}

/// Auto-scaler options
#[derive(Debug, Clone)]
pub struct AutoScalerOptions {
    pub enabled: bool,
    pub target_poll_duration: Duration,
    pub min_pollers: usize,
    pub max_pollers: usize,
}

impl Default for AutoScalerOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            target_poll_duration: Duration::from_millis(100),
            min_pollers: 2,
            max_pollers: 10,
        }
    }
}

/// Worker implementation
pub struct CadenceWorker {
    #[allow(dead_code)]
    domain: String,
    #[allow(dead_code)]
    task_list: String,
    #[allow(dead_code)]
    options: WorkerOptions,
    #[allow(dead_code)]
    registry: Arc<dyn Registry>,
    #[allow(dead_code)]
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl CadenceWorker {
    pub fn new(
        domain: impl Into<String>,
        task_list: impl Into<String>,
        options: WorkerOptions,
        registry: Arc<dyn Registry>,
    ) -> Self {
        Self {
            domain: domain.into(),
            task_list: task_list.into(),
            options,
            registry,
            shutdown_tx: None,
        }
    }
}

impl Worker for CadenceWorker {
    fn start(&self) -> Result<(), WorkerError> {
        // TODO: Implement worker start
        Ok(())
    }

    fn run(&self) -> Result<(), WorkerError> {
        // TODO: Implement blocking run
        Ok(())
    }

    fn stop(&self) {
        // TODO: Implement graceful shutdown
    }
}

/// Sets the global sticky workflow cache size
pub fn set_sticky_workflow_cache_size(_cache_size: usize) {
    // TODO: Implement
}

/// Sets the binary checksum
pub fn set_binary_checksum(_checksum: &str) {
    // TODO: Implement
}
