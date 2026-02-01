//! Type-safe workflow and activity types.
//!
//! This module demonstrates best practices for type safety:
//! - Newtype patterns for IDs
//! - Validated input types
//! - Strongly-typed enums
//! - Builder patterns for complex types

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Newtype pattern for workflow IDs - prevents mixing up different ID types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WorkflowId(String);

impl WorkflowId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Newtype pattern for activity IDs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ActivityId(String);

impl ActivityId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Newtype pattern for user IDs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UserId(String);

impl UserId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated email type - ensures email format at construction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Email(String);

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("Invalid email format: {0}")]
    InvalidFormat(String),
}

impl Email {
    pub fn new(email: impl Into<String>) -> Result<Self, EmailError> {
        let email = email.into();
        // Basic validation - in production use a proper email validation library
        if email.contains('@') && email.contains('.') {
            Ok(Self(email))
        } else {
            Err(EmailError::InvalidFormat(email))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated amount type - prevents negative amounts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Amount(f64);

#[derive(Debug, Error)]
pub enum AmountError {
    #[error("Amount cannot be negative: {0}")]
    Negative(f64),
}

impl Amount {
    pub fn new(amount: f64) -> Result<Self, AmountError> {
        if amount >= 0.0 {
            Ok(Self(amount))
        } else {
            Err(AmountError::Negative(amount))
        }
    }

    pub fn value(&self) -> f64 {
        self.0
    }

    pub fn zero() -> Self {
        Self(0.0)
    }
}

/// Strongly-typed workflow status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed { error_code: String },
    Cancelled,
}

/// Workflow metadata with builder pattern support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub workflow_id: WorkflowId,
    pub workflow_type: String,
    pub status: WorkflowStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: WorkflowVersion,
}

/// Semantic versioning for workflows
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl WorkflowVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn as_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Builder pattern for WorkflowMetadata
pub struct WorkflowMetadataBuilder {
    workflow_id: Option<WorkflowId>,
    workflow_type: Option<String>,
    version: Option<WorkflowVersion>,
}

impl WorkflowMetadataBuilder {
    pub fn new() -> Self {
        Self {
            workflow_id: None,
            workflow_type: None,
            version: None,
        }
    }

    pub fn workflow_id(mut self, id: WorkflowId) -> Self {
        self.workflow_id = Some(id);
        self
    }

    pub fn workflow_type(mut self, workflow_type: impl Into<String>) -> Self {
        self.workflow_type = Some(workflow_type.into());
        self
    }

    pub fn version(mut self, version: WorkflowVersion) -> Self {
        self.version = Some(version);
        self
    }

    pub fn build(self) -> Result<WorkflowMetadata, String> {
        Ok(WorkflowMetadata {
            workflow_id: self.workflow_id.ok_or("workflow_id is required")?,
            workflow_type: self.workflow_type.ok_or("workflow_type is required")?,
            status: WorkflowStatus::Pending,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            version: self.version.ok_or("version is required")?,
        })
    }
}

/// Idempotency key for exactly-once semantics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type-safe result wrapper for activity results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedActivityResult<T> {
    pub idempotency_key: IdempotencyKey,
    pub data: T,
    pub processed_at: chrono::DateTime<chrono::Utc>,
}

impl<T> TypedActivityResult<T> {
    pub fn new(data: T) -> Self {
        Self {
            idempotency_key: IdempotencyKey::generate(),
            data,
            processed_at: chrono::Utc::now(),
        }
    }
}

/// Context propagation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub baggage: std::collections::HashMap<String, String>,
}

impl RequestContext {
    pub fn new(trace_id: impl Into<String>, span_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            parent_span_id: None,
            baggage: std::collections::HashMap::new(),
        }
    }

    pub fn with_parent(mut self, parent_span_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_span_id.into());
        self
    }

    pub fn with_baggage(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.baggage.insert(key.into(), value.into());
        self
    }
}
