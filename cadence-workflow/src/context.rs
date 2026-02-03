//! Workflow context and core functions for authoring workflows.
//!
//! This module provides the main API for implementing workflows including
//! scheduling activities, child workflows, handling signals, and more.

use cadence_core::{ActivityOptions, ChildWorkflowOptions, RetryPolicy, WorkflowInfo};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::commands::{WorkflowCommand, ScheduleActivityCommand, StartTimerCommand, StartChildWorkflowCommand};
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use futures::future::poll_fn;

use std::sync::atomic::{AtomicU64, Ordering};

/// Type alias for query handlers
pub type QueryHandler = Box<dyn Fn(Vec<u8>) -> Vec<u8> + Send + Sync>;

/// Trait for handling workflow commands (implemented by worker)
pub trait CommandSink: Send + Sync {
    fn submit(&self, command: WorkflowCommand) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>>;
}

/// No-op command sink for testing/initialization
struct NoopCommandSink;
impl CommandSink for NoopCommandSink {
    fn submit(&self, _command: WorkflowCommand) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
        Box::pin(async { Err(WorkflowError::Generic("No command sink configured".into())) })
    }
}

/// Workflow context for executing workflow logic
pub struct WorkflowContext {
    workflow_info: WorkflowInfo,
    command_sink: Arc<dyn CommandSink>,
    sequence: AtomicU64,
    signals: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
    query_handlers: Arc<Mutex<HashMap<String, QueryHandler>>>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl WorkflowContext {
    pub fn new(workflow_info: WorkflowInfo) -> Self {
        Self { 
            workflow_info,
            command_sink: Arc::new(NoopCommandSink),
            sequence: AtomicU64::new(0),
            signals: Arc::new(Mutex::new(HashMap::new())),
            query_handlers: Arc::new(Mutex::new(HashMap::new())),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    pub fn with_sink(
        workflow_info: WorkflowInfo, 
        sink: Arc<dyn CommandSink>, 
        signals: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
        query_handlers: Arc<Mutex<HashMap<String, QueryHandler>>>,
    ) -> Self {
        Self {
            workflow_info,
            command_sink: sink,
            sequence: AtomicU64::new(0),
            signals,
            query_handlers,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    fn next_id(&self) -> String {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        format!("{}", seq)
    }

    /// Get workflow information
    pub fn workflow_info(&self) -> &WorkflowInfo {
        &self.workflow_info
    }

    /// Execute an activity
    pub async fn execute_activity(
        &self,
        activity_type: &str,
        args: Option<Vec<u8>>,
        options: ActivityOptions,
    ) -> Result<Vec<u8>, WorkflowError> {
        let activity_id = self.next_id();
        
        let command = WorkflowCommand::ScheduleActivity(ScheduleActivityCommand {
            activity_id,
            activity_type: activity_type.to_string(),
            args,
            options,
        });
        
        self.command_sink.submit(command).await
    }

    /// Execute a local activity (executed synchronously in workflow thread)
    pub async fn execute_local_activity(
        &self,
        _activity_type: &str,
        _args: Option<Vec<u8>>,
        _options: LocalActivityOptions,
    ) -> Result<Vec<u8>, WorkflowError> {
        // TODO: Implement local activity execution
        unimplemented!("Local activity execution not yet implemented")
    }

    /// Execute a child workflow
    pub async fn execute_child_workflow(
        &self,
        workflow_type: &str,
        args: Option<Vec<u8>>,
        options: ChildWorkflowOptions,
    ) -> Result<Vec<u8>, WorkflowError> {
        let workflow_id = if options.workflow_id.is_empty() {
             self.next_id()
        } else {
             options.workflow_id.clone()
        };

        let command = WorkflowCommand::StartChildWorkflow(StartChildWorkflowCommand {
            workflow_id,
            workflow_type: workflow_type.to_string(),
            args,
            options,
        });

        self.command_sink.submit(command).await
    }

    /// Get a signal channel for receiving signals
    pub fn get_signal_channel(&self, signal_name: &str) -> SignalChannel {
        SignalChannel::new(signal_name, self.signals.clone())
    }

    /// Signal an external workflow
    pub async fn signal_external_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        signal_name: &str,
        args: Option<Vec<u8>>,
    ) -> Result<(), WorkflowError> {
        let signal_id = self.next_id();
        let command = WorkflowCommand::SignalExternalWorkflow(crate::commands::SignalExternalWorkflowCommand {
            signal_id,
            domain: None, // TODO: support domain
            workflow_id: workflow_id.to_string(),
            run_id: run_id.map(|s| s.to_string()),
            signal_name: signal_name.to_string(),
            args,
            child_workflow_only: false,
        });
        
        let _ = self.command_sink.submit(command).await?;
        Ok(())
    }

    /// Request cancellation of an external workflow
    pub async fn request_cancel_external_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
    ) -> Result<(), WorkflowError> {
        let cancellation_id = self.next_id();
        let command = WorkflowCommand::RequestCancelExternalWorkflow(crate::commands::RequestCancelExternalWorkflowCommand {
            cancellation_id,
            domain: None, // TODO: support domain
            workflow_id: workflow_id.to_string(),
            run_id: run_id.map(|s| s.to_string()),
            child_workflow_only: false,
        });
        
        let _ = self.command_sink.submit(command).await?;
        Ok(())
    }

    /// Execute a side effect (non-deterministic operation)
    pub async fn side_effect<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // TODO: Implement side effect caching
        f()
    }

    /// Execute a mutable side effect (cached side effect)
    pub async fn mutable_side_effect<F, R>(&self, _id: &str, f: F) -> R
    where
        F: FnOnce() -> R,
        R: Clone,
    {
        // TODO: Implement mutable side effect with caching
        f()
    }

    /// Get version for backwards-compatible workflow changes
    pub fn get_version(&self, _change_id: &str, min_supported: i32, _max_supported: i32) -> i32 {
        // TODO: Implement versioning
        min_supported
    }

    /// Set a query handler
    pub fn set_query_handler<F>(&self, query_type: &str, handler: F)
    where
        F: Fn(Vec<u8>) -> Vec<u8> + Send + Sync + 'static,
    {
        let mut handlers = self.query_handlers.lock().unwrap();
        handlers.insert(query_type.to_string(), Box::new(handler));
    }

    /// Upsert search attributes
    pub fn upsert_search_attributes(&self, _search_attributes: Vec<(String, Vec<u8>)>) {
        // TODO: Implement search attributes upsert
    }

    /// Sleep for a duration (workflow-aware)
    pub async fn sleep(&self, duration: Duration) {
        let timer_id = self.next_id();
        let command = WorkflowCommand::StartTimer(StartTimerCommand {
            timer_id,
            duration,
        });
        let _ = self.command_sink.submit(command).await;
    }

    /// Get current workflow time (deterministic)
    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        // TODO: Implement deterministic time
        chrono::Utc::now()
    }

    /// Get current workflow time (alias for `now`)
    pub fn current_time(&self) -> chrono::DateTime<chrono::Utc> {
        self.now()
    }

    /// Create a timer
    pub fn new_timer(&self, duration: Duration) -> TimerFuture {
        let timer_id = self.next_id();
        let command = WorkflowCommand::StartTimer(StartTimerCommand {
            timer_id,
            duration,
        });
        let future = self.command_sink.submit(command);
        Box::pin(async move {
            let _ = future.await;
        })
    }

    /// Get logger
    pub fn get_logger(&self) -> Box<dyn Logger> {
        Box::new(ConsoleLogger)
    }

    /// Get metrics scope
    pub fn get_metrics_scope(&self) -> Box<dyn MetricsScope> {
        Box::new(NoopMetricsScope)
    }

    /// Continue workflow as new
    pub async fn continue_as_new(
        &self,
        workflow_type: &str,
        args: Option<Vec<u8>>,
        options: ContinueAsNewOptions,
    ) -> ! {
        let command = WorkflowCommand::ContinueAsNewWorkflow(crate::commands::ContinueAsNewWorkflowCommand {
            workflow_type: workflow_type.to_string(),
            input: args,
            options,
        });

        let _ = self.command_sink.submit(command).await;
        
        // Block forever
        std::future::pending().await
    }

    /// Get a cancellation channel
    pub fn get_cancellation_channel(&self) -> CancellationChannel {
        CancellationChannel::new(self.cancelled.clone())
    }

    /// Check if workflow is cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
    
    pub fn set_cancelled(&self, cancelled: bool) {
        self.cancelled.store(cancelled, Ordering::Relaxed);
    }
}

/// Local activity options
#[derive(Debug, Clone)]
pub struct LocalActivityOptions {
    pub schedule_to_close_timeout: Duration,
    pub retry_policy: Option<RetryPolicy>,
}

/// Continue as new options
#[derive(Debug, Clone)]
pub struct ContinueAsNewOptions {
    pub task_list: String,
    pub execution_start_to_close_timeout: Duration,
    pub task_start_to_close_timeout: Duration,
    pub retry_policy: Option<RetryPolicy>,
    pub cron_schedule: Option<String>,
    pub memo: Option<Vec<(String, Vec<u8>)>>,
    pub search_attributes: Option<Vec<(String, Vec<u8>)>>,
}

/// Signal channel for receiving signals
pub struct SignalChannel {
    signal_name: String,
    signals: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
}

impl SignalChannel {
    pub fn new(signal_name: &str, signals: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>) -> Self {
        Self {
            signal_name: signal_name.to_string(),
            signals,
        }
    }

    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        poll_fn(|_cx| {
            let mut signals = self.signals.lock().unwrap();
            if let Some(list) = signals.get_mut(&self.signal_name) {
                if !list.is_empty() {
                    return Poll::Ready(Some(list.remove(0)));
                }
            }
            Poll::Pending
        }).await
    }

    pub fn try_recv(&mut self) -> Option<Vec<u8>> {
        let mut signals = self.signals.lock().unwrap();
        if let Some(list) = signals.get_mut(&self.signal_name) {
            if !list.is_empty() {
                return Some(list.remove(0));
            }
        }
        None
    }
}

/// Cancellation channel
pub struct CancellationChannel {
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationChannel {
    pub fn new(cancelled: Arc<std::sync::atomic::AtomicBool>) -> Self {
        Self { cancelled }
    }

    pub async fn recv(&mut self) {
        poll_fn(|_cx| {
            if self.cancelled.load(Ordering::Relaxed) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }).await
    }
}

// Re-export types from future module
pub use crate::future::{ActivityError, TimerFuture, WorkflowError};

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