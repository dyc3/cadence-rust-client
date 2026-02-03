//! Workflow context and core functions for authoring workflows.
//!
//! This module provides the main API for implementing workflows including
//! scheduling activities, child workflows, handling signals, and more.

use crate::commands::{
    RecordMarkerCommand, ScheduleActivityCommand, StartChildWorkflowCommand, StartTimerCommand,
    WorkflowCommand,
};
use cadence_core::{ActivityOptions, ChildWorkflowOptions, RetryPolicy, WorkflowInfo};
use futures::future::poll_fn;
use serde::Serialize;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;

use std::sync::atomic::{AtomicU64, Ordering};

/// Marker names for side effects
pub const SIDE_EFFECT_MARKER_NAME: &str = "SideEffect";
pub const MUTABLE_SIDE_EFFECT_MARKER_NAME: &str = "MutableSideEffect";
pub const VERSION_MARKER_NAME: &str = "Version";

/// Type alias for query handlers
pub type QueryHandler = Box<dyn Fn(Vec<u8>) -> Vec<u8> + Send + Sync>;

/// Trait for handling workflow commands (implemented by worker)
pub trait CommandSink: Send + Sync {
    fn submit(
        &self,
        command: WorkflowCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>>;
}

/// No-op command sink for testing/initialization
struct NoopCommandSink;
impl CommandSink for NoopCommandSink {
    fn submit(
        &self,
        _command: WorkflowCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>> {
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
    // Side effect result caches for replay
    side_effect_results: Arc<Mutex<HashMap<u64, Vec<u8>>>>,
    mutable_side_effects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    // Replay flag - true when workflow is being replayed from history
    is_replay: Arc<std::sync::atomic::AtomicBool>,
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
            side_effect_results: Arc::new(Mutex::new(HashMap::new())),
            mutable_side_effects: Arc::new(Mutex::new(HashMap::new())),
            is_replay: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn with_sink(
        workflow_info: WorkflowInfo,
        sink: Arc<dyn CommandSink>,
        signals: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
        query_handlers: Arc<Mutex<HashMap<String, QueryHandler>>>,
        side_effect_results: Arc<Mutex<HashMap<u64, Vec<u8>>>>,
        mutable_side_effects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    ) -> Self {
        Self {
            workflow_info,
            command_sink: sink,
            sequence: AtomicU64::new(0),
            signals,
            query_handlers,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            side_effect_results,
            mutable_side_effects,
            is_replay: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Set replay mode
    pub fn set_replay_mode(&self, is_replay: bool) {
        self.is_replay.store(is_replay, Ordering::SeqCst);
    }

    /// Set side effect results cache (used during replay)
    pub fn set_side_effect_results(&self, results: HashMap<u64, Vec<u8>>) {
        let mut cache = self.side_effect_results.lock().unwrap();
        *cache = results;
    }

    /// Set mutable side effects cache (used during replay)
    pub fn set_mutable_side_effects(&self, effects: HashMap<String, Vec<u8>>) {
        let mut cache = self.mutable_side_effects.lock().unwrap();
        *cache = effects;
    }

    fn next_id(&self) -> String {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        format!("{}", seq)
    }

    /// Get the next sequence ID as u64 (for side effects)
    fn next_sequence_id(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::SeqCst)
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
        let command = WorkflowCommand::SignalExternalWorkflow(
            crate::commands::SignalExternalWorkflowCommand {
                signal_id,
                domain: None, // TODO: support domain
                workflow_id: workflow_id.to_string(),
                run_id: run_id.map(|s| s.to_string()),
                signal_name: signal_name.to_string(),
                args,
                child_workflow_only: false,
            },
        );

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
        let command = WorkflowCommand::RequestCancelExternalWorkflow(
            crate::commands::RequestCancelExternalWorkflowCommand {
                cancellation_id,
                domain: None, // TODO: support domain
                workflow_id: workflow_id.to_string(),
                run_id: run_id.map(|s| s.to_string()),
                child_workflow_only: false,
            },
        );

        let _ = self.command_sink.submit(command).await?;
        Ok(())
    }

    /// Execute a side effect (non-deterministic operation)
    ///
    /// Side effects are operations that are not deterministic and should only be
    /// executed once. The result is cached in workflow history and replayed
    /// during workflow replay to ensure determinism.
    ///
    /// # Example
    /// ```rust,ignore
    /// let uuid = ctx.side_effect(|| uuid::Uuid::new_v4().to_string()).await;
    /// ```
    pub async fn side_effect<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
        R: Serialize + serde::de::DeserializeOwned,
    {
        use crate::side_effect_serialization::encode_side_effect_details;

        let side_effect_id = self.next_sequence_id();

        // Check if we're in replay mode
        let is_replay = self.is_replay.load(Ordering::SeqCst);

        if is_replay {
            // During replay: retrieve cached result
            let cache = self.side_effect_results.lock().unwrap();
            if let Some(encoded_result) = cache.get(&side_effect_id) {
                // Deserialize result
                let result: R = match serde_json::from_slice(encoded_result) {
                    Ok(r) => r,
                    Err(e) => {
                        panic!(
                            "Failed to deserialize side effect result for id={}: {}",
                            side_effect_id, e
                        );
                    }
                };
                return result;
            } else {
                panic!(
                    "Side effect id={} not found during replay. This indicates non-deterministic workflow code.",
                    side_effect_id
                );
            }
        }

        // During execution: run the side effect
        let result = f();

        // Serialize result
        let encoded_result = match serde_json::to_vec(&result) {
            Ok(bytes) => bytes,
            Err(e) => {
                panic!(
                    "Failed to serialize side effect result for id={}: {}",
                    side_effect_id, e
                );
            }
        };

        // Encode (side_effect_id, result) for marker
        let details = encode_side_effect_details(side_effect_id, &encoded_result);

        // Record marker in history
        let command = RecordMarkerCommand {
            marker_name: SIDE_EFFECT_MARKER_NAME.to_string(),
            details,
            header: None,
        };

        // Store in cache for potential future use
        {
            let mut cache = self.side_effect_results.lock().unwrap();
            cache.insert(side_effect_id, encoded_result);
        }

        // Send command to record marker
        let _ = self
            .command_sink
            .submit(WorkflowCommand::RecordMarker(command))
            .await;

        result
    }

    /// Execute a mutable side effect (cached side effect with value comparison)
    ///
    /// Mutable side effects are similar to side effects but allow the value to change
    /// over time. A new marker is only recorded when the value actually changes.
    /// During replay, the cached value is returned.
    ///
    /// # Example
    /// ```rust,ignore
    /// let counter = ctx.mutable_side_effect("counter", || 0u32, None).await;
    /// ```
    pub async fn mutable_side_effect<F, R, Eq>(&self, id: &str, f: F, equals: Option<Eq>) -> R
    where
        F: FnOnce() -> R,
        R: Serialize + serde::de::DeserializeOwned + Clone,
        Eq: Fn(&R, &R) -> bool,
    {
        use crate::side_effect_serialization::encode_mutable_side_effect_details;

        let is_replay = self.is_replay.load(Ordering::SeqCst);

        // Check cache for existing result
        let cache = self.mutable_side_effects.lock().unwrap();
        if let Some(encoded_result) = cache.get(id) {
            let cached_value: R = serde_json::from_slice(encoded_result)
                .expect("Failed to deserialize mutable side effect result");

            if is_replay {
                // During replay: just return cached value
                return cached_value;
            }

            // During execution: check if value changed
            let new_value = f();

            let is_equal = if let Some(eq_fn) = equals {
                eq_fn(&cached_value, &new_value)
            } else {
                // Default comparison using serialization
                let new_encoded = serde_json::to_vec(&new_value).unwrap();
                encoded_result == &new_encoded
            };

            if is_equal {
                // Value unchanged, don't record new marker
                return cached_value;
            }

            // Value changed, record new marker and update cache
            drop(cache); // Release lock
            let new_encoded = serde_json::to_vec(&new_value).unwrap();

            let details = encode_mutable_side_effect_details(id, &new_encoded);
            let command = RecordMarkerCommand {
                marker_name: MUTABLE_SIDE_EFFECT_MARKER_NAME.to_string(),
                details,
                header: None,
            };

            // Update cache
            {
                let mut cache = self.mutable_side_effects.lock().unwrap();
                cache.insert(id.to_string(), new_encoded);
            }

            // Send command to record marker
            let _ = self
                .command_sink
                .submit(WorkflowCommand::RecordMarker(command))
                .await;

            return new_value;
        }

        drop(cache);

        if is_replay {
            panic!(
                "Mutable side effect id='{}' not found during replay. This indicates non-deterministic workflow code.",
                id
            );
        }

        // First execution: record initial value
        let result = f();
        let encoded_result = serde_json::to_vec(&result).unwrap();
        let details = encode_mutable_side_effect_details(id, &encoded_result);

        let command = RecordMarkerCommand {
            marker_name: MUTABLE_SIDE_EFFECT_MARKER_NAME.to_string(),
            details,
            header: None,
        };

        // Update cache
        {
            let mut cache = self.mutable_side_effects.lock().unwrap();
            cache.insert(id.to_string(), encoded_result);
        }

        // Send command to record marker
        let _ = self
            .command_sink
            .submit(WorkflowCommand::RecordMarker(command))
            .await;

        result
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
        let command = WorkflowCommand::StartTimer(StartTimerCommand { timer_id, duration });
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
        let command = WorkflowCommand::StartTimer(StartTimerCommand { timer_id, duration });
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
        let command =
            WorkflowCommand::ContinueAsNewWorkflow(crate::commands::ContinueAsNewWorkflowCommand {
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
        })
        .await
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
        })
        .await
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
