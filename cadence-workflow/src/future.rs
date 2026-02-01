//! Workflow futures.

use std::future::Future;
use std::pin::Pin;

/// Future for child workflow execution
pub type ChildWorkflowFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>>;

/// Future for activity execution
pub type ActivityFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>>;

/// Future for timer
pub type TimerFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Workflow error
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Workflow failed: {0}")]
    Failed(String),
}

/// Activity error
#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("Activity failed: {0}")]
    Failed(String),
}
