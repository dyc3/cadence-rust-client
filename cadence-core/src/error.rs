//! Error types for the Cadence client.
//!
//! This module defines all error types that can occur when working with
//! Cadence workflows and activities.

use std::fmt;
use thiserror::Error;

/// Custom error type for workflow-defined errors
#[derive(Debug, Clone, Error)]
#[error("CustomError: reason={reason}, details={details:?}")]
pub struct CustomError {
    pub reason: String,
    pub details: Vec<u8>,
}

impl CustomError {
    pub fn new(reason: impl Into<String>, details: Vec<u8>) -> Self {
        Self {
            reason: reason.into(),
            details,
        }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn details(&self) -> &[u8] {
        &self.details
    }
}

/// Error type for canceled operations
#[derive(Debug, Clone, Error)]
#[error("CanceledError: details={details:?}")]
pub struct CanceledError {
    pub details: Vec<u8>,
}

impl CanceledError {
    pub fn new(details: Vec<u8>) -> Self {
        Self { details }
    }

    pub fn details(&self) -> &[u8] {
        &self.details
    }
}

/// Error type for timeouts
#[derive(Debug, Clone, Error)]
#[error("TimeoutError: timeout_type={timeout_type:?}, details={details:?}")]
pub struct TimeoutError {
    pub timeout_type: TimeoutType,
    pub details: Vec<u8>,
    pub last_heartbeat_details: Vec<u8>,
}

impl TimeoutError {
    pub fn new(
        timeout_type: TimeoutType,
        details: Vec<u8>,
        last_heartbeat_details: Vec<u8>,
    ) -> Self {
        Self {
            timeout_type,
            details,
            last_heartbeat_details,
        }
    }

    pub fn timeout_type(&self) -> TimeoutType {
        self.timeout_type
    }

    pub fn details(&self) -> &[u8] {
        &self.details
    }

    pub fn last_heartbeat_details(&self) -> &[u8] {
        &self.last_heartbeat_details
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutType {
    StartToClose,
    ScheduleToStart,
    ScheduleToClose,
    Heartbeat,
}

impl fmt::Display for TimeoutType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeoutType::StartToClose => write!(f, "START_TO_CLOSE"),
            TimeoutType::ScheduleToStart => write!(f, "SCHEDULE_TO_START"),
            TimeoutType::ScheduleToClose => write!(f, "SCHEDULE_TO_CLOSE"),
            TimeoutType::Heartbeat => write!(f, "HEARTBEAT"),
        }
    }
}

/// Error type for terminated workflows
#[derive(Debug, Clone, Error)]
#[error("TerminatedError: details={details:?}")]
pub struct TerminatedError {
    pub details: Vec<u8>,
}

impl TerminatedError {
    pub fn new(details: Vec<u8>) -> Self {
        Self { details }
    }

    pub fn details(&self) -> &[u8] {
        &self.details
    }
}

/// Generic workflow error
#[derive(Debug, Clone, Error)]
#[error("GenericError: message={message}")]
pub struct GenericError {
    pub message: String,
}

impl GenericError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Reason for non-determinism error
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum NonDeterminismReason {
    #[error("missing replay decision")]
    MissingReplayDecision,
    #[error("extra replay decision")]
    ExtraReplayDecision,
    #[error("mismatch")]
    Mismatch,
}

/// Error type for non-deterministic workflow execution
///
/// TODO: explain what non-determinism is in this context
/// TODO: Should this just have `ReplayContext` instead?
/// FIXME: why so many strings? could mean we are losing type info, hurting debuggability or error recovery
#[derive(Debug, Clone, Error)]
#[error("NonDeterministicError: reason={reason}, workflow_type={workflow_type}, workflow_id={workflow_id}")]
pub struct NonDeterministicError {
    pub reason: NonDeterminismReason,
    pub workflow_type: String,
    pub workflow_id: String,
    pub run_id: String,
    pub task_list: String,
    pub domain_name: String,
    pub history_event_text: Option<String>,
    pub decision_text: Option<String>,
}

/// Error type for panics in workflows
#[derive(Debug, Error)]
#[error("PanicError: message={message}, stack_trace={stack_trace}")]
pub struct PanicError {
    pub message: String,
    /// TODO: use `Backtrace` instead?
    pub stack_trace: String,
}

impl PanicError {
    pub fn new(message: impl Into<String>, stack_trace: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            stack_trace: stack_trace.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn stack_trace(&self) -> &str {
        &self.stack_trace
    }
}

/// Error for unknown external workflow executions
#[derive(Debug, Clone, Error)]
#[error("UnknownExternalWorkflowExecutionError")]
pub struct UnknownExternalWorkflowExecutionError;

/// Continue-as-new error - signals that workflow should continue with new execution
#[derive(Debug, Clone, Error)]
#[error("ContinueAsNewError: workflow_type={workflow_type}, task_list={task_list}")]
pub struct ContinueAsNewError {
    pub workflow_type: String,
    pub task_list: String,
    pub input: Vec<u8>,
    pub execution_start_to_close_timeout_seconds: i32,
    pub task_start_to_close_timeout_seconds: i32,
}

impl ContinueAsNewError {
    pub fn new(
        workflow_type: impl Into<String>,
        task_list: impl Into<String>,
        input: Vec<u8>,
        execution_start_to_close_timeout_seconds: i32,
        task_start_to_close_timeout_seconds: i32,
    ) -> Self {
        Self {
            workflow_type: workflow_type.into(),
            task_list: task_list.into(),
            input,
            execution_start_to_close_timeout_seconds,
            task_start_to_close_timeout_seconds,
        }
    }

    pub fn workflow_type(&self) -> &str {
        &self.workflow_type
    }

    pub fn task_list(&self) -> &str {
        &self.task_list
    }

    pub fn input(&self) -> &[u8] {
        &self.input
    }

    pub fn execution_start_to_close_timeout_seconds(&self) -> i32 {
        self.execution_start_to_close_timeout_seconds
    }

    pub fn task_start_to_close_timeout_seconds(&self) -> i32 {
        self.task_start_to_close_timeout_seconds
    }
}

/// Result pending error - for async activity completion
#[derive(Debug, Clone, Error)]
#[error("ErrResultPending")]
pub struct ErrResultPending;

/// Server error types
#[derive(Debug, Clone, Error)]
pub enum ServerError {
    #[error("EntityNotExistsError: {message}")]
    EntityNotExists { message: String },

    #[error("BadRequestError: {message}")]
    BadRequest { message: String },

    #[error("WorkflowExecutionAlreadyStartedError: {message}")]
    WorkflowExecutionAlreadyStarted { message: String },

    #[error("WorkflowExecutionAlreadyCompletedError: {message}")]
    WorkflowExecutionAlreadyCompleted { message: String },

    #[error("DomainAlreadyExistsError: {message}")]
    DomainAlreadyExists { message: String },

    #[error("DomainNotActiveError: {message}")]
    DomainNotActive { message: String },

    #[error("ServiceBusyError: {message}")]
    ServiceBusy { message: String },

    #[error("InternalServiceError: {message}")]
    InternalService { message: String },

    #[error("QueryFailedError: {message}")]
    QueryFailed { message: String },

    #[error("ClientVersionNotSupportedError: {message}")]
    ClientVersionNotSupported { message: String },

    #[error("CancellationAlreadyRequestedError: {message}")]
    CancellationAlreadyRequested { message: String },
}

/// Main Cadence error type that encompasses all errors
///
/// TODO: docstrings for each variant
#[derive(Debug, Error)]
pub enum CadenceError {
    #[error(transparent)]
    Custom(#[from] CustomError),

    #[error(transparent)]
    Canceled(#[from] CanceledError),

    #[error(transparent)]
    Timeout(#[from] TimeoutError),

    #[error(transparent)]
    Terminated(#[from] TerminatedError),

    #[error(transparent)]
    Generic(#[from] GenericError),

    #[error(transparent)]
    Panic(#[from] PanicError),

    #[error(transparent)]
    UnknownExternalWorkflowExecution(#[from] UnknownExternalWorkflowExecutionError),

    #[error(transparent)]
    ContinueAsNew(#[from] ContinueAsNewError),

    #[error(transparent)]
    NonDeterministic(#[from] NonDeterministicError),

    #[error(transparent)]
    Server(#[from] ServerError),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Client error: {0}")]
    ClientError(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Unauthorized(String),

    #[error("Workflow execution failed: {0}, details: {1:?}")]
    WorkflowExecutionFailed(String, Vec<u8>),

    #[error("Workflow execution timed out")]
    WorkflowExecutionTimedOut,

    #[error("Workflow execution cancelled")]
    WorkflowExecutionCancelled,

    #[error("Workflow execution terminated")]
    WorkflowExecutionTerminated,

    #[error("Other error: {0}")]
    Other(String),
}

pub type CadenceResult<T> = Result<T, CadenceError>;

/// Helper functions for creating errors
pub mod factory {
    use super::*;

    pub fn custom_error(reason: impl Into<String>, details: Vec<u8>) -> CustomError {
        CustomError::new(reason, details)
    }

    pub fn canceled_error(details: Vec<u8>) -> CanceledError {
        CanceledError::new(details)
    }

    pub fn timeout_error(
        timeout_type: TimeoutType,
        details: Vec<u8>,
        last_heartbeat_details: Vec<u8>,
    ) -> TimeoutError {
        TimeoutError::new(timeout_type, details, last_heartbeat_details)
    }

    pub fn terminated_error(details: Vec<u8>) -> TerminatedError {
        TerminatedError::new(details)
    }

    pub fn generic_error(message: impl Into<String>) -> GenericError {
        GenericError::new(message)
    }

    pub fn panic_error(message: impl Into<String>, stack_trace: impl Into<String>) -> PanicError {
        PanicError::new(message, stack_trace)
    }

    pub fn unknown_external_workflow_execution_error() -> UnknownExternalWorkflowExecutionError {
        UnknownExternalWorkflowExecutionError
    }

    pub fn continue_as_new_error(
        workflow_type: impl Into<String>,
        task_list: impl Into<String>,
        input: Vec<u8>,
        execution_start_to_close_timeout_seconds: i32,
        task_start_to_close_timeout_seconds: i32,
    ) -> ContinueAsNewError {
        ContinueAsNewError::new(
            workflow_type,
            task_list,
            input,
            execution_start_to_close_timeout_seconds,
            task_start_to_close_timeout_seconds,
        )
    }

    #[expect(clippy::too_many_arguments)]
    pub fn non_deterministic_error(
        reason: NonDeterminismReason,
        workflow_type: impl Into<String>,
        workflow_id: impl Into<String>,
        run_id: impl Into<String>,
        task_list: impl Into<String>,
        domain_name: impl Into<String>,
        history_event_text: Option<String>,
        decision_text: Option<String>,
    ) -> NonDeterministicError {
        NonDeterministicError {
            reason,
            workflow_type: workflow_type.into(),
            workflow_id: workflow_id.into(),
            run_id: run_id.into(),
            task_list: task_list.into(),
            domain_name: domain_name.into(),
            history_event_text,
            decision_text,
        }
    }
}

/// Helper functions to check error types
pub fn is_custom_error(err: &CadenceError) -> bool {
    matches!(err, CadenceError::Custom(_))
}

pub fn is_canceled_error(err: &CadenceError) -> bool {
    matches!(err, CadenceError::Canceled(_))
}

pub fn is_timeout_error(err: &CadenceError) -> bool {
    matches!(err, CadenceError::Timeout(_))
}

pub fn is_terminated_error(err: &CadenceError) -> bool {
    matches!(err, CadenceError::Terminated(_))
}

pub fn is_continue_as_new_error(err: &CadenceError) -> bool {
    matches!(err, CadenceError::ContinueAsNew(_))
}

pub fn is_panic_error(err: &CadenceError) -> bool {
    matches!(err, CadenceError::Panic(_))
}

pub fn is_generic_error(err: &CadenceError) -> bool {
    matches!(err, CadenceError::Generic(_))
}

pub fn is_entity_not_exists_error(err: &CadenceError) -> bool {
    matches!(
        err,
        CadenceError::Server(ServerError::EntityNotExists { .. })
    )
}

pub fn is_workflow_execution_already_started_error(err: &CadenceError) -> bool {
    matches!(
        err,
        CadenceError::Server(ServerError::WorkflowExecutionAlreadyStarted { .. })
    )
}

pub fn is_non_deterministic_error(err: &CadenceError) -> bool {
    matches!(err, CadenceError::NonDeterministic(_))
}
