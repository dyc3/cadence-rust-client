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
    #[error("Activity failed: {0}")]
    ActivityFailed(String),
    #[error("Child workflow failed: {0}")]
    ChildWorkflowFailed(String),
    #[error("Signal failed: {0}")]
    SignalFailed(String),
    #[error("Cancel failed: {0}")]
    CancelFailed(String),
    #[error("Continue as new")]
    ContinueAsNew,
    #[error("Workflow cancelled")]
    Cancelled,
    #[error("Generic error: {0}")]
    Generic(String),
}

/// Activity error
#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("Activity failed: {0}")]
    Failed(String),
}
