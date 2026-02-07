//! Workflow and activity registry.
//!
//! This module provides the registry for registering workflows and activities.

use dashmap::DashMap;
use dyn_clone::DynClone;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use uber_cadence_activity::ActivityContext;
use uber_cadence_workflow::context::WorkflowContext;
pub use uber_cadence_workflow::WorkflowError;

/// Workflow trait
pub trait Workflow: Send + Sync + DynClone {
    fn execute(
        &self,
        ctx: WorkflowContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>>;
}

dyn_clone::clone_trait_object!(Workflow);

/// Activity trait
pub trait Activity: Send + Sync + DynClone {
    fn execute(
        &self,
        ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>>;
}

dyn_clone::clone_trait_object!(Activity);

/// Activity error
#[derive(Debug, Clone, thiserror::Error)]
pub enum ActivityError {
    #[error("Activity execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Activity panicked: {0}")]
    Panic(String),
    #[error("Retryable activity error: {0}")]
    Retryable(String),
    #[error("Non-retryable activity error: {0}")]
    NonRetryable(String),
    #[error("Application error: {0}")]
    Application(String),
    #[error("Retryable with delay: {0}ms")]
    RetryableWithDelay(String, u64),
    #[error("Activity cancelled")]
    Cancelled,
    #[error("Activity timed out: {0:?}")]
    Timeout(uber_cadence_proto::shared::TimeoutType),
}

impl ActivityError {
    /// Create a retryable error
    pub fn retryable(msg: impl Into<String>) -> Self {
        Self::Retryable(msg.into())
    }

    /// Create a non-retryable error
    pub fn non_retryable(msg: impl Into<String>) -> Self {
        Self::NonRetryable(msg.into())
    }

    /// Create an application error
    pub fn application(msg: impl Into<String>) -> Self {
        Self::Application(msg.into())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_) | Self::RetryableWithDelay(_, _))
    }
}

/// Registry trait
pub trait Registry: Send + Sync {
    /// Register a workflow
    fn register_workflow(&self, name: &str, workflow: Box<dyn Workflow>);

    /// Register a workflow with options
    fn register_workflow_with_options(
        &self,
        workflow: Box<dyn Workflow>,
        options: WorkflowRegisterOptions,
    );

    /// Get registered workflow info
    fn get_registered_workflows(&self) -> Vec<RegistryInfo>;

    /// Register an activity
    fn register_activity(&self, name: &str, activity: Box<dyn Activity>);

    /// Register an activity with options
    fn register_activity_with_options(
        &self,
        activity: Box<dyn Activity>,
        options: ActivityRegisterOptions,
    );

    /// Get registered activity info
    fn get_registered_activities(&self) -> Vec<RegistryInfo>;

    /// Get workflow by name
    fn get_workflow(&self, name: &str) -> Option<Box<dyn Workflow>>;

    /// Get activity by name
    fn get_activity(&self, name: &str) -> Option<Box<dyn Activity>>;
}

/// Workflow register options
#[derive(Debug, Clone, Default)]
pub struct WorkflowRegisterOptions {
    pub name: Option<String>,
    pub disable_already_registered_check: bool,
}

/// Activity register options
#[derive(Debug, Clone, Default)]
pub struct ActivityRegisterOptions {
    pub name: Option<String>,
    pub disable_already_registered_check: bool,
}

/// Registry information
#[derive(Debug, Clone)]
pub struct RegistryInfo {
    pub name: String,
    pub type_name: String,
}

/// Workflow registry implementation using DashMap for concurrent access
pub struct WorkflowRegistry {
    workflows: Arc<DashMap<String, Box<dyn Workflow>>>,
    activities: Arc<DashMap<String, Box<dyn Activity>>>,
}

impl WorkflowRegistry {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(DashMap::new()),
            activities: Arc::new(DashMap::new()),
        }
    }
}

impl Default for WorkflowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for WorkflowRegistry {
    fn clone(&self) -> Self {
        Self {
            workflows: Arc::clone(&self.workflows),
            activities: Arc::clone(&self.activities),
        }
    }
}

impl Registry for WorkflowRegistry {
    fn register_workflow(&self, name: &str, workflow: Box<dyn Workflow>) {
        self.workflows.insert(name.to_string(), workflow);
    }

    fn register_workflow_with_options(
        &self,
        workflow: Box<dyn Workflow>,
        options: WorkflowRegisterOptions,
    ) {
        let name = options.name.unwrap_or_else(|| "workflow".to_string());
        self.register_workflow(&name, workflow);
    }

    fn get_registered_workflows(&self) -> Vec<RegistryInfo> {
        self.workflows
            .iter()
            .map(|entry| RegistryInfo {
                name: entry.key().clone(),
                type_name: "workflow".to_string(),
            })
            .collect()
    }

    fn register_activity(&self, name: &str, activity: Box<dyn Activity>) {
        self.activities.insert(name.to_string(), activity);
    }

    fn register_activity_with_options(
        &self,
        activity: Box<dyn Activity>,
        options: ActivityRegisterOptions,
    ) {
        let name = options.name.unwrap_or_else(|| "activity".to_string());
        self.register_activity(&name, activity);
    }

    fn get_registered_activities(&self) -> Vec<RegistryInfo> {
        self.activities
            .iter()
            .map(|entry| RegistryInfo {
                name: entry.key().clone(),
                type_name: "activity".to_string(),
            })
            .collect()
    }

    fn get_workflow(&self, name: &str) -> Option<Box<dyn Workflow>> {
        self.workflows.get(name).map(|entry| entry.clone())
    }

    fn get_activity(&self, name: &str) -> Option<Box<dyn Activity>> {
        self.activities.get(name).map(|entry| entry.clone())
    }
}
