//! Core types for Cadence client.
//!
//! This module defines the main types used throughout the client for
//! workflow execution, task management, and configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Retry policy for activities and workflows
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Initial retry interval
    pub initial_interval: Duration,
    /// Backoff coefficient (e.g., 2.0 for exponential)
    pub backoff_coefficient: f64,
    /// Maximum retry interval
    pub maximum_interval: Duration,
    /// Maximum number of attempts
    pub maximum_attempts: i32,
    /// Non-retryable error types
    pub non_retryable_error_types: Vec<String>,
    /// Expiration interval
    pub expiration_interval: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_interval: Duration::from_secs(1),
            backoff_coefficient: 2.0,
            maximum_interval: Duration::from_secs(100),
            maximum_attempts: 0, // Unlimited
            non_retryable_error_types: vec![],
            expiration_interval: Duration::from_secs(0),
        }
    }
}

/// Workflow ID reuse policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(i32)]
pub enum WorkflowIdReusePolicy {
    /// Allow starting a workflow execution when the last execution close state
    /// is in [terminated, cancelled, timeout, failed]
    #[default]
    AllowDuplicateFailedOnly = 0,
    /// Allow starting a workflow execution using the same workflow ID when workflow is not running
    AllowDuplicate = 1,
    /// Do not allow starting a workflow execution using the same workflow ID at all
    RejectDuplicate = 2,
    /// Terminate current running workflow using the same workflow ID if exists,
    /// then start a new run in one transaction
    TerminateIfRunning = 3,
}

/// Parent close policy for child workflows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(i32)]
pub enum ParentClosePolicy {
    /// Terminate the child workflow
    #[default]
    Terminate = 0,
    /// Request cancellation on the child workflow
    RequestCancel = 1,
    /// Do nothing on the child workflow
    Abandon = 2,
}

/// Query consistency level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(i32)]
pub enum QueryConsistencyLevel {
    /// Use default consistency level provided by cluster
    #[default]
    Unspecified = 0,
    /// Pass request to receiving cluster, return eventually consistent results
    Eventual = 1,
    /// Redirect to active cluster, return strongly consistent results
    Strong = 2,
}

/// Activity options for scheduling activities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityOptions {
    /// Task list to schedule activity on
    pub task_list: String,
    /// Schedule to close timeout
    pub schedule_to_close_timeout: Duration,
    /// Schedule to start timeout
    pub schedule_to_start_timeout: Duration,
    /// Start to close timeout
    pub start_to_close_timeout: Duration,
    /// Heartbeat timeout
    pub heartbeat_timeout: Duration,
    /// Wait for cancellation before completing
    pub wait_for_cancellation: bool,
    /// Retry policy
    pub retry_policy: Option<RetryPolicy>,
    /// Local activity flag
    pub local_activity: bool,
}

impl Default for ActivityOptions {
    fn default() -> Self {
        Self {
            task_list: String::new(),
            schedule_to_close_timeout: Duration::from_secs(0),
            schedule_to_start_timeout: Duration::from_secs(0),
            start_to_close_timeout: Duration::from_secs(0),
            heartbeat_timeout: Duration::from_secs(0),
            wait_for_cancellation: false,
            retry_policy: None,
            local_activity: false,
        }
    }
}

/// Child workflow options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildWorkflowOptions {
    /// Domain for child workflow (defaults to parent domain)
    pub domain: Option<String>,
    /// Workflow ID for child (auto-generated if not specified)
    pub workflow_id: String,
    /// Task list to schedule child workflow on
    pub task_list: Option<String>,
    /// Retry policy
    pub retry_policy: Option<RetryPolicy>,
    /// Cron schedule
    pub cron_schedule: Option<String>,
    /// Execution start to close timeout
    pub execution_start_to_close_timeout: Duration,
    /// Task start to close timeout
    pub task_start_to_close_timeout: Duration,
    /// Wait for cancellation before completing
    pub wait_for_cancellation: bool,
    /// Workflow ID reuse policy
    pub workflow_id_reuse_policy: WorkflowIdReusePolicy,
    /// Parent close policy
    pub parent_close_policy: ParentClosePolicy,
    /// Memo
    pub memo: Option<HashMap<String, Vec<u8>>>,
    /// Search attributes
    pub search_attributes: Option<HashMap<String, Vec<u8>>>,
}

impl Default for ChildWorkflowOptions {
    fn default() -> Self {
        Self {
            domain: None,
            workflow_id: String::new(),
            task_list: None,
            retry_policy: None,
            cron_schedule: None,
            execution_start_to_close_timeout: Duration::from_secs(0),
            task_start_to_close_timeout: Duration::from_secs(0),
            wait_for_cancellation: false,
            workflow_id_reuse_policy: WorkflowIdReusePolicy::default(),
            parent_close_policy: ParentClosePolicy::default(),
            memo: None,
            search_attributes: None,
        }
    }
}

/// Activity information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityInfo {
    pub activity_id: String,
    pub activity_type: String,
    pub task_token: Vec<u8>,
    pub workflow_execution: WorkflowExecution,
    pub attempt: i32,
    pub scheduled_time: chrono::DateTime<chrono::Utc>,
    pub started_time: chrono::DateTime<chrono::Utc>,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub heartbeat_timeout: Duration,
}

/// Workflow execution identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub workflow_id: String,
    pub run_id: String,
}

impl WorkflowExecution {
    pub fn new(workflow_id: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            run_id: run_id.into(),
        }
    }
}

/// Workflow type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowType {
    pub name: String,
}

/// Activity type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityType {
    pub name: String,
}

/// Task list
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskList {
    pub name: String,
    pub kind: TaskListKind,
}

impl TaskList {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: TaskListKind::Normal,
        }
    }

    pub fn sticky(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: TaskListKind::Sticky,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum TaskListKind {
    Normal = 0,
    Sticky = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum TaskListType {
    Decision = 0,
    Activity = 1,
}

/// Workflow information available in workflow context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub workflow_execution: WorkflowExecution,
    pub workflow_type: WorkflowType,
    pub task_list: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub execution_start_to_close_timeout: Duration,
    pub task_start_to_close_timeout: Duration,
    pub attempt: i32,
    pub continued_execution_run_id: Option<String>,
    pub parent_workflow_execution: Option<WorkflowExecution>,
    pub cron_schedule: Option<String>,
    pub memo: Option<HashMap<String, Vec<u8>>>,
    pub search_attributes: Option<HashMap<String, Vec<u8>>>,
}

/// Header for passing context information
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Header {
    pub fields: HashMap<String, Vec<u8>>,
}

/// Memo for workflow data
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Memo {
    pub fields: HashMap<String, Vec<u8>>,
}

/// Search attributes for workflow visibility
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SearchAttributes {
    pub indexed_fields: HashMap<String, Vec<u8>>,
}

/// Domain information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainInfo {
    pub name: String,
    pub status: DomainStatus,
    pub description: String,
    pub owner_email: String,
    pub data: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum DomainStatus {
    Registered = 0,
    Deprecated = 1,
    Deleted = 2,
}

/// Domain configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainConfiguration {
    pub workflow_execution_retention_period: Duration,
    pub emit_metric: bool,
}

/// Cluster attribute for active-active domains
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterAttribute {
    pub scope: String,
    pub name: String,
}

/// Active cluster selection policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveClusterSelectionPolicy {
    pub cluster_attribute: Option<ClusterAttribute>,
}

/// Feature flags for client behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub enable_execution_cache: bool,
    pub enable_async_workflow_consistency: bool,
}

/// Non-deterministic workflow policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NonDeterministicWorkflowPolicy {
    #[default]
    BlockWorkflow,
    FailWorkflow,
}

/// Shadow mode for workflow shadower
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShadowMode {
    Normal,
    Continuous,
}

/// Time filter for workflow queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeFilter {
    pub min_time: chrono::DateTime<chrono::Utc>,
    pub max_time: chrono::DateTime<chrono::Utc>,
}

/// Shadow exit condition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadowExitCondition {
    pub expiration_interval: Duration,
}

/// Query types for workflow queries
pub const QUERY_TYPE_STACK_TRACE: &str = "__stack_trace";
pub const QUERY_TYPE_OPEN_SESSIONS: &str = "__open_sessions";
pub const QUERY_TYPE_QUERY_TYPES: &str = "__query_types";

/// Registration information for workflows and activities
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryInfo {
    pub name: String,
    pub fn_name: String,
}

/// Session information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub task_list: String,
}

/// Worker identity information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerIdentity {
    pub name: String,
    pub version: String,
}

impl Default for WorkerIdentity {
    fn default() -> Self {
        Self {
            name: format!(
                "cadence-rust-worker@{}-pid-{}",
                std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()),
                std::process::id()
            ),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
