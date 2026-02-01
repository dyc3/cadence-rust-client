//! Shared types used across the Cadence protocol.
//!
//! These types are generated from Thrift definitions and represent the
//! core data structures for workflow execution, history events, and more.

use serde::{Deserialize, Serialize};

/// Unique identifier for a workflow execution
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowExecution {
    /// The workflow ID (user-defined or system-generated)
    pub workflow_id: String,
    /// The run ID (unique for each run of a workflow)
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

/// Workflow type information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowType {
    pub name: String,
}

/// Activity type information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityType {
    pub name: String,
}

/// Task list identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskList {
    pub name: String,
    pub kind: TaskListKind,
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

/// Represents a single event in workflow history
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEvent {
    pub event_id: i64,
    pub timestamp: i64,
    pub event_type: EventType,
    pub version: i64,
    pub task_id: i64,
    pub attributes: Option<EventAttributes>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum EventType {
    WorkflowExecutionStarted = 0,
    WorkflowExecutionCompleted = 1,
    WorkflowExecutionFailed = 2,
    WorkflowExecutionTimedOut = 3,
    DecisionTaskScheduled = 4,
    DecisionTaskStarted = 5,
    DecisionTaskCompleted = 6,
    DecisionTaskTimedOut = 7,
    DecisionTaskFailed = 8,
    ActivityTaskScheduled = 9,
    ActivityTaskStarted = 10,
    ActivityTaskCompleted = 11,
    ActivityTaskFailed = 12,
    ActivityTaskTimedOut = 13,
    ActivityTaskCancelRequested = 14,
    ActivityTaskCanceled = 15,
    TimerStarted = 16,
    TimerFired = 17,
    TimerCanceled = 18,
    WorkflowExecutionCancelRequested = 19,
    WorkflowExecutionCanceled = 20,
    RequestCancelExternalWorkflowExecutionInitiated = 21,
    RequestCancelExternalWorkflowExecutionFailed = 22,
    ExternalWorkflowExecutionCancelRequested = 23,
    MarkerRecorded = 24,
    WorkflowExecutionSignaled = 25,
    WorkflowExecutionTerminated = 26,
    SignalExternalWorkflowExecutionInitiated = 27,
    SignalExternalWorkflowExecutionFailed = 28,
    ExternalWorkflowExecutionSignaled = 29,
    UpsertWorkflowSearchAttributes = 30,
    StartChildWorkflowExecutionInitiated = 35,
    ChildWorkflowExecutionStarted = 36,
    ChildWorkflowExecutionCompleted = 37,
    ChildWorkflowExecutionFailed = 38,
    ChildWorkflowExecutionTimedOut = 39,
    ChildWorkflowExecutionCanceled = 40,
    ChildWorkflowExecutionTerminated = 41,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventAttributes {
    WorkflowExecutionStartedEventAttributes(Box<WorkflowExecutionStartedEventAttributes>),
    WorkflowExecutionCompletedEventAttributes(Box<WorkflowExecutionCompletedEventAttributes>),
    WorkflowExecutionFailedEventAttributes(Box<WorkflowExecutionFailedEventAttributes>),
    WorkflowExecutionTimedOutEventAttributes(Box<WorkflowExecutionTimedOutEventAttributes>),
    DecisionTaskScheduledEventAttributes(Box<DecisionTaskScheduledEventAttributes>),
    DecisionTaskStartedEventAttributes(Box<DecisionTaskStartedEventAttributes>),
    DecisionTaskCompletedEventAttributes(Box<DecisionTaskCompletedEventAttributes>),
    ActivityTaskScheduledEventAttributes(Box<ActivityTaskScheduledEventAttributes>),
    ActivityTaskStartedEventAttributes(Box<ActivityTaskStartedEventAttributes>),
    ActivityTaskCompletedEventAttributes(Box<ActivityTaskCompletedEventAttributes>),
    ActivityTaskFailedEventAttributes(Box<ActivityTaskFailedEventAttributes>),
    ActivityTaskTimedOutEventAttributes(Box<ActivityTaskTimedOutEventAttributes>),
    TimerStartedEventAttributes(Box<TimerStartedEventAttributes>),
    TimerFiredEventAttributes(Box<TimerFiredEventAttributes>),
    WorkflowExecutionCancelRequestedEventAttributes(
        Box<WorkflowExecutionCancelRequestedEventAttributes>,
    ),
    WorkflowExecutionCanceledEventAttributes(Box<WorkflowExecutionCanceledEventAttributes>),
    MarkerRecordedEventAttributes(Box<MarkerRecordedEventAttributes>),
    WorkflowExecutionSignaledEventAttributes(Box<WorkflowExecutionSignaledEventAttributes>),
    SignalExternalWorkflowExecutionInitiatedEventAttributes(
        Box<SignalExternalWorkflowExecutionInitiatedEventAttributes>,
    ),
    ChildWorkflowExecutionStartedEventAttributes(Box<ChildWorkflowExecutionStartedEventAttributes>),
    ChildWorkflowExecutionCompletedEventAttributes(
        Box<ChildWorkflowExecutionCompletedEventAttributes>,
    ),
    ChildWorkflowExecutionFailedEventAttributes(Box<ChildWorkflowExecutionFailedEventAttributes>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExecutionStartedEventAttributes {
    pub workflow_type: Option<WorkflowType>,
    pub parent_workflow_execution: Option<WorkflowExecution>,
    pub task_list: Option<TaskList>,
    pub input: Vec<u8>,
    pub execution_start_to_close_timeout_seconds: i32,
    pub task_start_to_close_timeout_seconds: i32,
    pub identity: String,
    pub continued_execution_run_id: Option<String>,
    pub initiator: Option<ContinueAsNewInitiator>,
    pub continued_failure_details: Option<Vec<u8>>,
    pub last_completion_result: Option<Vec<u8>>,
    pub original_execution_run_id: Option<String>,
    pub first_execution_run_id: Option<String>,
    pub retry_policy: Option<RetryPolicy>,
    pub attempt: i32,
    pub expiration_timestamp: Option<i64>,
    pub cron_schedule: Option<String>,
    pub first_decision_task_backoff_seconds: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExecutionCompletedEventAttributes {
    pub result: Option<Vec<u8>>,
    pub decision_task_completed_event_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExecutionFailedEventAttributes {
    pub reason: Option<String>,
    pub details: Option<Vec<u8>>,
    pub decision_task_completed_event_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExecutionTimedOutEventAttributes {
    pub timeout_type: Option<TimeoutType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionTaskScheduledEventAttributes {
    pub task_list: Option<TaskList>,
    pub start_to_close_timeout_seconds: i32,
    pub attempt: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionTaskStartedEventAttributes {
    pub scheduled_event_id: i64,
    pub identity: String,
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionTaskCompletedEventAttributes {
    pub scheduled_event_id: i64,
    pub started_event_id: i64,
    pub identity: String,
    pub binary_checksum: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityTaskScheduledEventAttributes {
    pub activity_id: String,
    pub activity_type: Option<ActivityType>,
    pub task_list: Option<TaskList>,
    pub input: Option<Vec<u8>>,
    pub schedule_to_close_timeout_seconds: Option<i32>,
    pub schedule_to_start_timeout_seconds: Option<i32>,
    pub start_to_close_timeout_seconds: Option<i32>,
    pub heartbeat_timeout_seconds: Option<i32>,
    pub decision_task_completed_event_id: i64,
    pub retry_policy: Option<RetryPolicy>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityTaskStartedEventAttributes {
    pub scheduled_event_id: i64,
    pub identity: String,
    pub request_id: String,
    pub attempt: i32,
    pub last_failure_details: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityTaskCompletedEventAttributes {
    pub result: Option<Vec<u8>>,
    pub scheduled_event_id: i64,
    pub started_event_id: i64,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityTaskFailedEventAttributes {
    pub reason: Option<String>,
    pub details: Option<Vec<u8>>,
    pub scheduled_event_id: i64,
    pub started_event_id: i64,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityTaskTimedOutEventAttributes {
    pub details: Option<Vec<u8>>,
    pub scheduled_event_id: i64,
    pub started_event_id: i64,
    pub timeout_type: TimeoutType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimerStartedEventAttributes {
    pub timer_id: String,
    pub start_to_fire_timeout_seconds: i64,
    pub decision_task_completed_event_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimerFiredEventAttributes {
    pub timer_id: String,
    pub started_event_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExecutionCancelRequestedEventAttributes {
    pub cause: Option<String>,
    pub external_initiated_execution: Option<WorkflowExecution>,
    pub external_workflow_execution: Option<WorkflowExecution>,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExecutionCanceledEventAttributes {
    pub details: Option<Vec<u8>>,
    pub decision_task_completed_event_id: i64,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerRecordedEventAttributes {
    pub marker_name: String,
    pub details: Option<Vec<u8>>,
    pub decision_task_completed_event_id: i64,
    pub header: Option<Header>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExecutionSignaledEventAttributes {
    pub signal_name: String,
    pub input: Option<Vec<u8>>,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalExternalWorkflowExecutionInitiatedEventAttributes {
    pub workflow_id: String,
    pub run_id: Option<String>,
    pub signal_name: String,
    pub input: Option<Vec<u8>>,
    pub decision_task_completed_event_id: i64,
    pub control: Option<String>,
    pub child_workflow_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChildWorkflowExecutionStartedEventAttributes {
    pub workflow_execution: Option<WorkflowExecution>,
    pub workflow_type: Option<WorkflowType>,
    pub initiated_event_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChildWorkflowExecutionCompletedEventAttributes {
    pub workflow_execution: Option<WorkflowExecution>,
    pub workflow_type: Option<WorkflowType>,
    pub result: Option<Vec<u8>>,
    pub initiated_event_id: i64,
    pub started_event_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChildWorkflowExecutionFailedEventAttributes {
    pub workflow_execution: Option<WorkflowExecution>,
    pub workflow_type: Option<WorkflowType>,
    pub reason: Option<String>,
    pub details: Option<Vec<u8>>,
    pub initiated_event_id: i64,
    pub started_event_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ContinueAsNewInitiator {
    Decider = 0,
    Retry = 1,
    CronSchedule = 2,
    Workflow = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum TimeoutType {
    StartToClose = 0,
    ScheduleToStart = 1,
    ScheduleToClose = 2,
    Heartbeat = 3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub initial_interval_in_seconds: i32,
    pub backoff_coefficient: f64,
    pub maximum_interval_in_seconds: i32,
    pub maximum_attempts: i32,
    pub non_retryable_error_types: Vec<String>,
    pub expiration_interval_in_seconds: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    pub fields: std::collections::HashMap<String, Vec<u8>>,
}

/// Workflow ID reuse policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum WorkflowIdReusePolicy {
    AllowDuplicateFailedOnly = 0,
    AllowDuplicate = 1,
    RejectDuplicate = 2,
    TerminateIfRunning = 3,
}

/// Parent close policy for child workflows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ParentClosePolicy {
    Terminate = 0,
    RequestCancel = 1,
    Abandon = 2,
}

/// Decision types for decision tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum DecisionType {
    ScheduleActivityTask = 0,
    RequestCancelActivityTask = 1,
    StartTimer = 2,
    CompleteWorkflowExecution = 3,
    FailWorkflowExecution = 4,
    CancelTimer = 5,
    CancelWorkflowExecution = 6,
    RequestCancelExternalWorkflowExecution = 7,
    RecordMarker = 8,
    ContinueAsNewWorkflowExecution = 9,
    StartChildWorkflowExecution = 10,
    SignalExternalWorkflowExecution = 11,
    UpsertWorkflowSearchAttributes = 12,
}

/// A decision to be made by the workflow
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    pub decision_type: DecisionType,
    pub attributes: Option<DecisionAttributes>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecisionAttributes {
    ScheduleActivityTaskDecisionAttributes(Box<ScheduleActivityTaskDecisionAttributes>),
    StartTimerDecisionAttributes(Box<StartTimerDecisionAttributes>),
    CompleteWorkflowExecutionDecisionAttributes(Box<CompleteWorkflowExecutionDecisionAttributes>),
    FailWorkflowExecutionDecisionAttributes(Box<FailWorkflowExecutionDecisionAttributes>),
    CancelTimerDecisionAttributes(Box<CancelTimerDecisionAttributes>),
    CancelWorkflowExecutionDecisionAttributes(Box<CancelWorkflowExecutionDecisionAttributes>),
    RequestCancelExternalWorkflowExecutionDecisionAttributes(
        Box<RequestCancelExternalWorkflowExecutionDecisionAttributes>,
    ),
    RecordMarkerDecisionAttributes(Box<RecordMarkerDecisionAttributes>),
    ContinueAsNewWorkflowExecutionDecisionAttributes(
        Box<ContinueAsNewWorkflowExecutionDecisionAttributes>,
    ),
    StartChildWorkflowExecutionDecisionAttributes(
        Box<StartChildWorkflowExecutionDecisionAttributes>,
    ),
    SignalExternalWorkflowExecutionDecisionAttributes(
        Box<SignalExternalWorkflowExecutionDecisionAttributes>,
    ),
    UpsertWorkflowSearchAttributesDecisionAttributes(
        Box<UpsertWorkflowSearchAttributesDecisionAttributes>,
    ),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleActivityTaskDecisionAttributes {
    pub activity_id: String,
    pub activity_type: Option<ActivityType>,
    pub task_list: Option<TaskList>,
    pub input: Option<Vec<u8>>,
    pub schedule_to_close_timeout_seconds: Option<i32>,
    pub schedule_to_start_timeout_seconds: Option<i32>,
    pub start_to_close_timeout_seconds: Option<i32>,
    pub heartbeat_timeout_seconds: Option<i32>,
    pub retry_policy: Option<RetryPolicy>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartTimerDecisionAttributes {
    pub timer_id: String,
    pub start_to_fire_timeout_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompleteWorkflowExecutionDecisionAttributes {
    pub result: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailWorkflowExecutionDecisionAttributes {
    pub reason: Option<String>,
    pub details: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelTimerDecisionAttributes {
    pub timer_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelWorkflowExecutionDecisionAttributes {
    pub details: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestCancelExternalWorkflowExecutionDecisionAttributes {
    pub domain: String,
    pub workflow_execution: Option<WorkflowExecution>,
    pub control: Option<String>,
    pub child_workflow_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordMarkerDecisionAttributes {
    pub marker_name: String,
    pub details: Option<Vec<u8>>,
    pub header: Option<Header>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinueAsNewWorkflowExecutionDecisionAttributes {
    pub workflow_type: Option<WorkflowType>,
    pub task_list: Option<TaskList>,
    pub input: Option<Vec<u8>>,
    pub execution_start_to_close_timeout_seconds: Option<i32>,
    pub task_start_to_close_timeout_seconds: Option<i32>,
    pub backoff_start_interval_in_seconds: Option<i32>,
    pub retry_policy: Option<RetryPolicy>,
    pub initiator: Option<ContinueAsNewInitiator>,
    pub failure_details: Option<Vec<u8>>,
    pub last_completion_result: Option<Vec<u8>>,
    pub cron_schedule: Option<String>,
    pub memo: Option<Memo>,
    pub search_attributes: Option<SearchAttributes>,
    pub header: Option<Header>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartChildWorkflowExecutionDecisionAttributes {
    pub domain: String,
    pub workflow_id: String,
    pub workflow_type: Option<WorkflowType>,
    pub task_list: Option<TaskList>,
    pub input: Option<Vec<u8>>,
    pub execution_start_to_close_timeout_seconds: Option<i32>,
    pub task_start_to_close_timeout_seconds: Option<i32>,
    pub parent_close_policy: Option<ParentClosePolicy>,
    pub control: Option<String>,
    pub decision_task_completed_event_id: i64,
    pub workflow_id_reuse_policy: Option<WorkflowIdReusePolicy>,
    pub retry_policy: Option<RetryPolicy>,
    pub cron_schedule: Option<String>,
    pub memo: Option<Memo>,
    pub search_attributes: Option<SearchAttributes>,
    pub header: Option<Header>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalExternalWorkflowExecutionDecisionAttributes {
    pub domain: String,
    pub workflow_execution: Option<WorkflowExecution>,
    pub signal_name: String,
    pub input: Option<Vec<u8>>,
    pub control: Option<String>,
    pub child_workflow_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpsertWorkflowSearchAttributesDecisionAttributes {
    pub search_attributes: Option<SearchAttributes>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Memo {
    pub fields: std::collections::HashMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchAttributes {
    pub indexed_fields: std::collections::HashMap<String, Vec<u8>>,
}

/// History of a workflow execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct History {
    pub events: Vec<HistoryEvent>,
}

/// History branch token
pub type HistoryBranch = Vec<u8>;

/// Event filter type for history queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum HistoryEventFilterType {
    AllEvent = 0,
    CloseEvent = 1,
}
