//! Activity context and functions for authoring activities.
//!
//! This module provides the API for implementing activities including
//! heartbeats, context access, and activity information.

use std::time::{Duration, Instant};
use uber_cadence_core::RetryPolicy;

/// Activity runtime trait for interacting with the worker
pub trait ActivityRuntime: Send + Sync {
    /// Record a heartbeat
    fn record_heartbeat(&self, details: Option<Vec<u8>>);

    /// Check if activity is cancelled
    fn is_cancelled(&self) -> bool;
}

/// Activity context for executing activity logic
#[derive(Clone)]
pub struct ActivityContext {
    activity_info: ActivityInfo,
    deadline: Option<Instant>,
    worker_stop_channel: Option<tokio::sync::watch::Receiver<bool>>,
    runtime: Option<std::sync::Arc<dyn ActivityRuntime>>,
}

impl ActivityContext {
    pub fn new(
        activity_info: ActivityInfo,
        runtime: Option<std::sync::Arc<dyn ActivityRuntime>>,
    ) -> Self {
        Self {
            deadline: activity_info.deadline,
            worker_stop_channel: None,
            activity_info,
            runtime,
        }
    }

    /// Create a new activity context for a local activity execution
    ///
    /// Local activities are executed synchronously in the workflow worker process,
    /// so they don't have task tokens or heartbeat capabilities.
    pub fn new_for_local_activity(
        workflow_info: uber_cadence_core::WorkflowInfo,
        activity_type: String,
        activity_id: String,
        attempt: i32,
        scheduled_time: std::time::SystemTime,
    ) -> Self {
        use std::time::{Duration, UNIX_EPOCH};

        // Convert SystemTime to chrono::DateTime
        let scheduled_time_chrono = {
            let duration_since_epoch = scheduled_time
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0));
            let secs = duration_since_epoch.as_secs() as i64;
            let nanos = duration_since_epoch.subsec_nanos();
            chrono::DateTime::from_timestamp(secs, nanos).unwrap_or_else(chrono::Utc::now)
        };

        let activity_info = ActivityInfo {
            task_token: Vec::new(), // Local activities don't have task tokens
            workflow_execution: WorkflowExecution {
                workflow_id: workflow_info.workflow_execution.workflow_id,
                run_id: workflow_info.workflow_execution.run_id,
            },
            activity_id,
            activity_type,
            attempt,
            scheduled_time: scheduled_time_chrono,
            started_time: chrono::Utc::now(),
            deadline: None, // Set by executor based on timeout
            heartbeat_timeout: Duration::from_secs(0), // Local activities don't heartbeat
            heartbeat_details: None,
        };

        Self {
            deadline: None,
            worker_stop_channel: None,
            activity_info,
            runtime: None, // Local activities don't have runtime
        }
    }

    /// Set the worker stop channel
    pub fn set_worker_stop_channel(&mut self, channel: tokio::sync::watch::Receiver<bool>) {
        self.worker_stop_channel = Some(channel);
    }

    /// Get activity information
    pub fn get_info(&self) -> &ActivityInfo {
        &self.activity_info
    }

    /// Record a heartbeat with optional details
    pub fn record_heartbeat(&self, details: Option<&[u8]>) {
        if let Some(runtime) = &self.runtime {
            runtime.record_heartbeat(details.map(|d| d.to_vec()));
        }
    }

    /// Check if heartbeat details exist from previous attempts
    pub fn has_heartbeat_details(&self) -> bool {
        self.activity_info.heartbeat_details.is_some()
    }

    /// Get heartbeat details from previous attempts
    pub fn get_heartbeat_details(&self) -> Option<&[u8]> {
        self.activity_info.heartbeat_details.as_deref()
    }

    /// Get the worker stop channel
    /// This can be used to detect when the worker is shutting down
    pub fn get_worker_stop_channel(&self) -> Option<&tokio::sync::watch::Receiver<bool>> {
        self.worker_stop_channel.as_ref()
    }

    /// Check if the activity has been cancelled
    pub fn is_cancelled(&self) -> bool {
        if let Some(runtime) = &self.runtime {
            return runtime.is_cancelled();
        }
        false
    }

    /// Get the deadline for activity completion
    pub fn get_deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// Get the remaining time before deadline
    pub fn get_remaining_time(&self) -> Option<Duration> {
        self.deadline.map(|d| {
            let now = Instant::now();
            if d > now {
                d - now
            } else {
                Duration::from_secs(0)
            }
        })
    }

    /// Get logger for activity
    pub fn get_logger(&self) -> Box<dyn Logger> {
        Box::new(ConsoleLogger)
    }

    /// Get metrics scope for activity
    pub fn get_metrics_scope(&self) -> Box<dyn MetricsScope> {
        Box::new(NoopMetricsScope)
    }
}

/// Activity information
#[derive(Debug, Clone)]
pub struct ActivityInfo {
    pub activity_id: String,
    pub activity_type: String,
    pub task_token: Vec<u8>,
    pub workflow_execution: WorkflowExecution,
    pub attempt: i32,
    pub scheduled_time: chrono::DateTime<chrono::Utc>,
    pub started_time: chrono::DateTime<chrono::Utc>,
    pub deadline: Option<Instant>,
    pub heartbeat_timeout: Duration,
    pub heartbeat_details: Option<Vec<u8>>,
}

/// Workflow execution identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowExecution {
    pub workflow_id: String,
    pub run_id: String,
}

impl WorkflowExecution {
    pub fn new(workflow_id: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            run_id: run_id.into(),
        }
    }
}

/// Activity options for registration
#[derive(Debug, Clone)]
pub struct ActivityOptions {
    pub name: String,
    pub task_list: String,
    pub schedule_to_close_timeout: Duration,
    pub schedule_to_start_timeout: Duration,
    pub start_to_close_timeout: Duration,
    pub heartbeat_timeout: Duration,
    pub retry_policy: Option<RetryPolicy>,
    pub disable_ekg: bool,
}

impl Default for ActivityOptions {
    fn default() -> Self {
        Self {
            name: String::new(),
            task_list: String::new(),
            schedule_to_close_timeout: Duration::from_secs(0),
            schedule_to_start_timeout: Duration::from_secs(0),
            start_to_close_timeout: Duration::from_secs(0),
            heartbeat_timeout: Duration::from_secs(0),
            retry_policy: None,
            disable_ekg: false,
        }
    }
}

/// Activity registration options
#[derive(Debug, Clone, Default)]
pub struct ActivityRegisterOptions {
    pub name: Option<String>,
    pub disable_already_registered_check: bool,
}

/// Error indicating activity result is pending (async completion)
///
/// An activity can return this error to indicate that it will complete
/// asynchronously. The activity can then be completed later using
/// Client::complete_activity() or Client::complete_activity_by_id().
#[derive(Debug, thiserror::Error)]
#[error("Activity result is pending - will be completed asynchronously")]
pub struct ErrResultPending;

/// Logger trait
pub trait Logger: Send + Sync {
    fn debug(&self, msg: &str);
    fn info(&self, msg: &str);
    fn warn(&self, msg: &str);
    fn error(&self, msg: &str);
}

/// Console logger implementation
struct ConsoleLogger;

impl Logger for ConsoleLogger {
    fn debug(&self, msg: &str) {
        println!("[DEBUG] {}", msg);
    }

    fn info(&self, msg: &str) {
        println!("[INFO] {}", msg);
    }

    fn warn(&self, msg: &str) {
        println!("[WARN] {}", msg);
    }

    fn error(&self, msg: &str) {
        eprintln!("[ERROR] {}", msg);
    }
}

/// Metrics scope trait
pub trait MetricsScope: Send + Sync {
    fn counter(&self, name: &str) -> Box<dyn Counter>;
    fn timer(&self, name: &str) -> Box<dyn Timer>;
    fn gauge(&self, name: &str) -> Box<dyn Gauge>;
}

pub trait Counter: Send + Sync {
    fn inc(&self, delta: i64);
}

pub trait Timer: Send + Sync {
    fn record(&self, duration: Duration);
}

pub trait Gauge: Send + Sync {
    fn update(&self, value: f64);
}

/// Noop metrics scope
struct NoopMetricsScope;

impl MetricsScope for NoopMetricsScope {
    fn counter(&self, _name: &str) -> Box<dyn Counter> {
        Box::new(NoopCounter)
    }

    fn timer(&self, _name: &str) -> Box<dyn Timer> {
        Box::new(NoopTimer)
    }

    fn gauge(&self, _name: &str) -> Box<dyn Gauge> {
        Box::new(NoopGauge)
    }
}

struct NoopCounter;
impl Counter for NoopCounter {
    fn inc(&self, _delta: i64) {}
}

struct NoopTimer;
impl Timer for NoopTimer {
    fn record(&self, _duration: Duration) {}
}

struct NoopGauge;
impl Gauge for NoopGauge {
    fn update(&self, _value: f64) {}
}

/// Register an activity globally
pub fn register<F, R>(_activity_fn: F)
where
    F: Fn(&ActivityContext) -> R + Send + Sync + 'static,
    R: Send + 'static,
{
    // TODO: Implement global activity registration
}

/// Register an activity with options globally
pub fn register_with_options<F, R>(_activity_fn: F, _options: ActivityRegisterOptions)
where
    F: Fn(&ActivityContext) -> R + Send + Sync + 'static,
    R: Send + 'static,
{
    // TODO: Implement global activity registration with options
}

/// Get activity info from context (convenience function)
pub fn get_info(ctx: &ActivityContext) -> &ActivityInfo {
    ctx.get_info()
}

/// Record heartbeat (convenience function)
pub fn record_heartbeat(ctx: &ActivityContext, details: &[u8]) {
    ctx.record_heartbeat(Some(details));
}

/// Check if heartbeat details exist (convenience function)
pub fn has_heartbeat_details(ctx: &ActivityContext) -> bool {
    ctx.has_heartbeat_details()
}

/// Get heartbeat details (convenience function)
pub fn get_heartbeat_details(ctx: &ActivityContext) -> Option<&[u8]> {
    ctx.get_heartbeat_details()
}

/// Get logger (convenience function)
pub fn get_logger(ctx: &ActivityContext) -> Box<dyn Logger> {
    ctx.get_logger()
}

/// Get metrics scope (convenience function)
pub fn get_metrics_scope(ctx: &ActivityContext) -> Box<dyn MetricsScope> {
    ctx.get_metrics_scope()
}

/// Get worker stop channel (convenience function)
pub fn get_worker_stop_channel(
    ctx: &ActivityContext,
) -> Option<&tokio::sync::watch::Receiver<bool>> {
    ctx.get_worker_stop_channel()
}
