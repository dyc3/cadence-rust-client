//! Workflow context and core functions for authoring workflows.
//!
//! This module provides the main API for implementing workflows including
//! scheduling activities, child workflows, handling signals, and more.

use crate::commands::{
    RecordMarkerCommand, ScheduleActivityCommand, ScheduleLocalActivityCommand,
    StartChildWorkflowCommand, StartTimerCommand, WorkflowCommand,
};
use crate::local_activity::LocalActivityMarkerData;
use crabdance_core::{
    ActivityOptions, ChildWorkflowOptions, DataConverter, JsonDataConverter, PropagationContext,
    RetryPolicy, WorkflowInfo,
};
use futures::future::poll_fn;
use serde::Serialize;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;
use tracing::debug;

// Type aliases to reduce complexity
type PendingSpawnTasks = Arc<Mutex<Option<Arc<Mutex<Vec<WorkflowTask>>>>>>;
type CompletedResults = Arc<Mutex<Option<Arc<Mutex<HashMap<u64, Box<dyn Any + Send>>>>>>>;

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

use crate::channel::{channel, Receiver, Sender};
use crate::dispatcher::{WorkflowDispatcher, WorkflowTask};
use crabdance_core::ResourceContext;

/// Marker names for side effects
pub const SIDE_EFFECT_MARKER_NAME: &str = "SideEffect";
pub const MUTABLE_SIDE_EFFECT_MARKER_NAME: &str = "MutableSideEffect";
pub const VERSION_MARKER_NAME: &str = "Version";

/// Default version constant (-1) used when no version is specified
pub const DEFAULT_VERSION: i32 = -1;

/// Search-attribute key under which `get_version` records the change/version pairs,
/// matching the Go client's `CadenceChangeVersion`.
pub const CADENCE_CHANGE_VERSION_SEARCH_ATTRIBUTE: &str = "CadenceChangeVersion";

/// First-execution options for [`WorkflowContext::get_version_with_options`],
/// mirroring Go's `GetVersionOption` (`ExecuteWithVersion` / `ExecuteWithMinVersion`).
///
/// These only affect the version chosen the *first* time a changeID is seen during
/// live execution; cached and replayed versions are unaffected.
#[derive(Debug, Clone, Default)]
pub struct GetVersionOptions {
    custom_version: Option<i32>,
    use_min_version: bool,
}

impl GetVersionOptions {
    /// Force a specific version on first execution (Go's `ExecuteWithVersion`).
    pub fn execute_with_version(version: i32) -> Self {
        Self {
            custom_version: Some(version),
            use_min_version: false,
        }
    }

    /// Force `min_supported` on first execution (Go's `ExecuteWithMinVersion`).
    pub fn execute_with_min_version() -> Self {
        Self {
            custom_version: None,
            use_min_version: true,
        }
    }
}

/// Type alias for query handlers
pub type QueryHandler = Box<dyn Fn(Vec<u8>) -> Vec<u8> + Send + Sync>;

/// Builder for WorkflowContext creation with command sink
pub struct WorkflowContextBuilder {
    workflow_info: WorkflowInfo,
    sink: Arc<dyn CommandSink>,
    signals: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
    query_handlers: Arc<Mutex<HashMap<String, QueryHandler>>>,
    side_effect_results: Arc<Mutex<HashMap<u64, Vec<u8>>>>,
    mutable_side_effects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    change_versions: Arc<Mutex<HashMap<String, i32>>>,
    local_activity_results: Arc<Mutex<HashMap<String, LocalActivityMarkerData>>>,
}

impl WorkflowContextBuilder {
    pub fn new(workflow_info: WorkflowInfo, sink: Arc<dyn CommandSink>) -> Self {
        Self {
            workflow_info,
            sink,
            signals: Arc::new(Mutex::new(HashMap::new())),
            query_handlers: Arc::new(Mutex::new(HashMap::new())),
            side_effect_results: Arc::new(Mutex::new(HashMap::new())),
            mutable_side_effects: Arc::new(Mutex::new(HashMap::new())),
            change_versions: Arc::new(Mutex::new(HashMap::new())),
            local_activity_results: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn signals(mut self, signals: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>) -> Self {
        self.signals = signals;
        self
    }

    pub fn query_handlers(
        mut self,
        query_handlers: Arc<Mutex<HashMap<String, QueryHandler>>>,
    ) -> Self {
        self.query_handlers = query_handlers;
        self
    }

    pub fn side_effect_results(
        mut self,
        side_effect_results: Arc<Mutex<HashMap<u64, Vec<u8>>>>,
    ) -> Self {
        self.side_effect_results = side_effect_results;
        self
    }

    pub fn mutable_side_effects(
        mut self,
        mutable_side_effects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    ) -> Self {
        self.mutable_side_effects = mutable_side_effects;
        self
    }

    pub fn change_versions(mut self, change_versions: Arc<Mutex<HashMap<String, i32>>>) -> Self {
        self.change_versions = change_versions;
        self
    }

    pub fn local_activity_results(
        mut self,
        local_activity_results: Arc<Mutex<HashMap<String, LocalActivityMarkerData>>>,
    ) -> Self {
        self.local_activity_results = local_activity_results;
        self
    }

    pub fn build(self) -> WorkflowContext {
        WorkflowContext::with_sink(
            self.workflow_info,
            self.sink,
            self.signals,
            self.query_handlers,
            self.side_effect_results,
            self.mutable_side_effects,
            self.change_versions,
            self.local_activity_results,
            None,
        )
    }
}

/// Trait for handling workflow commands (implemented by worker)
pub trait CommandSink: Send + Sync {
    fn submit(
        &self,
        command: WorkflowCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, DefaultWorkflowError>> + Send>>;

    /// Submit a fire-and-forget command synchronously and in order, without yielding
    /// to the async runtime.
    ///
    /// This exists for markers recorded from *synchronous* workflow APIs such as
    /// [`WorkflowContext::get_version`], where spawning a detached `tokio` task would
    /// escape the deterministic dispatcher (the marker could be recorded out of order,
    /// late, or not at all — a replay-corrupting bug). Marker submissions resolve
    /// immediately, so the default implementation drives the returned future with a
    /// single no-op poll, recording the command in place before returning.
    fn submit_now(&self, command: WorkflowCommand) {
        let mut future = self.submit(command);
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);
        // Markers complete on the first poll; we deliberately do not loop, since a
        // Pending result would mean a non-marker (genuinely async) command was routed
        // through the synchronous path.
        let _ = future.as_mut().poll(&mut cx);
    }
}

/// No-op command sink for testing/initialization
struct NoopCommandSink;
impl CommandSink for NoopCommandSink {
    fn submit(
        &self,
        _command: WorkflowCommand,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, DefaultWorkflowError>> + Send>> {
        Box::pin(async { Err(WorkflowError::message("No command sink configured")) })
    }
}

/// Workflow context for executing workflow logic
#[derive(Clone)]
pub struct WorkflowContext {
    workflow_info: WorkflowInfo,
    command_sink: Arc<dyn CommandSink>,
    sequence: Arc<AtomicU64>,
    signals: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
    query_handlers: Arc<Mutex<HashMap<String, QueryHandler>>>,
    cancelled: Arc<AtomicBool>,
    // Side effect result caches for replay
    side_effect_results: Arc<Mutex<HashMap<u64, Vec<u8>>>>,
    mutable_side_effects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    // Replay flag - true when workflow is being replayed from history
    is_replay: Arc<AtomicBool>,
    // When false (default), workflow logs are suppressed during replay (Go's
    // EnableLoggingInReplay). The worker sets this from its options.
    log_in_replay: Arc<AtomicBool>,
    // Deterministic time - current time in nanoseconds (unix epoch)
    current_time_nanos: Arc<AtomicI64>,
    // Version markers cache for workflow versioning
    change_versions: Arc<Mutex<HashMap<String, i32>>>,
    // Search attributes, seeded from WorkflowInfo and updated by upsert_search_attributes.
    search_attributes: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    // Cron: result of the previous run, if this workflow was started by a cron schedule.
    last_completion_result: Arc<Mutex<Option<Vec<u8>>>>,
    // Propagated trace context / baggage, extracted from the workflow's start header by
    // the worker (see ContextPropagator). Available to workflow code and injectable into
    // outbound activity/child headers.
    propagation_context: Arc<Mutex<PropagationContext>>,
    // History size accounting for the current decision task (set by the worker).
    history_count: Arc<AtomicU64>,
    total_history_bytes: Arc<AtomicU64>,
    // Local activity results cache for replay
    local_activity_results: Arc<Mutex<HashMap<String, LocalActivityMarkerData>>>,
    // Dispatcher for managing spawned tasks
    dispatcher: Arc<Mutex<Option<Arc<Mutex<WorkflowDispatcher>>>>>,
    // Pending tasks queue (shared with dispatcher for lock-free spawning)
    pending_spawn_tasks: PendingSpawnTasks,
    // Completed results (shared with dispatcher for lock-free join)
    completed_results: CompletedResults,
    // Task sequence counter
    task_sequence: Arc<AtomicU64>,
    // Channel sequence counter
    channel_sequence: Arc<AtomicU64>,
    resources: Option<Arc<dyn Any + Send + Sync>>,
    // Configured payload converter (the worker injects its own; defaults to JSON)
    converter: Arc<dyn DataConverter>,
}

/// Build the initial search-attribute cache from a workflow's `WorkflowInfo`.
fn seed_search_attributes(workflow_info: &WorkflowInfo) -> HashMap<String, Vec<u8>> {
    workflow_info.search_attributes.clone().unwrap_or_default()
}

impl WorkflowContext {
    pub fn new(workflow_info: WorkflowInfo) -> Self {
        // Initialize with start time from workflow info
        let start_time_nanos = workflow_info.start_time.timestamp_nanos_opt().unwrap_or(0);
        let search_attributes = seed_search_attributes(&workflow_info);

        Self {
            workflow_info,
            command_sink: Arc::new(NoopCommandSink),
            sequence: Arc::new(AtomicU64::new(0)),
            signals: Arc::new(Mutex::new(HashMap::new())),
            query_handlers: Arc::new(Mutex::new(HashMap::new())),
            cancelled: Arc::new(AtomicBool::new(false)),
            side_effect_results: Arc::new(Mutex::new(HashMap::new())),
            mutable_side_effects: Arc::new(Mutex::new(HashMap::new())),
            is_replay: Arc::new(AtomicBool::new(false)),
            log_in_replay: Arc::new(AtomicBool::new(false)),
            current_time_nanos: Arc::new(AtomicI64::new(start_time_nanos)),
            change_versions: Arc::new(Mutex::new(HashMap::new())),
            search_attributes: Arc::new(Mutex::new(search_attributes)),
            last_completion_result: Arc::new(Mutex::new(None)),
            propagation_context: Arc::new(Mutex::new(PropagationContext::new())),
            history_count: Arc::new(AtomicU64::new(0)),
            total_history_bytes: Arc::new(AtomicU64::new(0)),
            local_activity_results: Arc::new(Mutex::new(HashMap::new())),
            dispatcher: Arc::new(Mutex::new(None)),
            pending_spawn_tasks: Arc::new(Mutex::new(None)),
            completed_results: Arc::new(Mutex::new(None)),
            // Start task_sequence at 1 because root workflow task is ID 0
            task_sequence: Arc::new(AtomicU64::new(1)),
            channel_sequence: Arc::new(AtomicU64::new(0)),
            resources: None,
            converter: Arc::new(JsonDataConverter),
        }
    }

    #[expect(clippy::too_many_arguments)]
    pub fn with_sink(
        workflow_info: WorkflowInfo,
        sink: Arc<dyn CommandSink>,
        signals: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
        query_handlers: Arc<Mutex<HashMap<String, QueryHandler>>>,
        side_effect_results: Arc<Mutex<HashMap<u64, Vec<u8>>>>,
        mutable_side_effects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
        change_versions: Arc<Mutex<HashMap<String, i32>>>,
        local_activity_results: Arc<Mutex<HashMap<String, LocalActivityMarkerData>>>,
        resources: Option<Arc<dyn Any + Send + Sync>>,
    ) -> Self {
        // Initialize with start time from workflow info
        let start_time_nanos = workflow_info.start_time.timestamp_nanos_opt().unwrap_or(0);
        let search_attributes = seed_search_attributes(&workflow_info);

        Self {
            workflow_info,
            command_sink: sink,
            sequence: Arc::new(AtomicU64::new(0)),
            signals,
            query_handlers,
            cancelled: Arc::new(AtomicBool::new(false)),
            side_effect_results,
            mutable_side_effects,
            is_replay: Arc::new(AtomicBool::new(false)),
            log_in_replay: Arc::new(AtomicBool::new(false)),
            current_time_nanos: Arc::new(AtomicI64::new(start_time_nanos)),
            change_versions,
            search_attributes: Arc::new(Mutex::new(search_attributes)),
            last_completion_result: Arc::new(Mutex::new(None)),
            propagation_context: Arc::new(Mutex::new(PropagationContext::new())),
            history_count: Arc::new(AtomicU64::new(0)),
            total_history_bytes: Arc::new(AtomicU64::new(0)),
            local_activity_results,
            dispatcher: Arc::new(Mutex::new(None)),
            pending_spawn_tasks: Arc::new(Mutex::new(None)),
            completed_results: Arc::new(Mutex::new(None)),
            // Start task_sequence at 1 because root workflow task is ID 0
            task_sequence: Arc::new(AtomicU64::new(1)),
            channel_sequence: Arc::new(AtomicU64::new(0)),
            resources,
            converter: Arc::new(JsonDataConverter),
        }
    }

    pub fn with_resources(mut self, resources: Option<Arc<dyn Any + Send + Sync>>) -> Self {
        self.resources = resources;
        self
    }

    /// Inject the configured payload converter (the worker calls this to thread
    /// its `DataConverter` into workflow code; defaults to JSON otherwise).
    pub fn with_converter(mut self, converter: Arc<dyn DataConverter>) -> Self {
        self.converter = converter;
        self
    }

    /// Borrow the configured payload converter.
    pub fn converter(&self) -> &dyn DataConverter {
        self.converter.as_ref()
    }

    /// Clone the configured payload converter. Used by the `#[workflow]` /
    /// `call_activity!` macros to encode/decode payloads through the seam.
    pub fn converter_arc(&self) -> Arc<dyn DataConverter> {
        self.converter.clone()
    }

    /// Set replay mode
    pub fn set_replay_mode(&self, is_replay: bool) {
        self.is_replay.store(is_replay, Ordering::SeqCst);
    }

    /// Set current workflow time (from deterministic source)
    pub fn set_current_time_nanos(&self, time_nanos: i64) {
        self.current_time_nanos.store(time_nanos, Ordering::SeqCst);
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

    /// Set change versions cache (used during replay)
    pub fn set_change_versions(&self, versions: HashMap<String, i32>) {
        let mut cache = self.change_versions.lock().unwrap();
        *cache = versions;
    }

    /// Set local activity results cache (used during replay)
    pub fn set_local_activity_results(&self, results: HashMap<String, LocalActivityMarkerData>) {
        let mut cache = self.local_activity_results.lock().unwrap();
        *cache = results;
    }

    /// Get local activity result from cache (used during replay)
    fn get_local_activity_result(&self, activity_id: &str) -> Option<LocalActivityMarkerData> {
        let cache = self.local_activity_results.lock().unwrap();
        cache.get(activity_id).cloned()
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
    ///
    /// You probably want to use the [`crabdance_macros::call_activity`] macro instead of calling this directly, as it saves a lot of boilerplate around serialization and type safety.
    pub async fn execute_activity(
        &self,
        activity_type: &str,
        args: Option<Vec<u8>>,
        options: ActivityOptions,
    ) -> Result<Vec<u8>, DefaultWorkflowError> {
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
        activity_type: &str,
        args: Option<Vec<u8>>,
        options: LocalActivityOptions,
    ) -> Result<Vec<u8>, DefaultWorkflowError> {
        let activity_id = self.next_id();

        // Check if replay mode - return cached result if available
        if self.is_replay.load(Ordering::SeqCst) {
            if let Some(marker_data) = self.get_local_activity_result(&activity_id) {
                return crate::local_activity::marker_data_to_result(marker_data);
            }
            // If not in cache during replay, this is a non-determinism error
            return Err(WorkflowError::message(format!(
                "Local activity {} not found during replay - non-deterministic workflow code",
                activity_id
            )));
        }

        // Execution mode - schedule local activity
        let command = WorkflowCommand::ScheduleLocalActivity(ScheduleLocalActivityCommand {
            activity_id: activity_id.clone(),
            activity_type: activity_type.to_string(),
            args,
            options,
        });

        // Submit and wait for result
        self.command_sink.submit(command).await
    }

    /// Execute a child workflow
    pub async fn execute_child_workflow(
        &self,
        workflow_type: &str,
        args: Option<Vec<u8>>,
        options: ChildWorkflowOptions,
    ) -> Result<Vec<u8>, DefaultWorkflowError> {
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

    /// Deliver a signal to the workflow (used by the worker on signal events and by the
    /// test environment to inject signals before a run).
    pub fn add_signal(&self, signal_name: &str, payload: Vec<u8>) {
        self.signals
            .lock()
            .unwrap()
            .entry(signal_name.to_string())
            .or_default()
            .push(payload);
    }

    /// Signal an external workflow
    pub async fn signal_external_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        signal_name: &str,
        args: Option<Vec<u8>>,
    ) -> Result<(), DefaultWorkflowError> {
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
    ) -> Result<(), DefaultWorkflowError> {
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
        let cached_result = {
            let cache = self.mutable_side_effects.lock().unwrap();
            cache.get(id).cloned()
        };

        if let Some(encoded_result) = cached_result {
            let cached_value: R = serde_json::from_slice(&encoded_result)
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
                encoded_result == new_encoded
            };

            if is_equal {
                // Value unchanged, don't record new marker
                return cached_value;
            }

            // Value changed, record new marker and update cache
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
    ///
    /// This function implements workflow versioning to allow safe, backwards-compatible
    /// changes to workflow definitions. It returns a version number that determines
    /// which code path to execute.
    ///
    /// # Arguments
    /// * `change_id` - Unique identifier for this versioning change (e.g., "add-new-step-v2")
    /// * `min_supported` - Minimum version supported by current code
    /// * `max_supported` - Maximum version supported by current code
    ///
    /// # Behavior
    /// - On first execution: Returns `max_supported` (or DEFAULT_VERSION if in replay)
    /// - On replay: Returns cached version from history
    /// - Version is validated against [min_supported, max_supported] range
    /// - DEFAULT_VERSION (-1) is never recorded as a marker
    /// - Version is stable for the entire workflow execution per changeID
    ///
    /// # Panics
    /// Panics if:
    /// - Version is less than min_supported (code removed support)
    /// - Version is greater than max_supported (workflow history too new)
    ///
    /// # Example
    /// ```rust,ignore
    /// let version = ctx.get_version("add-approval-step", DEFAULT_VERSION, 2);
    /// if version >= 1 {
    ///     // New code path with approval step
    ///     ctx.execute_activity("approval", ...).await?;
    /// }
    /// // Original code continues...
    /// ```
    pub fn get_version(&self, change_id: &str, min_supported: i32, max_supported: i32) -> i32 {
        self.get_version_with_options(
            change_id,
            min_supported,
            max_supported,
            GetVersionOptions::default(),
        )
    }

    /// [`get_version`](Self::get_version) with explicit first-execution options.
    ///
    /// Mirrors Go's `GetVersion(ctx, changeID, min, max, opts...)`:
    /// - cached changeID → the recorded version (validated against the range);
    /// - replay (uncached) → `DEFAULT_VERSION`;
    /// - [`GetVersionOptions::execute_with_version`] → the supplied version;
    /// - [`GetVersionOptions::execute_with_min_version`] → `min_supported`;
    /// - otherwise → `max_supported`.
    ///
    /// On first execution (not replay) of a non-default version, the version marker is
    /// recorded through the deterministic command pipeline and the `CadenceChangeVersion`
    /// search attribute is upserted, matching Go.
    pub fn get_version_with_options(
        &self,
        change_id: &str,
        min_supported: i32,
        max_supported: i32,
        options: GetVersionOptions,
    ) -> i32 {
        // Step 1: a version already assigned to this changeID is stable for the run.
        {
            let versions = self.change_versions.lock().unwrap();
            if let Some(&version) = versions.get(change_id) {
                Self::validate_version(change_id, version, min_supported, max_supported);
                return version;
            }
        }

        // Step 2: choose the version for this first execution.
        let is_replay = self.is_replay.load(Ordering::SeqCst);
        let version = if is_replay {
            // The actual version is loaded from history markers (seeded into the cache);
            // an uncached changeID during replay predates this change → DEFAULT_VERSION.
            DEFAULT_VERSION
        } else if let Some(custom) = options.custom_version {
            custom
        } else if options.use_min_version {
            min_supported
        } else {
            max_supported
        };

        // Step 3: validate the chosen version is in range.
        Self::validate_version(change_id, version, min_supported, max_supported);

        // Step 4: on first non-replay execution of a non-default version, record the
        // marker (synchronously and in order via submit_now — not a detached spawn) and
        // upsert the CadenceChangeVersion search attribute, matching Go.
        if !is_replay && version != DEFAULT_VERSION {
            use crate::side_effect_serialization::encode_version_details;

            let details = encode_version_details(change_id, version);
            let command = RecordMarkerCommand {
                marker_name: VERSION_MARKER_NAME.to_string(),
                details,
                header: None,
            };
            self.command_sink
                .submit_now(WorkflowCommand::RecordMarker(command));

            self.upsert_change_version_search_attribute(change_id, version);
        }

        // Step 5: record the version so it is stable for the rest of the run.
        {
            let mut versions = self.change_versions.lock().unwrap();
            versions.insert(change_id.to_string(), version);
        }

        version
    }

    /// Upsert the `CadenceChangeVersion` search attribute (a JSON array of
    /// `"<changeID>-<version>"` entries for this and all prior changes), matching Go.
    fn upsert_change_version_search_attribute(&self, change_id: &str, version: i32) {
        let mut entries = vec![format!("{change_id}-{version}")];
        {
            let versions = self.change_versions.lock().unwrap();
            for (existing_id, existing_version) in versions.iter() {
                // Skip the change being recorded (added above) and any default-version
                // entries, which are not meaningful recorded versions.
                if existing_id == change_id || *existing_version == DEFAULT_VERSION {
                    continue;
                }
                entries.push(format!("{existing_id}-{existing_version}"));
            }
        }
        // Sort for a deterministic payload — HashMap iteration order is unspecified,
        // and this attribute is recorded through the deterministic command pipeline.
        entries.sort();
        if let Ok(encoded) = serde_json::to_vec(&entries) {
            self.upsert_search_attributes(vec![(
                CADENCE_CHANGE_VERSION_SEARCH_ATTRIBUTE.to_string(),
                encoded,
            )]);
        }
    }

    /// Validate version is within supported range
    ///
    /// # Panics
    /// Panics if version is outside [min_supported, max_supported] range
    fn validate_version(change_id: &str, version: i32, min_supported: i32, max_supported: i32) {
        if version < min_supported {
            panic!(
                "Workflow code removed support of version {} for '{}' changeID. \
                 The oldest supported version is {}",
                version, change_id, min_supported
            );
        }
        if version > max_supported {
            panic!(
                "Workflow code is too old to support version {} for '{}' changeID. \
                 The maximum supported version is {}",
                version, change_id, max_supported
            );
        }
    }

    /// Set a query handler
    pub fn set_query_handler<F>(&self, query_type: &str, handler: F)
    where
        F: Fn(Vec<u8>) -> Vec<u8> + Send + Sync + 'static,
    {
        let mut handlers = self.query_handlers.lock().unwrap();
        handlers.insert(query_type.to_string(), Box::new(handler));
    }

    /// Upsert (insert or update) workflow search attributes.
    ///
    /// The attributes are merged into the workflow's search-attribute set and, during
    /// execution, an `UpsertWorkflowSearchAttributes` decision is emitted through the
    /// deterministic command pipeline (synchronously and in order, like a marker — not
    /// a detached task). On replay the local set is updated from the seeded value and
    /// no decision is re-emitted, mirroring how versions and side effects replay.
    pub fn upsert_search_attributes(&self, search_attributes: Vec<(String, Vec<u8>)>) {
        if search_attributes.is_empty() {
            return;
        }

        // Merge into the local set so get_search_attributes reflects the upsert
        // immediately, on both the execution and replay paths.
        {
            let mut current = self.search_attributes.lock().unwrap();
            for (key, value) in &search_attributes {
                current.insert(key.clone(), value.clone());
            }
        }

        // Emit the decision only during execution; on replay it is already in history.
        if !self.is_replay.load(Ordering::SeqCst) {
            let command = crate::commands::UpsertSearchAttributesCommand { search_attributes };
            self.command_sink
                .submit_now(WorkflowCommand::UpsertSearchAttributes(command));
        }
    }

    /// Get the workflow's current search attributes (initial set merged with any
    /// upserts performed during the workflow).
    pub fn get_search_attributes(&self) -> HashMap<String, Vec<u8>> {
        self.search_attributes.lock().unwrap().clone()
    }

    /// Whether the workflow is currently replaying history (Go's `IsReplaying`).
    ///
    /// Use this to suppress non-replay-safe side effects (e.g. external logging) when
    /// the dispatcher is re-deriving state from history rather than executing live.
    pub fn is_replaying(&self) -> bool {
        self.is_replay.load(Ordering::SeqCst)
    }

    /// Whether this run has a previous-run result available (cron workflows).
    pub fn has_last_completion_result(&self) -> bool {
        self.last_completion_result.lock().unwrap().is_some()
    }

    /// The encoded result of the previous run, if this workflow was started by a cron
    /// schedule and the previous run completed successfully (Go's `LastCompletionResult`).
    pub fn get_last_completion_result(&self) -> Option<Vec<u8>> {
        self.last_completion_result.lock().unwrap().clone()
    }

    /// Set the previous-run result (called by the worker from the started event).
    pub fn set_last_completion_result(&self, result: Option<Vec<u8>>) {
        *self.last_completion_result.lock().unwrap() = result;
    }

    /// Number of events in the workflow's history for the current decision task.
    pub fn get_history_count(&self) -> u64 {
        self.history_count.load(Ordering::SeqCst)
    }

    /// Approximate size, in bytes, of the workflow's history for the current decision
    /// task. This is a best-effort serialized size (the server-tracked figure is not
    /// surfaced on the poll path); use it for relative growth signals, not exact limits.
    pub fn get_total_history_bytes(&self) -> u64 {
        self.total_history_bytes.load(Ordering::SeqCst)
    }

    /// Set the history-size accounting (called by the worker per decision task).
    pub fn set_history_size(&self, count: u64, total_bytes: u64) {
        self.history_count.store(count, Ordering::SeqCst);
        self.total_history_bytes
            .store(total_bytes, Ordering::SeqCst);
    }

    /// Wrap a single encoded payload in an [`EncodedValue`](crabdance_core::EncodedValue)
    /// for lazy, typed decoding (Go's `workflow.NewValue`).
    pub fn new_value(&self, data: Vec<u8>) -> crabdance_core::EncodedValue {
        crabdance_core::EncodedValue::new(data)
    }

    /// Wrap an encoded multi-value payload in an
    /// [`EncodedValues`](crabdance_core::EncodedValues) (Go's `workflow.NewValues`).
    pub fn new_values(&self, data: Vec<u8>) -> crabdance_core::EncodedValues {
        crabdance_core::EncodedValues::new(data)
    }

    /// Set the propagated trace context / baggage, called by the worker after extracting
    /// it from the workflow's start header through the configured `ContextPropagator`s.
    pub fn set_propagation_context(&self, context: PropagationContext) {
        *self.propagation_context.lock().unwrap() = context;
    }

    /// The propagated trace context / baggage available to this workflow (Go's header
    /// values exposed to workflow code).
    pub fn propagation_context(&self) -> PropagationContext {
        self.propagation_context.lock().unwrap().clone()
    }

    /// Build a header carrier for an outbound boundary (scheduling an activity or child
    /// workflow) by injecting the current propagation context through `propagators`.
    pub fn inject_propagation(
        &self,
        propagators: &[Arc<dyn crabdance_core::ContextPropagator>],
    ) -> crabdance_core::PropagationCarrier {
        let context = self.propagation_context.lock().unwrap();
        let mut carrier = crabdance_core::PropagationCarrier::new();
        for propagator in propagators {
            propagator.inject(&context, &mut carrier);
        }
        carrier
    }

    /// Sleep for a duration (workflow-aware)
    pub async fn sleep(&self, duration: Duration) {
        let timer_id = self.next_id();
        let command = WorkflowCommand::StartTimer(StartTimerCommand { timer_id, duration });
        let _ = self.command_sink.submit(command).await;
    }

    /// Get current workflow time (deterministic)
    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        let nanos = self.current_time_nanos.load(Ordering::SeqCst);
        chrono::DateTime::from_timestamp(nanos / 1_000_000_000, (nanos % 1_000_000_000) as u32)
            .unwrap_or_else(chrono::Utc::now)
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

    /// Enable or disable workflow log emission during replay (Go's
    /// `EnableLoggingInReplay`). Disabled by default, so replayed history does not
    /// re-emit logs. The worker sets this from its options.
    pub fn set_logging_enabled_in_replay(&self, enabled: bool) {
        self.log_in_replay.store(enabled, Ordering::SeqCst);
    }

    /// Whether a workflow log statement should be emitted right now: always during live
    /// execution, and during replay only when logging-in-replay is enabled.
    pub fn should_log(&self) -> bool {
        !self.is_replay.load(Ordering::SeqCst) || self.log_in_replay.load(Ordering::SeqCst)
    }

    /// Emit a workflow log event through `tracing`, suppressed during replay unless
    /// logging-in-replay is enabled. Events are tagged with the workflow id, run id and
    /// type so they can be correlated.
    ///
    /// Prefer these over calling `tracing` directly from workflow code, since they
    /// honor the replay-aware emission guard (the analogue of Go's injected logger).
    pub fn log_debug(&self, message: &str) {
        if self.should_log() {
            let exec = &self.workflow_info.workflow_execution;
            tracing::debug!(
                workflow_id = %exec.workflow_id,
                run_id = %exec.run_id,
                workflow_type = %self.workflow_info.workflow_type.name,
                "{message}"
            );
        }
    }

    /// See [`log_debug`](Self::log_debug).
    pub fn log_info(&self, message: &str) {
        if self.should_log() {
            let exec = &self.workflow_info.workflow_execution;
            tracing::info!(
                workflow_id = %exec.workflow_id,
                run_id = %exec.run_id,
                workflow_type = %self.workflow_info.workflow_type.name,
                "{message}"
            );
        }
    }

    /// See [`log_debug`](Self::log_debug).
    pub fn log_warn(&self, message: &str) {
        if self.should_log() {
            let exec = &self.workflow_info.workflow_execution;
            tracing::warn!(
                workflow_id = %exec.workflow_id,
                run_id = %exec.run_id,
                workflow_type = %self.workflow_info.workflow_type.name,
                "{message}"
            );
        }
    }

    /// See [`log_debug`](Self::log_debug).
    pub fn log_error(&self, message: &str) {
        if self.should_log() {
            let exec = &self.workflow_info.workflow_execution;
            tracing::error!(
                workflow_id = %exec.workflow_id,
                run_id = %exec.run_id,
                workflow_type = %self.workflow_info.workflow_type.name,
                "{message}"
            );
        }
    }

    // Metrics are native via the `metrics` facade. The worker emits parity metrics
    // (poller/task/panic/quota) directly; workflow code can emit its own with the
    // `metrics` crate, guarding with `should_log()` to honor replay-aware emission. The
    // previous no-op MetricsScope/Counter/Timer/Gauge traits were removed.

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

    /// Create a new channel for coordinating between spawned tasks
    ///
    /// # Arguments
    /// * `buffer_size` - Capacity of the channel buffer. Use 0 for unbuffered.
    ///
    /// # Returns
    /// A tuple of (Sender, Receiver)
    ///
    /// # Example
    /// ```rust,ignore
    /// let (tx, rx) = ctx.new_channel(10);
    /// ```
    pub fn new_channel<T>(&self, buffer_size: usize) -> (Sender<T>, Receiver<T>) {
        let _channel_id = self.channel_sequence.fetch_add(1, Ordering::SeqCst);
        channel(buffer_size)
    }

    /// Spawn a new task in the workflow
    ///
    /// The task will be executed cooperatively by the workflow dispatcher.
    /// Tasks are polled in creation order to ensure deterministic execution.
    ///
    /// # Arguments
    /// * `f` - The future to execute
    ///
    /// # Returns
    /// A JoinHandle that can be awaited to get the task result
    ///
    /// # Example
    /// ```rust,ignore
    /// let handle = ctx.spawn(async move {
    ///     ctx.execute_activity("process", args, options).await
    /// });
    ///
    /// let result = handle.join().await?;
    /// ```
    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let task_id = self.task_sequence.fetch_add(1, Ordering::SeqCst);

        debug!(task_id, "spawning workflow task");

        // Create the task
        let task = WorkflowTask::new(task_id, format!("task-{}", task_id), f);

        // Get completed_results Arc (for JoinHandle) - no dispatcher lock needed
        let completed_results_arc = {
            let completed_opt = self.completed_results.lock().unwrap();
            completed_opt
                .as_ref()
                .expect("Completed results not set on WorkflowContext. This is a bug.")
                .clone()
        };

        // Add task to pending queue WITHOUT locking the dispatcher
        // This avoids deadlock when spawning from within a task
        let pending_tasks_opt = self.pending_spawn_tasks.lock().unwrap();
        if let Some(pending_tasks_arc) = pending_tasks_opt.as_ref() {
            let mut pending = pending_tasks_arc.lock().unwrap();
            let queue_len = pending.len();
            pending.push(task);
            debug!(
                task_id,
                queue_len = queue_len + 1,
                "task added to pending queue"
            );
        } else {
            panic!("Pending tasks queue not set on WorkflowContext. This is a bug.");
        }

        JoinHandle {
            task_id,
            completed_results: completed_results_arc,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the dispatcher for this context (for workflow executor)
    pub fn set_dispatcher(&self, dispatcher: Arc<Mutex<WorkflowDispatcher>>) {
        // Get the dispatcher's pending_tasks and completed_results Arcs to share with context
        let (pending_tasks_arc, completed_results_arc) = {
            let disp = dispatcher.lock().unwrap();
            (disp.pending_tasks.clone(), disp.get_completed_results_arc())
        };

        // Store the dispatcher
        let mut disp_opt = self.dispatcher.lock().unwrap();
        *disp_opt = Some(dispatcher);

        // Share the dispatcher's pending_tasks Arc with context
        let mut pending_opt = self.pending_spawn_tasks.lock().unwrap();
        *pending_opt = Some(pending_tasks_arc);

        // Share the dispatcher's completed_results Arc with context
        let mut completed_opt = self.completed_results.lock().unwrap();
        *completed_opt = Some(completed_results_arc);
    }

    /// Get the dispatcher (for workflow executor)
    pub fn get_dispatcher(&self) -> Option<Arc<Mutex<WorkflowDispatcher>>> {
        self.dispatcher.lock().unwrap().clone()
    }

    pub fn resources<R: 'static>(&self) -> Option<&R> {
        self.resources
            .as_ref()
            .and_then(|resources| resources.downcast_ref::<R>())
    }

    pub fn resource_context(&self) -> Option<ResourceContext<'_>> {
        self.resources
            .as_ref()
            .map(|resources| ResourceContext::Workflow {
                resources: resources.as_ref(),
            })
    }
}

/// Local activity options
#[derive(Debug, Clone)]
pub struct LocalActivityOptions {
    pub schedule_to_close_timeout: Duration,
    pub retry_policy: Option<RetryPolicy>,
}

/// Handle for a spawned workflow task
pub struct JoinHandle<T> {
    task_id: u64,
    completed_results: Arc<Mutex<HashMap<u64, Box<dyn Any + Send>>>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + 'static> JoinHandle<T> {
    /// Wait for the task to complete and get its result
    ///
    /// In the workflow execution model, this will poll until the task completes.
    /// The actual execution happens in execute_until_all_blocked().
    pub async fn join(self) -> Result<T, JoinError> {
        debug!(task_id = self.task_id, "waiting for task to complete");
        // Poll until task is complete
        poll_fn(|_cx| {
            let results = self.completed_results.lock().unwrap();
            if results.contains_key(&self.task_id) {
                debug!(task_id = self.task_id, "task is complete");
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;

        // Get the result
        debug!(task_id = self.task_id, "getting task result");
        let mut results = self.completed_results.lock().unwrap();
        let results_count = results.len();
        debug!(results_count, "locked results");
        let result = results
            .remove(&self.task_id)
            .ok_or(JoinError::TaskNotComplete(self.task_id))?;
        let remaining_count = results.len();
        debug!(
            task_id = self.task_id,
            remaining_count, "removed task result"
        );

        // Downcast to the expected type
        result
            .downcast::<T>()
            .map(|boxed| *boxed)
            .map_err(|_| JoinError::TaskCancelled(self.task_id))
    }
}

/// Error returned when joining a task fails
#[derive(Debug, Clone)]
pub enum JoinError {
    /// Task has not completed yet
    TaskNotComplete(u64),
    /// Task was cancelled
    TaskCancelled(u64),
}

impl std::fmt::Display for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinError::TaskNotComplete(id) => write!(f, "task {} not complete", id),
            JoinError::TaskCancelled(id) => write!(f, "task {} cancelled", id),
        }
    }
}

impl std::error::Error for JoinError {}

/// Continue as new options
#[derive(Debug, Clone, Default)]
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
    cancelled: Arc<AtomicBool>,
}

impl CancellationChannel {
    pub fn new(cancelled: Arc<AtomicBool>) -> Self {
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
pub use crate::future::{
    ActivityError, DefaultActivityError, DefaultWorkflowError, TimerFuture, WorkflowError,
};

// Workflow logging is native via `tracing` — see WorkflowContext::log_info etc., which
// emit through `tracing` with a replay-aware guard. The previous `Logger`/`ConsoleLogger`
// trait stubs were removed in favor of the ecosystem-standard facade.

// Metrics are emitted natively through the `metrics` facade (see crabdance_worker's
// `metrics` module for the parity metric names). The previous MetricsScope/Counter/
// Timer/Gauge trait stubs were removed in favor of the ecosystem-standard facade.

#[cfg(test)]
mod tests {
    use super::*;
    use crabdance_core::{WorkflowExecution, WorkflowType};

    fn create_test_workflow_info() -> WorkflowInfo {
        WorkflowInfo {
            workflow_execution: WorkflowExecution {
                workflow_id: "test-workflow-123".to_string(),
                run_id: "test-run-456".to_string(),
            },
            workflow_type: WorkflowType {
                name: "TestWorkflow".to_string(),
            },
            task_list: "test-task-list".to_string(),
            start_time: chrono::Utc::now(),
            execution_start_to_close_timeout: std::time::Duration::from_secs(3600),
            task_start_to_close_timeout: std::time::Duration::from_secs(10),
            attempt: 1,
            continued_execution_run_id: None,
            parent_workflow_execution: None,
            cron_schedule: None,
            memo: None,
            search_attributes: None,
        }
    }

    #[tokio::test]
    async fn test_get_version_first_execution_returns_max() {
        let workflow_info = create_test_workflow_info();
        let ctx = WorkflowContext::new(workflow_info);
        ctx.set_replay_mode(false);

        let version = ctx.get_version("test-change", DEFAULT_VERSION, 3);
        assert_eq!(version, 3, "First execution should return max_supported");
    }

    #[tokio::test]
    async fn test_get_version_replay_returns_default() {
        let workflow_info = create_test_workflow_info();
        let ctx = WorkflowContext::new(workflow_info);
        ctx.set_replay_mode(true);

        let version = ctx.get_version("test-change", DEFAULT_VERSION, 3);
        assert_eq!(
            version, DEFAULT_VERSION,
            "Replay mode should return DEFAULT_VERSION"
        );
    }

    #[tokio::test]
    async fn test_get_version_returns_cached() {
        let workflow_info = create_test_workflow_info();
        let ctx = WorkflowContext::new(workflow_info);
        ctx.set_replay_mode(false);

        let v1 = ctx.get_version("test-change", DEFAULT_VERSION, 3);
        let v2 = ctx.get_version("test-change", DEFAULT_VERSION, 5);

        // Second call returns cached version, not new max
        assert_eq!(v1, v2, "Should return cached version");
        assert_eq!(v1, 3, "Cached version should be original max_supported");
    }

    #[tokio::test]
    async fn test_get_version_different_change_ids_independent() {
        let workflow_info = create_test_workflow_info();
        let ctx = WorkflowContext::new(workflow_info);
        ctx.set_replay_mode(false);

        let v1 = ctx.get_version("change-1", DEFAULT_VERSION, 2);
        let v2 = ctx.get_version("change-2", DEFAULT_VERSION, 5);

        assert_eq!(v1, 2, "First change ID should return its max");
        assert_eq!(v2, 5, "Second change ID should return its max");
    }

    #[test]
    #[should_panic(expected = "removed support of version")]
    fn test_get_version_panics_if_version_too_low() {
        let workflow_info = create_test_workflow_info();
        let ctx = WorkflowContext::new(workflow_info);

        // Simulate replayed version = 0
        let mut versions = HashMap::new();
        versions.insert("test-change".to_string(), 0);
        ctx.set_change_versions(versions);

        // Min supported is now 1, should panic
        ctx.get_version("test-change", 1, 3);
    }

    #[test]
    #[should_panic(expected = "too old to support version")]
    fn test_get_version_panics_if_version_too_high() {
        let workflow_info = create_test_workflow_info();
        let ctx = WorkflowContext::new(workflow_info);

        // Simulate replayed version = 5
        let mut versions = HashMap::new();
        versions.insert("test-change".to_string(), 5);
        ctx.set_change_versions(versions);

        // Max supported is now 3, should panic
        ctx.get_version("test-change", DEFAULT_VERSION, 3);
    }

    #[tokio::test]
    async fn test_get_version_validates_cached_version_on_each_call() {
        let workflow_info = create_test_workflow_info();
        let ctx = WorkflowContext::new(workflow_info);

        // First call with wide range
        ctx.set_replay_mode(false);
        let v1 = ctx.get_version("test-change", DEFAULT_VERSION, 5);
        assert_eq!(v1, 5);

        // Second call with same range - should work
        let v2 = ctx.get_version("test-change", DEFAULT_VERSION, 5);
        assert_eq!(v2, 5);

        // Verify cached version is still within an even wider range
        let v3 = ctx.get_version("test-change", DEFAULT_VERSION, 10);
        assert_eq!(v3, 5, "Should return cached version even with wider range");
    }

    #[tokio::test]
    async fn test_default_version_handling() {
        let workflow_info = create_test_workflow_info();
        let ctx = WorkflowContext::new(workflow_info);
        ctx.set_replay_mode(true);

        // DEFAULT_VERSION should be valid in range
        let version = ctx.get_version("test-change", DEFAULT_VERSION, 3);
        assert_eq!(version, DEFAULT_VERSION);

        // Verify it got cached
        let version2 = ctx.get_version("test-change", DEFAULT_VERSION, 3);
        assert_eq!(version2, DEFAULT_VERSION);
    }
}
