//! Task pollers for polling decision and activity tasks from Cadence server.
//!
//! This module provides the pollers that continuously poll for new tasks
//! from the Cadence server and dispatch them for processing.

use crate::autoscaler::{PollerAutoScaler, RateLimiter};
use crate::handlers::activity::ActivityTaskHandler;
use crate::handlers::decision::DecisionTaskHandler;
use async_trait::async_trait;
use crabdance_core::{CadenceError, TransportError};
use crabdance_proto::workflow_service::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info};

/// Task poller trait
#[async_trait]
pub trait TaskPoller: Send + Sync {
    type Task: Send;
    type Response: Send;

    /// Poll for a new task
    async fn poll(&self) -> Result<Option<Self::Task>, TransportError>;

    /// Process a task
    async fn process(&self, task: Self::Task) -> Result<Self::Response, CadenceError>;
}

/// Decision task poller
pub struct DecisionTaskPoller {
    service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync>,
    domain: String,
    task_list: String,
    identity: String,
    binary_checksum: String,
    sticky_task_list: Option<String>,
    handler: Arc<DecisionTaskHandler>,
}

impl DecisionTaskPoller {
    pub fn new(
        service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync>,
        domain: impl Into<String>,
        task_list: impl Into<String>,
        identity: impl Into<String>,
        handler: Arc<DecisionTaskHandler>,
    ) -> Self {
        Self {
            service,
            domain: domain.into(),
            task_list: task_list.into(),
            identity: identity.into(),
            binary_checksum: String::new(),
            sticky_task_list: None,
            handler,
        }
    }

    pub fn with_sticky_task_list(mut self, sticky_task_list: impl Into<String>) -> Self {
        self.sticky_task_list = Some(sticky_task_list.into());
        self
    }

    pub fn with_binary_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.binary_checksum = checksum.into();
        self
    }

    /// Poll for decision task with sticky execution support
    async fn poll_decision_task(
        &self,
    ) -> Result<Option<PollForDecisionTaskResponse>, TransportError> {
        // Try sticky task list first if available
        if let Some(ref sticky) = self.sticky_task_list {
            let request = PollForDecisionTaskRequest {
                domain: self.domain.clone(),
                task_list: Some(crabdance_proto::shared::TaskList {
                    name: sticky.clone(),
                    kind: crabdance_proto::shared::TaskListKind::Sticky,
                }),
                identity: self.identity.clone(),
                binary_checksum: self.binary_checksum.clone(),
            };

            // Poll with shorter timeout for sticky queue
            match tokio::time::timeout(
                Duration::from_millis(500),
                self.service.poll_for_decision_task(request),
            )
            .await
            {
                Ok(Ok(response)) => return Ok(Some(response)),
                Ok(Err(_)) => {} // Error polling sticky, fall through to normal
                Err(_) => {}     // Timeout, fall through to normal
            }
        }

        // Poll normal task list
        let request = PollForDecisionTaskRequest {
            domain: self.domain.clone(),
            task_list: Some(crabdance_proto::shared::TaskList {
                name: self.task_list.clone(),
                kind: crabdance_proto::shared::TaskListKind::Normal,
            }),
            identity: self.identity.clone(),
            binary_checksum: self.binary_checksum.clone(),
        };

        match self.service.poll_for_decision_task(request).await {
            Ok(response) => {
                if response.task_token.is_empty() {
                    return Ok(None);
                }
                Ok(Some(response))
            }
            Err(e) => {
                // Log error and return None to continue polling
                tracing::error!("Error polling decision task: {}", e);
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl TaskPoller for DecisionTaskPoller {
    type Task = PollForDecisionTaskResponse;
    type Response = RespondDecisionTaskCompletedResponse;

    async fn poll(&self) -> Result<Option<Self::Task>, TransportError> {
        self.poll_decision_task().await
    }

    async fn process(&self, task: Self::Task) -> Result<Self::Response, CadenceError> {
        self.handler.handle(task).await
    }
}

/// Activity task poller
pub struct ActivityTaskPoller {
    service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync>,
    domain: String,
    task_list: String,
    identity: String,
    /// Server-side per-task-list activities/sec (Go's `TaskListActivitiesPerSecond`).
    /// `None` when unlimited.
    task_list_activities_per_second: Option<f64>,
    handler: Arc<ActivityTaskHandler>,
}

impl ActivityTaskPoller {
    pub fn new(
        service: Arc<dyn WorkflowService<Error = TransportError> + Send + Sync>,
        domain: impl Into<String>,
        task_list: impl Into<String>,
        identity: impl Into<String>,
        handler: Arc<ActivityTaskHandler>,
    ) -> Self {
        Self {
            service,
            domain: domain.into(),
            task_list: task_list.into(),
            identity: identity.into(),
            task_list_activities_per_second: None,
            handler,
        }
    }

    /// Set the server-side per-task-list activity rate limit sent on each poll.
    /// Rates at or above the unlimited sentinel leave it unset.
    pub fn with_task_list_activities_per_second(mut self, rate: f64) -> Self {
        self.task_list_activities_per_second = if rate >= crate::autoscaler::UNLIMITED_RPS {
            None
        } else {
            Some(rate)
        };
        self
    }

    /// Poll for activity task
    async fn poll_activity_task(
        &self,
    ) -> Result<Option<PollForActivityTaskResponse>, TransportError> {
        debug!(task_list = %self.task_list, "polling activity task list");

        let request = PollForActivityTaskRequest {
            domain: self.domain.clone(),
            task_list: Some(crabdance_proto::shared::TaskList {
                name: self.task_list.clone(),
                kind: crabdance_proto::shared::TaskListKind::Normal,
            }),
            identity: self.identity.clone(),
            task_list_metadata: self.task_list_activities_per_second.map(|rate| {
                crabdance_proto::workflow_service::TaskListMetadata {
                    max_tasks_per_second: Some(rate),
                }
            }),
        };

        match self.service.poll_for_activity_task(request).await {
            Ok(response) => {
                if response.task_token.is_empty() {
                    return Ok(None);
                }
                info!(
                    task_list = %self.task_list,
                    activity_id = ?response.activity_id,
                    "received activity task"
                );
                Ok(Some(response))
            }
            Err(e) => {
                error!(
                    task_list = %self.task_list,
                    error = %e,
                    "error polling activity task list"
                );
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl TaskPoller for ActivityTaskPoller {
    type Task = PollForActivityTaskResponse;
    type Response = RespondActivityTaskCompletedResponse;

    async fn poll(&self) -> Result<Option<Self::Task>, TransportError> {
        self.poll_activity_task().await
    }

    async fn process(&self, task: Self::Task) -> Result<Self::Response, CadenceError> {
        self.handler.handle(task).await
    }
}

use tokio::task::JoinHandle;

/// Poller manager that runs multiple pollers.
///
/// Each poll loop paces itself through a [`RateLimiter`] (per-second poll limit)
/// and, when an [`PollerAutoScaler`] is attached, acquires a permit from the
/// scaler's resizable gate so the active poller count tracks the configured
/// min/max. Both default to no-ops (unlimited rate, no gate).
pub struct PollerManager {
    decision_pollers: Vec<Arc<DecisionTaskPoller>>,
    activity_pollers: Vec<Arc<ActivityTaskPoller>>,
    decision_rate_limiter: Arc<RateLimiter>,
    activity_rate_limiter: Arc<RateLimiter>,
    decision_scaler: Option<Arc<PollerAutoScaler>>,
    activity_scaler: Option<Arc<PollerAutoScaler>>,
    join_handles: Vec<JoinHandle<()>>,
}

impl PollerManager {
    pub fn new() -> Self {
        Self {
            decision_pollers: Vec::new(),
            activity_pollers: Vec::new(),
            decision_rate_limiter: Arc::new(RateLimiter::new(crate::autoscaler::UNLIMITED_RPS)),
            activity_rate_limiter: Arc::new(RateLimiter::new(crate::autoscaler::UNLIMITED_RPS)),
            decision_scaler: None,
            activity_scaler: None,
            join_handles: Vec::new(),
        }
    }

    /// Apply per-second poll rate limits (Go's `WorkerDecisionTasksPerSecond` /
    /// `WorkerActivitiesPerSecond`). Rates at or above the unlimited sentinel
    /// disable pacing.
    pub fn with_rate_limits(&mut self, decision_per_second: f64, activity_per_second: f64) {
        self.decision_rate_limiter = Arc::new(RateLimiter::new(decision_per_second));
        self.activity_rate_limiter = Arc::new(RateLimiter::new(activity_per_second));
    }

    /// Attach a decision-poller auto-scaler. Its control loop is started in [`start`].
    pub fn set_decision_scaler(&mut self, scaler: Arc<PollerAutoScaler>) {
        self.decision_scaler = Some(scaler);
    }

    /// Attach an activity-poller auto-scaler. Its control loop is started in [`start`].
    pub fn set_activity_scaler(&mut self, scaler: Arc<PollerAutoScaler>) {
        self.activity_scaler = Some(scaler);
    }

    /// Add a decision task poller
    pub fn add_decision_poller(&mut self, poller: Arc<DecisionTaskPoller>) {
        self.decision_pollers.push(poller);
    }

    /// Add an activity task poller
    pub fn add_activity_poller(&mut self, poller: Arc<ActivityTaskPoller>) {
        self.activity_pollers.push(poller);
    }

    /// Add a join handle for a background task
    pub fn add_join_handle(&mut self, handle: tokio::task::JoinHandle<()>) {
        self.join_handles.push(handle);
    }

    /// Start all pollers
    pub fn start(&mut self) {
        // Start decision pollers
        for poller in &self.decision_pollers {
            let handle = spawn_poll_loop(
                Arc::clone(poller),
                Arc::clone(&self.decision_rate_limiter),
                self.decision_scaler.clone(),
                "decision",
            );
            self.join_handles.push(handle);
        }

        // Start activity pollers
        for poller in &self.activity_pollers {
            let handle = spawn_poll_loop(
                Arc::clone(poller),
                Arc::clone(&self.activity_rate_limiter),
                self.activity_scaler.clone(),
                "activity",
            );
            self.join_handles.push(handle);
        }

        // Start auto-scaler control loops (one each, even though several pollers
        // share a scaler).
        if let Some(scaler) = &self.decision_scaler {
            let scaler = Arc::clone(scaler);
            self.join_handles
                .push(tokio::spawn(async move { scaler.run().await }));
        }
        if let Some(scaler) = &self.activity_scaler {
            let scaler = Arc::clone(scaler);
            self.join_handles
                .push(tokio::spawn(async move { scaler.run().await }));
        }
    }

    /// Stop all pollers
    pub async fn stop(&mut self) {
        for handle in &self.join_handles {
            handle.abort();
        }
        self.join_handles.clear();
    }
}

impl Default for PollerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn one poller's loop. Each iteration: (optionally) acquire an auto-scaler
/// gate permit, pace through the rate limiter, poll, record the poll latency for
/// scaling, and process any task. The gate permit is held across processing so
/// the active count reflects busy pollers.
fn spawn_poll_loop<P>(
    poller: Arc<P>,
    rate_limiter: Arc<RateLimiter>,
    scaler: Option<Arc<PollerAutoScaler>>,
    kind: &'static str,
) -> JoinHandle<()>
where
    P: TaskPoller + 'static,
{
    let gate = scaler.as_ref().map(|s| s.gate());
    tokio::spawn(async move {
        let mut poll_interval = interval(Duration::from_millis(100));
        loop {
            poll_interval.tick().await;

            let _permit = match &gate {
                Some(g) => Some(g.acquire().await),
                None => None,
            };
            rate_limiter.acquire().await;

            let started = std::time::Instant::now();
            let result = poller.poll().await;
            if let Some(s) = &scaler {
                s.record_poll(started.elapsed());
            }

            match result {
                Ok(Some(task)) => {
                    if let Err(e) = poller.process(task).await {
                        error!(kind, error = %e, "error processing task");
                    }
                }
                Ok(None) => {} // No task available
                Err(e) => {
                    error!(kind, error = %e, "error polling task");
                }
            }
        }
    })
}
