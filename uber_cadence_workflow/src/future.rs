//! Workflow futures.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

/// Future for child workflow execution
pub type ChildWorkflowFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>, WorkflowError>> + Send>>;

/// Future for activity execution
pub type ActivityFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>>;

/// Future for timer
pub type TimerFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Classification of activity failure types
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityFailureType {
    ExecutionFailed,
    Panic,
    Retryable,
    NonRetryable,
    Application,
    Cancelled,
    Timeout(TimeoutType),
}

/// Timeout type for activity failures
#[derive(Debug, Clone, PartialEq)]
pub enum TimeoutType {
    StartToClose,
    ScheduleToStart,
    ScheduleToClose,
    Heartbeat,
}

/// Detailed information about an activity failure
#[derive(Debug, Clone)]
pub struct ActivityFailureInfo {
    /// Classification of the error type
    pub failure_type: ActivityFailureType,
    /// Human-readable error message
    pub message: String,
    /// Structured error details/payload (optional)
    pub details: Option<Vec<u8>>,
    /// Whether this error is retryable
    pub retryable: bool,
}

impl ActivityFailureInfo {
    /// Convert details bytes to a string representation
    pub fn details_as_string(&self) -> Option<String> {
        self.details
            .as_ref()
            .map(|d| String::from_utf8_lossy(d).to_string())
    }
}

impl fmt::Display for ActivityFailureInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Workflow error
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Workflow execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Non-deterministic workflow: {0}")]
    NonDeterministic(String),
    #[error("Workflow panicked: {0}")]
    Panic(String),
    #[error("Activity failed: {0}")]
    ActivityFailed(ActivityFailureInfo),
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
