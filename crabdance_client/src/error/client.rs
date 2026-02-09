//! Client-layer error types for workflow operations and validation.
//!
//! This module defines errors that occur at the client level when
//! performing workflow operations, including validation errors,
//! workflow not found errors, and query-related errors.

use crabdance_core::EncodingError;
use thiserror::Error;

use super::TransportError;

/// Client-level errors for workflow operations.
///
/// These errors represent validation failures, workflow state issues,
/// and client-level operation failures. They wrap transport errors
/// and encoding errors to provide a complete error hierarchy.
///
/// # Examples
///
/// ```no_run
/// use crabdance_client::error::ClientError;
///
/// // Match on specific error types
/// match result {
///     Err(ClientError::InvalidWorkflowId(id)) => {
///         eprintln!("Invalid workflow ID: {}", id);
///     },
///     Err(ClientError::WorkflowNotFound { workflow_id, .. }) => {
///         eprintln!("Workflow not found: {}", workflow_id);
///     },
///     Err(ClientError::QueryRejected { reason, .. }) => {
///         eprintln!("Query rejected: {}", reason);
///     },
///     _ => {}
/// }
/// ```
#[derive(Debug, Error)]
pub enum ClientError {
    /// Invalid workflow ID provided
    #[error("Invalid workflow ID: {0}")]
    InvalidWorkflowId(String),

    /// Invalid run ID provided
    #[error("Invalid run ID: {0}")]
    InvalidRunId(String),

    /// Empty task token provided
    #[error("Empty task token")]
    EmptyTaskToken,

    /// Workflow execution not found
    #[error("Workflow not found: workflow_id={workflow_id}, run_id={run_id:?}")]
    WorkflowNotFound {
        workflow_id: String,
        run_id: Option<String>,
    },

    /// Query was rejected by the workflow
    #[error("Query rejected: workflow status={status:?}, reason={reason}")]
    QueryRejected { status: String, reason: String },

    /// Error during pagination of results
    #[error("Pagination error: {0}")]
    PaginationError(String),

    /// Timeout waiting for workflow result
    #[error("Timeout waiting for workflow result")]
    WorkflowTimeout,

    /// Invalid argument provided to operation
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Domain not found or invalid
    #[error("Domain error: {0}")]
    DomainError(String),

    /// Transport-layer error
    #[error(transparent)]
    Transport(#[from] TransportError),

    /// Serialization/deserialization error
    #[error(transparent)]
    Encoding(#[from] EncodingError),
}

impl ClientError {
    /// Create a workflow not found error
    pub fn workflow_not_found(workflow_id: impl Into<String>, run_id: Option<String>) -> Self {
        ClientError::WorkflowNotFound {
            workflow_id: workflow_id.into(),
            run_id,
        }
    }

    /// Create a query rejected error
    pub fn query_rejected(status: impl Into<String>, reason: impl Into<String>) -> Self {
        ClientError::QueryRejected {
            status: status.into(),
            reason: reason.into(),
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            ClientError::Transport(e) => e.is_retryable(),
            ClientError::WorkflowTimeout => true,
            _ => false,
        }
    }
}
