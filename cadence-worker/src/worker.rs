//! Worker implementation for hosting workflows and activities.
//!
//! This module provides the worker for polling tasks from the Cadence server
//! and executing workflow and activity implementations.

use crate::executor::cache::WorkflowCache;
use crate::executor::workflow::WorkflowExecutor;
use crate::handlers::activity::ActivityTaskHandler;
use crate::handlers::decision::DecisionTaskHandler;
use crate::pollers::{ActivityTaskPoller, DecisionTaskPoller, PollerManager};
use crate::registry::Registry;
use cadence_core::CadenceError;
use cadence_proto::workflow_service::WorkflowService;
use std::sync::{Arc, Mutex};
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
    #[error("Cadence error: {0}")]
    CadenceError(#[from] CadenceError),
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
    /// Max cached workflows
    pub max_cached_workflows: usize,
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
            max_cached_workflows: 1000,
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
            .field("max_cached_workflows", &self.max_cached_workflows)
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
    domain: String,
    task_list: String,
    options: WorkerOptions,
    registry: Arc<dyn Registry>,
    service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync>,
    poller_manager: Arc<Mutex<Option<PollerManager>>>,
}

impl CadenceWorker {
    pub fn new(
        domain: impl Into<String>,
        task_list: impl Into<String>,
        options: WorkerOptions,
        registry: Arc<dyn Registry>,
        service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync>,
    ) -> Self {
        Self {
            domain: domain.into(),
            task_list: task_list.into(),
            options,
            registry,
            service,
            poller_manager: Arc::new(Mutex::new(None)),
        }
    }
}

impl Worker for CadenceWorker {
    fn start(&self) -> Result<(), WorkerError> {
        let mut manager_lock = self.poller_manager.lock().unwrap();
        if manager_lock.is_some() {
            return Err(WorkerError::AlreadyStarted);
        }

        let mut poller_manager = PollerManager::new();

        // Create cache
        let cache = Arc::new(WorkflowCache::new(self.options.max_cached_workflows));

        // Create executor
        let executor = Arc::new(WorkflowExecutor::new(
            self.registry.clone(),
            cache,
            self.options.clone(),
            self.task_list.clone(),
        ));

        // Create decision handler
        let decision_handler = Arc::new(DecisionTaskHandler::new(
            self.service.clone(),
            executor,
            self.options.identity.clone(),
        ));

        // Create activity handler
        let activity_handler = Arc::new(ActivityTaskHandler::new(
            self.service.clone(),
            self.registry.clone(),
            self.options.identity.clone(),
        ));

        // Create decision pollers
        for i in 0..self.options.max_concurrent_decision_task_pollers {
            let identity = format!("{}-decision-{}", self.options.identity, i);
            let mut poller = DecisionTaskPoller::new(
                self.service.clone(),
                &self.domain,
                &self.task_list,
                &identity,
                decision_handler.clone(),
            );

            if !self.options.disable_sticky_execution {
                let sticky_task_list =
                    format!("{}-{}-sticky", self.task_list, self.options.identity);
                poller = poller.with_sticky_task_list(sticky_task_list);
            }

            poller_manager.add_decision_poller(Arc::new(poller));
        }

        // Create activity pollers
        for i in 0..self.options.max_concurrent_activity_task_pollers {
            let identity = format!("{}-activity-{}", self.options.identity, i);
            let poller = ActivityTaskPoller::new(
                self.service.clone(),
                &self.domain,
                &self.task_list,
                &identity,
                activity_handler.clone(),
            );
            poller_manager.add_activity_poller(Arc::new(poller));
        }

        // Start pollers
        poller_manager.start();

        *manager_lock = Some(poller_manager);
        Ok(())
    }

    fn run(&self) -> Result<(), WorkerError> {
        self.start()?;

        // Create a runtime for the signal handler if we aren't in one,
        // but typically this run() is called from main.
        // We'll assume we are in a context where we can block on a future
        // or we are just blocking the thread until a signal.

        // Since we need to block this thread, and we want to wait for SIGINT,
        // we can use a simple approach:

        tracing::info!("Worker started. Press Ctrl+C to stop.");

        let (tx, rx) = std::sync::mpsc::channel();

        // Spawn a thread to handle signals
        std::thread::spawn(move || {
            // This requires the signal-hook or similar crate which might not be in dependencies.
            // Alternatively, if we are in a tokio runtime, we can spawn a task.
            // But run() is not async.

            // Let's rely on the user to stop via `stop()` if they are embedding this.
            // But for a standalone worker, we need signal handling.

            // For now, let's use a simple sleep loop with a check, but better than before.
            // Or better: just block until we get a notification.

            // If the user wants to use this in main:
            // let worker = Worker::new(...);
            // worker.run()?;

            // We can use tokio's block_on to wait for a signal if tokio is available.
            // Assuming tokio is a dependency (it is).

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                match tokio::signal::ctrl_c().await {
                    Ok(_) => {
                        println!("Received Ctrl+C, shutting down worker...");
                        let _ = tx.send(());
                    }
                    Err(err) => {
                        eprintln!("Unable to listen for shutdown signal: {}", err);
                        // don't kill, just exit signal listener
                    }
                }
            });
        });

        // Block until signal received
        let _ = rx.recv();

        self.stop();

        Ok(())
    }

    fn stop(&self) {
        let mut manager_lock = self.poller_manager.lock().unwrap();
        if let Some(manager) = manager_lock.as_mut() {
            futures::executor::block_on(manager.stop());
        }
        *manager_lock = None;
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
