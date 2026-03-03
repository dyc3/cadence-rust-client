//! Workflow futures.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crabdance_core::GenericError;
use crabdance_proto::shared::TimeoutType as ProtoTimeoutType;
use std::error::Error;

/// Future for child workflow execution
pub type ChildWorkflowFuture =
    Pin<Box<dyn Future<Output = Result<Vec<u8>, DefaultWorkflowError>> + Send>>;

/// Future for activity execution
pub type ActivityFuture =
    Pin<Box<dyn Future<Output = Result<Vec<u8>, DefaultActivityError>> + Send>>;

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

/// Boxed error type for workflow/activity failures
pub type BoxError = Arc<dyn Error + Send + Sync>;

/// Convert a message into a boxed error
pub fn boxed_error(message: impl Into<String>) -> BoxError {
    Arc::new(GenericError::new(message))
}

pub fn boxed_error_from<E>(error: E) -> BoxError
where
    E: Error + Send + Sync + 'static,
{
    Arc::new(error)
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("Workflow execution failed: {message}")]
pub struct WorkflowFailureMessage {
    message: String,
}

impl WorkflowFailureMessage {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
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
#[derive(Debug, Clone, thiserror::Error)]
pub enum WorkflowError<E = BoxError>
where
    E: fmt::Display + fmt::Debug + Send + Sync + Clone + 'static,
{
    #[error("Workflow execution failed: {0}")]
    ExecutionFailed(E),
    #[error("Non-deterministic workflow: {0}")]
    NonDeterministic(String),
    #[error("Workflow panicked: {0}")]
    Panic(String),
    #[error("Activity failed: {0}")]
    ActivityFailed(ActivityFailureInfo),
    #[error("Child workflow failed: {0}")]
    ChildWorkflowFailed(E),
    #[error("Signal failed: {0}")]
    SignalFailed(E),
    #[error("Cancel failed: {0}")]
    CancelFailed(E),
    #[error("Continue as new")]
    ContinueAsNew,
    #[error("Workflow cancelled")]
    Cancelled,
    #[error("Generic error: {0}")]
    Generic(E),
}

pub type DefaultWorkflowError = WorkflowError<BoxError>;

impl WorkflowError<BoxError> {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Generic(boxed_error(message))
    }

    pub fn execution_failed(message: impl Into<String>) -> Self {
        Self::ExecutionFailed(Arc::new(WorkflowFailureMessage::new(message)))
    }

    pub fn execution_failed_error<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::ExecutionFailed(boxed_error_from(error))
    }

    pub fn generic_error<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::Generic(boxed_error_from(error))
    }

    pub fn child_workflow_failed(message: impl Into<String>) -> Self {
        Self::ChildWorkflowFailed(boxed_error(message))
    }

    pub fn child_workflow_failed_error<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::ChildWorkflowFailed(boxed_error_from(error))
    }

    pub fn signal_failed(message: impl Into<String>) -> Self {
        Self::SignalFailed(boxed_error(message))
    }

    pub fn cancel_failed(message: impl Into<String>) -> Self {
        Self::CancelFailed(boxed_error(message))
    }
}

/// Activity error
#[derive(Debug, Clone, thiserror::Error)]
pub enum ActivityError<E = BoxError>
where
    E: fmt::Display + fmt::Debug + Send + Sync + Clone + 'static,
{
    #[error("Activity execution failed: {0}")]
    ExecutionFailed(E),
    #[error("Activity panicked: {0}")]
    Panic(E),
    #[error("Retryable activity error: {0}")]
    Retryable(E),
    #[error("Non-retryable activity error: {0}")]
    NonRetryable(E),
    #[error("Application error: {0}")]
    Application(E),
    #[error("Retryable with delay: {0}ms")]
    RetryableWithDelay(E, u64),
    #[error("Activity cancelled")]
    Cancelled,
    #[error("Activity timed out: {0:?}")]
    Timeout(ProtoTimeoutType),
}

pub type DefaultActivityError = ActivityError<BoxError>;

impl ActivityError<BoxError> {
    /// Create a retryable error
    pub fn retryable(message: impl Into<String>) -> Self {
        Self::Retryable(boxed_error(message))
    }

    /// Create a non-retryable error
    pub fn non_retryable(message: impl Into<String>) -> Self {
        Self::NonRetryable(boxed_error(message))
    }

    /// Create an application error
    pub fn application(message: impl Into<String>) -> Self {
        Self::Application(boxed_error(message))
    }

    /// Create an execution failed error
    pub fn execution_failed(message: impl Into<String>) -> Self {
        Self::ExecutionFailed(boxed_error(message))
    }

    /// Create an execution failed error from a typed error
    pub fn execution_failed_error<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::ExecutionFailed(boxed_error_from(error))
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_) | Self::RetryableWithDelay(_, _))
    }
}
