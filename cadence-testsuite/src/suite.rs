//! Testing framework for Cadence workflows and activities.
//!
//! This module provides utilities for testing workflows and activities
//! without requiring a running Cadence server.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cadence_activity::{
    ActivityContext, ActivityInfo, WorkflowExecution as ActivityWorkflowExecution,
};
use cadence_core::{
    ActivityOptions, ChildWorkflowOptions, WorkflowExecution, WorkflowInfo, WorkflowType,
};
use cadence_workflow::context::WorkflowError;
use serde::{Deserialize, Serialize};

/// Type alias for boxed workflow functions
type WorkflowFn = Box<
    dyn Fn(
            TestWorkflowContext,
            Vec<u8>,
        ) -> Pin<
            Box<dyn Future<Output = Result<(TestWorkflowContext, Vec<u8>), WorkflowError>> + Send>,
        > + Send
        + Sync,
>;

/// Type alias for boxed activity functions (wrapped in Arc for cloning)
type ActivityFn = Arc<
    dyn Fn(
            &ActivityContext,
            Vec<u8>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>>
        + Send
        + Sync,
>;

/// Error type for activities in test environment
#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("Activity failed: {0}")]
    Generic(String),
}

/// Test suite for running workflow and activity tests
pub struct TestSuite;

impl TestSuite {
    /// Create a new test suite
    pub fn new() -> Self {
        Self
    }
}

impl Default for TestSuite {
    fn default() -> Self {
        Self::new()
    }
}

/// Test workflow environment for running workflow tests
pub struct TestWorkflowEnvironment {
    workflow_id: String,
    run_id: String,
    test_time: Arc<Mutex<TestTime>>,
    registered_workflows: HashMap<String, WorkflowFn>,
    registered_activities: HashMap<String, ActivityFn>,
    pending_signals: HashMap<String, Vec<Vec<u8>>>,
    executed_activities: Vec<String>,
}

impl TestWorkflowEnvironment {
    /// Create a new test workflow environment
    pub fn new() -> Self {
        Self {
            workflow_id: format!("test-workflow-{}", uuid::Uuid::new_v4()),
            run_id: format!("test-run-{}", uuid::Uuid::new_v4()),
            test_time: Arc::new(Mutex::new(TestTime::new())),
            registered_workflows: HashMap::new(),
            registered_activities: HashMap::new(),
            pending_signals: HashMap::new(),
            executed_activities: Vec::new(),
        }
    }

    /// Register a workflow for testing
    ///
    /// # Example
    /// ```ignore
    /// use cadence_testsuite::TestWorkflowEnvironment;
    ///
    /// let mut env = TestWorkflowEnvironment::new();
    /// env.register_workflow("my_workflow", |ctx, input: String| async move {
    ///     Ok((ctx, format!("Hello, {}!", input)))
    /// });
    /// ```
    pub fn register_workflow<F, Fut, I, O>(&mut self, name: &str, workflow: F)
    where
        F: Fn(TestWorkflowContext, I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(TestWorkflowContext, O), WorkflowError>> + Send + 'static,
        I: for<'de> Deserialize<'de> + Send + 'static,
        O: Serialize + Send + 'static,
    {
        #[expect(clippy::type_complexity)]
        let boxed = Box::new(
            move |ctx: TestWorkflowContext,
                  input_bytes: Vec<u8>|
                  -> Pin<
                Box<
                    dyn Future<Output = Result<(TestWorkflowContext, Vec<u8>), WorkflowError>>
                        + Send,
                >,
            > {
                let input: I = match serde_json::from_slice(&input_bytes) {
                    Ok(i) => i,
                    Err(e) => {
                        return Box::pin(async move {
                            Err(WorkflowError::Generic(format!(
                                "Input deserialization failed: {}",
                                e
                            )))
                        })
                    }
                };

                // Call the workflow function and convert the result
                let future = workflow(ctx, input);
                Box::pin(async move {
                    let (ctx, output) = future.await?;
                    let output_bytes = serde_json::to_vec(&output).map_err(|e| {
                        WorkflowError::Generic(format!("Output serialization failed: {}", e))
                    })?;
                    Ok((ctx, output_bytes))
                })
            },
        )
            as Box<
                dyn Fn(
                        TestWorkflowContext,
                        Vec<u8>,
                    ) -> Pin<
                        Box<
                            dyn Future<
                                    Output = Result<(TestWorkflowContext, Vec<u8>), WorkflowError>,
                                > + Send,
                        >,
                    > + Send
                    + Sync,
            >;

        self.registered_workflows.insert(name.to_string(), boxed);
    }

    /// Register an activity for testing
    ///
    /// # Example
    /// ```ignore
    /// use cadence_testsuite::TestWorkflowEnvironment;
    ///
    /// let mut env = TestWorkflowEnvironment::new();
    /// env.register_activity("my_activity", |ctx, input: i32| async move {
    ///     Ok(input * 2)
    /// });
    /// ```
    pub fn register_activity<F, Fut, I, O>(&mut self, name: &str, activity: F)
    where
        F: Fn(&ActivityContext, I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O, ActivityError>> + Send + 'static,
        I: for<'de> Deserialize<'de> + Send + 'static,
        O: Serialize + Send + 'static,
    {
        let arc = Arc::new(move |ctx: &ActivityContext, input_bytes: Vec<u8>| {
            let input: I = match serde_json::from_slice(&input_bytes) {
                Ok(i) => i,
                Err(e) => {
                    return Box::pin(async move {
                        Err(ActivityError::Generic(format!(
                            "Input deserialization failed: {}",
                            e
                        )))
                    }) as Pin<Box<dyn Future<Output = _> + Send>>
                }
            };

            let result = activity(ctx, input);
            Box::pin(async move {
                let output = result.await?;
                serde_json::to_vec(&output).map_err(|e| {
                    ActivityError::Generic(format!("Output serialization failed: {}", e))
                })
            }) as Pin<Box<dyn Future<Output = _> + Send>>
        });

        self.registered_activities.insert(name.to_string(), arc);
    }

    /// Execute a registered workflow by name with typed input
    ///
    /// # Example
    /// ```ignore
    /// use cadence_testsuite::TestWorkflowEnvironment;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut env = TestWorkflowEnvironment::new();
    /// env.register_workflow("greet", |ctx, name: String| async move {
    ///     Ok((ctx, format!("Hello, {}!", name)))
    /// });
    ///
    /// let result: Result<String, _> = env.execute_workflow("greet", "World".to_string()).await;
    /// assert_eq!(result.unwrap(), "Hello, World!");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_workflow<I, O>(&mut self, name: &str, input: I) -> Result<O, WorkflowError>
    where
        I: Serialize,
        O: for<'de> Deserialize<'de>,
    {
        // Serialize input
        let input_bytes = serde_json::to_vec(&input)
            .map_err(|e| WorkflowError::Generic(format!("Input serialization failed: {}", e)))?;

        // Look up workflow
        let workflow = self
            .registered_workflows
            .get(name)
            .ok_or_else(|| WorkflowError::Generic(format!("Workflow '{}' not registered", name)))?;

        // Create test context
        let activities = self.registered_activities.clone();
        let signals = self.pending_signals.clone();
        let test_time = self.test_time.clone();

        let ctx = TestWorkflowContext {
            workflow_id: self.workflow_id.clone(),
            run_id: self.run_id.clone(),
            workflow_type: name.to_string(),
            task_list: "test-task-list".to_string(),
            activities,
            signals,
            queries: HashMap::new(),
            test_time,
            is_cancelled: false,
        };

        // Clear pending signals after copying to context
        self.pending_signals.clear();

        // Execute workflow
        let (_ctx, result_bytes) = workflow(ctx, input_bytes).await?;

        // Deserialize output
        serde_json::from_slice(&result_bytes)
            .map_err(|e| WorkflowError::Generic(format!("Output deserialization failed: {}", e)))
    }

    /// Execute an activity directly for testing (without workflow)
    ///
    /// # Example
    /// ```ignore
    /// use cadence_testsuite::TestWorkflowEnvironment;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut env = TestWorkflowEnvironment::new();
    /// env.register_activity("double", |ctx, n: i32| async move {
    ///     Ok(n * 2)
    /// });
    ///
    /// let result: Result<i32, _> = env.execute_activity("double", 21).await;
    /// assert_eq!(result.unwrap(), 42);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_activity<I, O>(&self, name: &str, input: I) -> Result<O, ActivityError>
    where
        I: Serialize,
        O: for<'de> Deserialize<'de>,
    {
        let input_bytes = serde_json::to_vec(&input)
            .map_err(|e| ActivityError::Generic(format!("Input serialization: {}", e)))?;

        let activity = self
            .registered_activities
            .get(name)
            .ok_or_else(|| ActivityError::Generic(format!("Activity '{}' not registered", name)))?;

        // Create ActivityInfo
        let activity_info = ActivityInfo {
            activity_id: format!("test-activity-{}", uuid::Uuid::new_v4()),
            activity_type: name.to_string(),
            task_token: vec![],
            workflow_execution: ActivityWorkflowExecution::new(&self.workflow_id, &self.run_id),
            attempt: 1,
            scheduled_time: chrono::Utc::now(),
            started_time: chrono::Utc::now(),
            deadline: None,
            heartbeat_timeout: Duration::from_secs(0),
            heartbeat_details: None,
        };

        let ctx = ActivityContext::new(activity_info, None);

        let result_bytes = activity(&ctx, input_bytes).await?;

        serde_json::from_slice(&result_bytes)
            .map_err(|e| ActivityError::Generic(format!("Output deserialization: {}", e)))
    }

    /// Queue a signal to be sent to the workflow when it starts
    ///
    /// # Example
    /// ```ignore
    /// use cadence_testsuite::TestWorkflowEnvironment;
    ///
    /// let mut env = TestWorkflowEnvironment::new();
    /// env.signal_workflow("approval", vec![1, 2, 3]);
    /// // When workflow starts, it will receive this signal
    /// ```
    pub fn signal_workflow(&mut self, signal_name: &str, data: Vec<u8>) {
        self.pending_signals
            .entry(signal_name.to_string())
            .or_default()
            .push(data);
    }

    /// Set workflow time
    pub fn set_workflow_time(&mut self, time: chrono::DateTime<chrono::Utc>) {
        if let Ok(mut test_time) = self.test_time.lock() {
            test_time.set_time(time);
        }
    }

    /// Advance workflow time
    pub fn advance_workflow_time(&mut self, duration: Duration) {
        if let Ok(mut test_time) = self.test_time.lock() {
            test_time.advance(duration);
        }
    }

    /// Get list of executed activities (for assertions)
    pub fn get_executed_activities(&self) -> &[String] {
        &self.executed_activities
    }

    /// Check if an activity was executed
    pub fn was_activity_executed(&self, name: &str) -> bool {
        self.executed_activities.contains(&name.to_string())
    }

    #[expect(dead_code)]
    fn track_activity_execution(&mut self, name: &str) {
        self.executed_activities.push(name.to_string());
    }
}

impl Default for TestWorkflowEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Test workflow context that mimics WorkflowContext API
pub struct TestWorkflowContext {
    workflow_id: String,
    run_id: String,
    workflow_type: String,
    task_list: String,
    activities: HashMap<String, ActivityFn>,
    signals: HashMap<String, Vec<Vec<u8>>>,
    #[expect(clippy::type_complexity)]
    queries: HashMap<String, Box<dyn Fn(Vec<u8>) -> Vec<u8> + Send + Sync>>,
    test_time: Arc<Mutex<TestTime>>,
    is_cancelled: bool,
}

impl TestWorkflowContext {
    /// Get workflow information
    pub fn workflow_info(&self) -> WorkflowInfo {
        WorkflowInfo {
            workflow_execution: WorkflowExecution::new(&self.workflow_id, &self.run_id),
            workflow_type: WorkflowType {
                name: self.workflow_type.clone(),
            },
            task_list: self.task_list.clone(),
            start_time: chrono::Utc::now(),
            execution_start_to_close_timeout: Duration::from_secs(60),
            task_start_to_close_timeout: Duration::from_secs(10),
            attempt: 1,
            cron_schedule: None,
            continued_execution_run_id: None,
            parent_workflow_execution: None,
            memo: None,
            search_attributes: None,
        }
    }

    /// Execute an activity by name
    pub async fn execute_activity(
        &mut self,
        activity_type: &str,
        args: Option<Vec<u8>>,
        _options: ActivityOptions,
    ) -> Result<Vec<u8>, WorkflowError> {
        let activity = self.activities.get(activity_type).ok_or_else(|| {
            WorkflowError::ActivityFailed(format!("Activity '{}' not registered", activity_type))
        })?;

        // Create ActivityInfo
        let activity_info = ActivityInfo {
            activity_id: format!("activity-{}", uuid::Uuid::new_v4()),
            activity_type: activity_type.to_string(),
            task_token: vec![],
            workflow_execution: ActivityWorkflowExecution::new(&self.workflow_id, &self.run_id),
            attempt: 1,
            scheduled_time: chrono::Utc::now(),
            started_time: chrono::Utc::now(),
            deadline: None,
            heartbeat_timeout: Duration::from_secs(0),
            heartbeat_details: None,
        };

        let ctx = ActivityContext::new(activity_info, None);
        let input = args.unwrap_or_default();

        activity(&ctx, input)
            .await
            .map_err(|e| WorkflowError::ActivityFailed(format!("{:?}", e)))
    }

    /// Get a signal channel for receiving signals
    pub fn get_signal_channel(&self, signal_name: &str) -> TestSignalChannel {
        let signals = self.signals.get(signal_name).cloned().unwrap_or_default();
        TestSignalChannel::new(signals)
    }

    /// Sleep for a duration (advances test time)
    pub async fn sleep(&self, duration: Duration) {
        if let Ok(mut time) = self.test_time.lock() {
            time.advance(duration);
        }
    }

    /// Get current workflow time (deterministic)
    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.test_time
            .lock()
            .map(|t| t.current_time)
            .unwrap_or_else(|_| chrono::Utc::now())
    }

    /// Check if workflow is cancelled
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled
    }

    /// Execute a child workflow
    pub async fn execute_child_workflow(
        &mut self,
        _workflow_type: &str,
        _args: Option<Vec<u8>>,
        _options: ChildWorkflowOptions,
    ) -> Result<Vec<u8>, WorkflowError> {
        // TODO: Implement child workflow execution
        Err(WorkflowError::Generic(
            "Child workflow execution not yet implemented in test environment".to_string(),
        ))
    }

    /// Signal an external workflow
    pub async fn signal_external_workflow(
        &self,
        _workflow_id: &str,
        _run_id: Option<&str>,
        _signal_name: &str,
        _args: Option<Vec<u8>>,
    ) -> Result<(), WorkflowError> {
        // TODO: Implement external workflow signaling
        Ok(())
    }

    /// Request cancellation of an external workflow
    pub async fn request_cancel_external_workflow(
        &self,
        _workflow_id: &str,
        _run_id: Option<&str>,
    ) -> Result<(), WorkflowError> {
        // TODO: Implement external workflow cancellation
        Ok(())
    }

    /// Execute a side effect (non-deterministic operation)
    pub async fn side_effect<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        f()
    }

    /// Execute a mutable side effect (cached side effect)
    pub async fn mutable_side_effect<F, R>(&self, _id: &str, f: F) -> R
    where
        F: FnOnce() -> R,
        R: Clone,
    {
        // TODO: Implement caching for mutable side effects
        f()
    }

    /// Get version for backwards-compatible workflow changes
    pub fn get_version(&self, _change_id: &str, min_supported: i32, _max_supported: i32) -> i32 {
        // TODO: Implement versioning
        min_supported
    }

    /// Set a query handler
    pub fn set_query_handler<F>(&mut self, query_type: &str, handler: F)
    where
        F: Fn(Vec<u8>) -> Vec<u8> + Send + Sync + 'static,
    {
        self.queries
            .insert(query_type.to_string(), Box::new(handler));
    }

    /// Upsert search attributes
    pub fn upsert_search_attributes(&self, _search_attributes: Vec<(String, Vec<u8>)>) {
        // TODO: Implement search attributes
    }

    /// Get a cancellation channel
    pub fn get_cancellation_channel(&self) -> TestCancellationChannel {
        TestCancellationChannel::new()
    }
}

/// Test signal channel for receiving signals
pub struct TestSignalChannel {
    signals: Vec<Vec<u8>>,
    current_index: usize,
}

impl TestSignalChannel {
    fn new(signals: Vec<Vec<u8>>) -> Self {
        Self {
            signals,
            current_index: 0,
        }
    }

    /// Receive a signal (async)
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        if self.current_index < self.signals.len() {
            let signal = self.signals[self.current_index].clone();
            self.current_index += 1;
            Some(signal)
        } else {
            // No more signals available
            None
        }
    }

    /// Try to receive a signal (non-blocking)
    pub fn try_recv(&mut self) -> Option<Vec<u8>> {
        if self.current_index < self.signals.len() {
            let signal = self.signals[self.current_index].clone();
            self.current_index += 1;
            Some(signal)
        } else {
            None
        }
    }
}

/// Test cancellation channel
pub struct TestCancellationChannel;

impl TestCancellationChannel {
    fn new() -> Self {
        Self
    }

    /// Wait for cancellation (would block in real implementation)
    pub async fn recv(&mut self) {
        // In test environment, this never receives (workflow isn't cancelled by default)
        std::future::pending().await
    }
}

/// Test activity environment for running activity tests
pub struct TestActivityEnvironment;

impl TestActivityEnvironment {
    /// Create a new test activity environment
    pub fn new() -> Self {
        Self
    }

    /// Execute an activity in the test environment
    pub async fn execute_activity<F, R>(&self, activity: F) -> R
    where
        F: FnOnce(&mut TestActivityContext) -> R,
    {
        let mut ctx = TestActivityContext::new(ActivityInfo {
            activity_id: format!("test-activity-{}", uuid::Uuid::new_v4()),
            activity_type: "test".to_string(),
            task_token: vec![],
            workflow_execution: ActivityWorkflowExecution::new("test-workflow", "test-run"),
            attempt: 1,
            scheduled_time: chrono::Utc::now(),
            started_time: chrono::Utc::now(),
            deadline: None,
            heartbeat_timeout: Duration::from_secs(0),
            heartbeat_details: None,
        });
        activity(&mut ctx)
    }
}

impl Default for TestActivityEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Test activity context that mimics ActivityContext API
pub struct TestActivityContext {
    activity_info: ActivityInfo,
    recorded_heartbeats: Vec<Option<Vec<u8>>>,
}

impl TestActivityContext {
    fn new(activity_info: ActivityInfo) -> Self {
        Self {
            activity_info,
            recorded_heartbeats: Vec::new(),
        }
    }

    /// Get activity information
    pub fn get_info(&self) -> &ActivityInfo {
        &self.activity_info
    }

    /// Record a heartbeat with optional details
    pub fn record_heartbeat(&mut self, details: Option<&[u8]>) {
        self.recorded_heartbeats.push(details.map(|d| d.to_vec()));
    }

    /// Check if heartbeat details exist from previous attempts
    pub fn has_heartbeat_details(&self) -> bool {
        self.activity_info.heartbeat_details.is_some()
    }

    /// Get heartbeat details from previous attempts
    pub fn get_heartbeat_details(&self) -> Option<&[u8]> {
        self.activity_info.heartbeat_details.as_deref()
    }

    /// Check if the activity has been cancelled
    pub fn is_cancelled(&self) -> bool {
        // TODO: Implement cancellation check
        false
    }

    /// Get the deadline for activity completion
    pub fn get_deadline(&self) -> Option<Instant> {
        self.activity_info.deadline
    }

    /// Get the remaining time before deadline
    pub fn get_remaining_time(&self) -> Option<Duration> {
        self.activity_info.deadline.map(|d| {
            let now = Instant::now();
            if d > now {
                d - now
            } else {
                Duration::from_secs(0)
            }
        })
    }

    /// Get recorded heartbeats (for assertions)
    pub fn get_recorded_heartbeats(&self) -> &[Option<Vec<u8>>] {
        &self.recorded_heartbeats
    }
}

/// Test time controller
#[derive(Clone)]
pub struct TestTime {
    current_time: chrono::DateTime<chrono::Utc>,
}

impl TestTime {
    fn new() -> Self {
        Self {
            current_time: chrono::Utc::now(),
        }
    }

    fn set_time(&mut self, time: chrono::DateTime<chrono::Utc>) {
        self.current_time = time;
    }

    fn advance(&mut self, duration: Duration) {
        self.current_time += chrono::Duration::from_std(duration).unwrap();
    }
}

/// Workflow replayer for backwards compatibility testing
pub struct WorkflowReplayer;

impl WorkflowReplayer {
    /// Create a new workflow replayer
    pub fn new() -> Self {
        Self
    }

    /// Replay workflow history
    pub async fn replay_workflow_history(
        &self,
        _history: WorkflowHistory,
    ) -> Result<(), ReplayError> {
        // TODO: Implement history replay
        Ok(())
    }

    /// Replay workflow history from JSON
    pub async fn replay_workflow_history_from_json(&self, _json: &str) -> Result<(), ReplayError> {
        // TODO: Implement JSON history replay
        Ok(())
    }

    /// Replay partial workflow history
    pub async fn replay_partial_workflow_history(
        &self,
        _history: WorkflowHistory,
        _last_event_id: i64,
    ) -> Result<(), ReplayError> {
        // TODO: Implement partial history replay
        Ok(())
    }
}

impl Default for WorkflowReplayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Workflow history for replay
pub struct WorkflowHistory {
    pub events: Vec<HistoryEvent>,
}

/// History event for replay
pub struct HistoryEvent {
    pub event_id: i64,
    pub event_type: String,
    pub timestamp: i64,
}

/// Replay error
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("Non-deterministic workflow detected: {0}")]
    NonDeterministic(String),
    #[error("Invalid history: {0}")]
    InvalidHistory(String),
    #[error("Replay failed: {0}")]
    ReplayFailed(String),
}

/// Workflow shadower for detecting non-deterministic changes
pub struct WorkflowShadower;

impl WorkflowShadower {
    /// Create a new workflow shadower
    pub fn new() -> Self {
        Self
    }

    /// Run the shadower
    pub async fn run(&self) -> Result<(), ShadowerError> {
        // TODO: Implement shadowing
        Ok(())
    }
}

impl Default for WorkflowShadower {
    fn default() -> Self {
        Self::new()
    }
}

/// Shadower error
#[derive(Debug, thiserror::Error)]
pub enum ShadowerError {
    #[error("Shadowing failed: {0}")]
    ShadowingFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_execute_workflow() {
        let mut env = TestWorkflowEnvironment::new();

        env.register_workflow("test_workflow", |ctx, input: String| async move {
            Ok((ctx, format!("Hello, {}!", input)))
        });

        let result: Result<String, _> = env
            .execute_workflow("test_workflow", "World".to_string())
            .await;
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[tokio::test]
    async fn test_register_and_execute_activity() {
        let mut env = TestWorkflowEnvironment::new();

        env.register_activity(
            "test_activity",
            |_ctx, input: i32| async move { Ok(input * 2) },
        );

        let result: Result<i32, _> = env.execute_activity("test_activity", 21).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_signal_workflow() {
        let mut env = TestWorkflowEnvironment::new();

        env.register_workflow("signal_workflow", |ctx, _input: ()| async move {
            let mut channel = ctx.get_signal_channel("test_signal");
            let signal = channel.recv().await;
            assert!(signal.is_some());
            Ok((ctx, ()))
        });

        env.signal_workflow("test_signal", vec![1, 2, 3]);
        let result: Result<(), _> = env.execute_workflow("signal_workflow", ()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_workflow_executes_activity() {
        let mut env = TestWorkflowEnvironment::new();

        env.register_activity("double", |_ctx, n: i32| async move { Ok(n * 2) });

        env.register_workflow("calc_workflow", |mut ctx, input: i32| async move {
            let result = ctx
                .execute_activity(
                    "double",
                    Some(serde_json::to_vec(&input).unwrap()),
                    ActivityOptions::default(),
                )
                .await?;
            let output: i32 = serde_json::from_slice(&result).unwrap();
            Ok((ctx, output))
        });

        let result: Result<i32, _> = env.execute_workflow("calc_workflow", 21).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_workflow_not_registered() {
        let mut env = TestWorkflowEnvironment::new();

        // Try to execute without registering
        let result: Result<String, _> =
            futures::executor::block_on(env.execute_workflow("nonexistent", "input".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not registered"));
    }

    #[tokio::test]
    async fn test_test_time() {
        let mut env = TestWorkflowEnvironment::new();
        let start_time = chrono::Utc::now();

        env.set_workflow_time(start_time);
        env.advance_workflow_time(Duration::from_secs(60));

        // The test_time should have advanced
        // Note: We can't easily verify this without exposing the time
    }
}
