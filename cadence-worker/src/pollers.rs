//! Task pollers for polling decision and activity tasks from Cadence server.
//!
//! This module provides the pollers that continuously poll for new tasks
//! from the Cadence server and dispatch them for processing.

use crate::handlers::activity::ActivityTaskHandler;
use crate::handlers::decision::DecisionTaskHandler;
use async_trait::async_trait;
use cadence_core::CadenceError;
use cadence_proto::workflow_service::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

/// Task poller trait
#[async_trait]
pub trait TaskPoller: Send + Sync {
    type Task: Send;
    type Response: Send;

    /// Poll for a new task
    async fn poll(&self) -> Result<Option<Self::Task>, CadenceError>;

    /// Process a task
    async fn process(&self, task: Self::Task) -> Result<Self::Response, CadenceError>;
}

/// Decision task poller
pub struct DecisionTaskPoller {
    service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync>,
    domain: String,
    task_list: String,
    identity: String,
    binary_checksum: String,
    sticky_task_list: Option<String>,
    handler: Arc<DecisionTaskHandler>,
}

impl DecisionTaskPoller {
    pub fn new(
        service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync>,
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
    ) -> Result<Option<PollForDecisionTaskResponse>, CadenceError> {
        // Try sticky task list first if available
        if let Some(ref sticky) = self.sticky_task_list {
            let request = PollForDecisionTaskRequest {
                domain: self.domain.clone(),
                task_list: Some(cadence_proto::shared::TaskList {
                    name: sticky.clone(),
                    kind: cadence_proto::shared::TaskListKind::Sticky,
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
            task_list: Some(cadence_proto::shared::TaskList {
                name: self.task_list.clone(),
                kind: cadence_proto::shared::TaskListKind::Normal,
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

    async fn poll(&self) -> Result<Option<Self::Task>, CadenceError> {
        self.poll_decision_task().await
    }

    async fn process(&self, task: Self::Task) -> Result<Self::Response, CadenceError> {
        self.handler.handle(task).await
    }
}

/// Activity task poller
pub struct ActivityTaskPoller {
    service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync>,
    domain: String,
    task_list: String,
    identity: String,
    handler: Arc<ActivityTaskHandler>,
}

impl ActivityTaskPoller {
    pub fn new(
        service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync>,
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
            handler,
        }
    }

    /// Poll for activity task
    async fn poll_activity_task(
        &self,
    ) -> Result<Option<PollForActivityTaskResponse>, CadenceError> {
        println!("[ActivityTaskPoller] Polling task list: {}", self.task_list);

        let request = PollForActivityTaskRequest {
            domain: self.domain.clone(),
            task_list: Some(cadence_proto::shared::TaskList {
                name: self.task_list.clone(),
                kind: cadence_proto::shared::TaskListKind::Normal,
            }),
            identity: self.identity.clone(),
            task_list_metadata: None,
        };

        match self.service.poll_for_activity_task(request).await {
            Ok(response) => {
                if response.task_token.is_empty() {
                    return Ok(None);
                }
                println!(
                    "[ActivityTaskPoller] Received task on list {}: ActivityId={:?}",
                    self.task_list, response.activity_id
                );
                Ok(Some(response))
            }
            Err(e) => {
                println!(
                    "[ActivityTaskPoller] Error polling task list {}: {}",
                    self.task_list, e
                );
                tracing::error!("Error polling activity task: {}", e);
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl TaskPoller for ActivityTaskPoller {
    type Task = PollForActivityTaskResponse;
    type Response = RespondActivityTaskCompletedResponse;

    async fn poll(&self) -> Result<Option<Self::Task>, CadenceError> {
        self.poll_activity_task().await
    }

    async fn process(&self, task: Self::Task) -> Result<Self::Response, CadenceError> {
        self.handler.handle(task).await
    }
}

use tokio::task::JoinHandle;

/// Poller manager that runs multiple pollers
pub struct PollerManager {
    decision_pollers: Vec<Arc<DecisionTaskPoller>>,
    activity_pollers: Vec<Arc<ActivityTaskPoller>>,
    join_handles: Vec<JoinHandle<()>>,
}

impl PollerManager {
    pub fn new() -> Self {
        Self {
            decision_pollers: Vec::new(),
            activity_pollers: Vec::new(),
            join_handles: Vec::new(),
        }
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
            let poller = Arc::clone(poller);
            let handle = tokio::spawn(async move {
                let mut poll_interval = interval(Duration::from_millis(100));
                loop {
                    poll_interval.tick().await;

                    match poller.poll().await {
                        Ok(Some(task)) => {
                            if let Err(e) = poller.process(task).await {
                                tracing::error!("Error processing decision task: {}", e);
                            }
                        }
                        Ok(None) => {} // No task available
                        Err(e) => {
                            tracing::error!("Error polling decision task: {}", e);
                        }
                    }
                }
            });
            self.join_handles.push(handle);
        }

        // Start activity pollers
        for poller in &self.activity_pollers {
            let poller = Arc::clone(poller);
            let handle = tokio::spawn(async move {
                let mut poll_interval = interval(Duration::from_millis(100));
                loop {
                    poll_interval.tick().await;

                    match poller.poll().await {
                        Ok(Some(task)) => {
                            if let Err(e) = poller.process(task).await {
                                tracing::error!("Error processing activity task: {}", e);
                            }
                        }
                        Ok(None) => {} // No task available
                        Err(e) => {
                            tracing::error!("Error polling activity task: {}", e);
                        }
                    }
                }
            });
            self.join_handles.push(handle);
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

/// Rate limited poller decorator
pub struct RateLimitedPoller<P: TaskPoller> {
    inner: P,
    rate_limiter: tokio::sync::Semaphore,
}

impl<P: TaskPoller> RateLimitedPoller<P> {
    pub fn new(inner: P, max_tasks_per_second: f64) -> Self {
        let permits = if max_tasks_per_second >= 100_000.0 {
            // Unlimited
            1000
        } else {
            (max_tasks_per_second as usize).max(1)
        };

        Self {
            inner,
            rate_limiter: tokio::sync::Semaphore::new(permits),
        }
    }
}

#[async_trait]
impl<P: TaskPoller> TaskPoller for RateLimitedPoller<P> {
    type Task = P::Task;
    type Response = P::Response;

    async fn poll(&self) -> Result<Option<Self::Task>, CadenceError> {
        let _permit = self.rate_limiter.acquire().await.unwrap();
        self.inner.poll().await
    }

    async fn process(&self, task: Self::Task) -> Result<Self::Response, CadenceError> {
        self.inner.process(task).await
    }
}
