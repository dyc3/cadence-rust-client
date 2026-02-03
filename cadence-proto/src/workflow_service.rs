//! Workflow service client interface.
//!
//! This module defines the service interface for communicating with the
//! Cadence server. It includes request/response types for all operations.

use crate::shared::*;
use serde::{Deserialize, Serialize};

/// Start workflow execution request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartWorkflowExecutionRequest {
    pub domain: String,
    pub workflow_id: String,
    pub workflow_type: Option<WorkflowType>,
    pub task_list: Option<TaskList>,
    pub input: Option<Vec<u8>>,
    pub execution_start_to_close_timeout_seconds: Option<i32>,
    pub task_start_to_close_timeout_seconds: Option<i32>,
    pub identity: String,
    pub request_id: String,
    pub workflow_id_reuse_policy: Option<WorkflowIdReusePolicy>,
    pub retry_policy: Option<RetryPolicy>,
    pub cron_schedule: Option<String>,
    pub memo: Option<Memo>,
    pub search_attributes: Option<SearchAttributes>,
    pub header: Option<Header>,
    pub delay_start_seconds: Option<i32>,
    pub jitter_start_seconds: Option<i32>,
    pub first_execution_run_id: Option<String>,
    pub first_decision_task_backoff_seconds: Option<i32>,
    pub partition_config: Option<PartitionConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionConfig {
    pub properties: std::collections::HashMap<String, String>,
}

/// Start workflow execution response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartWorkflowExecutionResponse {
    pub run_id: String,
}

/// Signal workflow execution request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalWorkflowExecutionRequest {
    pub domain: String,
    pub workflow_execution: Option<WorkflowExecution>,
    pub signal_name: String,
    pub input: Option<Vec<u8>>,
    pub identity: String,
    pub request_id: String,
    pub control: Option<String>,
}

/// Signal workflow execution response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalWorkflowExecutionResponse {}

/// Signal with start workflow execution request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalWithStartWorkflowExecutionRequest {
    pub domain: String,
    pub workflow_id: String,
    pub workflow_type: Option<WorkflowType>,
    pub task_list: Option<TaskList>,
    pub input: Option<Vec<u8>>,
    pub execution_start_to_close_timeout_seconds: Option<i32>,
    pub task_start_to_close_timeout_seconds: Option<i32>,
    pub identity: String,
    pub request_id: String,
    pub workflow_id_reuse_policy: Option<WorkflowIdReusePolicy>,
    pub signal_name: String,
    pub signal_input: Option<Vec<u8>>,
    pub retry_policy: Option<RetryPolicy>,
    pub cron_schedule: Option<String>,
    pub memo: Option<Memo>,
    pub search_attributes: Option<SearchAttributes>,
    pub header: Option<Header>,
}

/// Cancel workflow execution request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestCancelWorkflowExecutionRequest {
    pub domain: String,
    pub workflow_execution: Option<WorkflowExecution>,
    pub identity: String,
    pub request_id: String,
    pub cause: Option<String>,
    pub first_execution_run_id: Option<String>,
}

/// Cancel workflow execution response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestCancelWorkflowExecutionResponse {}

/// Terminate workflow execution request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerminateWorkflowExecutionRequest {
    pub domain: String,
    pub workflow_execution: Option<WorkflowExecution>,
    pub reason: Option<String>,
    pub details: Option<Vec<u8>>,
    pub identity: String,
    pub first_execution_run_id: Option<String>,
}

/// Terminate workflow execution response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerminateWorkflowExecutionResponse {}

/// Query workflow request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryWorkflowRequest {
    pub domain: String,
    pub execution: Option<WorkflowExecution>,
    pub query: Option<WorkflowQuery>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowQuery {
    pub query_type: String,
    pub query_args: Option<Vec<u8>>,
}

/// Query workflow response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryWorkflowResponse {
    pub query_result: Option<Vec<u8>>,
    pub query_rejected: Option<QueryRejected>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRejected {
    pub close_status: Option<WorkflowExecutionCloseStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum WorkflowExecutionCloseStatus {
    Completed = 0,
    Failed = 1,
    Canceled = 2,
    Terminated = 3,
    ContinuedAsNew = 4,
    TimedOut = 5,
}

/// Poll for decision task request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PollForDecisionTaskRequest {
    pub domain: String,
    pub task_list: Option<TaskList>,
    pub identity: String,
    pub binary_checksum: String,
}

/// Poll for decision task response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PollForDecisionTaskResponse {
    pub task_token: Vec<u8>,
    pub workflow_execution: Option<WorkflowExecution>,
    pub workflow_type: Option<WorkflowType>,
    pub previous_started_event_id: i64,
    pub started_event_id: i64,
    pub attempt: i32,
    pub backlog_count_hint: i64,
    pub history: Option<History>,
    pub next_page_token: Option<Vec<u8>>,
    pub query: Option<WorkflowQuery>,
    pub workflow_execution_task_list: Option<TaskList>,
    pub scheduled_timestamp: Option<i64>,
    pub started_timestamp: Option<i64>,
    pub queries: Option<std::collections::HashMap<String, WorkflowQuery>>,
}

/// Respond decision task completed request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RespondDecisionTaskCompletedRequest {
    pub task_token: Vec<u8>,
    pub decisions: Vec<Decision>,
    pub execution_context: Option<Vec<u8>>,
    pub identity: String,
    pub sticky_attributes: Option<StickyExecutionAttributes>,
    pub return_new_decision_task: bool,
    pub force_create_new_decision_task: bool,
    pub binary_checksum: String,
    pub query_results: Option<std::collections::HashMap<String, WorkflowQueryResult>>,
    pub complete_execution_signal_decision_task: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StickyExecutionAttributes {
    pub worker_task_list: Option<TaskList>,
    pub schedule_to_start_timeout_seconds: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowQueryResult {
    pub query_type: Option<String>,
    pub answer: Option<Vec<u8>>,
    pub error_message: Option<String>,
    pub query_result_type: QueryResultType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum QueryResultType {
    Answered = 0,
    Failed = 1,
}

/// Respond decision task completed response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RespondDecisionTaskCompletedResponse {
    pub decision_task: Option<PollForDecisionTaskResponse>,
}

/// Respond decision task failed request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RespondDecisionTaskFailedRequest {
    pub task_token: Vec<u8>,
    pub cause: DecisionTaskFailedCause,
    pub details: Option<Vec<u8>>,
    pub identity: String,
    pub binary_checksum: String,
}

/// Respond decision task failed response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RespondDecisionTaskFailedResponse {}

/// Poll for activity task request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PollForActivityTaskRequest {
    pub domain: String,
    pub task_list: Option<TaskList>,
    pub identity: String,
    pub task_list_metadata: Option<TaskListMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskListMetadata {
    pub max_tasks_per_second: Option<f64>,
}

/// Poll for activity task response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PollForActivityTaskResponse {
    pub task_token: Vec<u8>,
    pub workflow_execution: Option<WorkflowExecution>,
    pub activity_id: String,
    pub activity_type: Option<ActivityType>,
    pub input: Option<Vec<u8>>,
    pub scheduled_timestamp: Option<i64>,
    pub started_timestamp: Option<i64>,
    pub schedule_to_close_timeout_seconds: Option<i32>,
    pub start_to_close_timeout_seconds: Option<i32>,
    pub heartbeat_timeout_seconds: Option<i32>,
    pub attempt: i32,
    pub scheduled_timestamp_of_this_attempt: Option<i64>,
    pub heartbeat_details: Option<Vec<u8>>,
    pub workflow_type: Option<WorkflowType>,
    pub workflow_domain: Option<String>,
    pub header: Option<Header>,
}

/// Record activity task heartbeat request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordActivityTaskHeartbeatRequest {
    pub task_token: Vec<u8>,
    pub details: Option<Vec<u8>>,
    pub identity: String,
}

/// Record activity task heartbeat response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordActivityTaskHeartbeatResponse {
    pub cancel_requested: bool,
}

/// Respond activity task completed request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RespondActivityTaskCompletedRequest {
    pub task_token: Vec<u8>,
    pub result: Option<Vec<u8>>,
    pub identity: String,
}

/// Respond activity task completed response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RespondActivityTaskCompletedResponse {}

/// Respond activity task failed request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RespondActivityTaskFailedRequest {
    pub task_token: Vec<u8>,
    pub reason: Option<String>,
    pub details: Option<Vec<u8>>,
    pub identity: String,
}

/// Respond activity task failed response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RespondActivityTaskFailedResponse {}

/// Get workflow execution history request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetWorkflowExecutionHistoryRequest {
    pub domain: String,
    pub execution: Option<WorkflowExecution>,
    pub page_size: i32,
    pub next_page_token: Option<Vec<u8>>,
    pub wait_for_new_event: bool,
    pub history_event_filter_type: Option<HistoryEventFilterType>,
    pub skip_archival: bool,
}

/// Get workflow execution history response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetWorkflowExecutionHistoryResponse {
    pub history: Option<History>,
    pub next_page_token: Option<Vec<u8>>,
    pub archived: bool,
}

/// Describe workflow execution request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescribeWorkflowExecutionRequest {
    pub domain: String,
    pub execution: Option<WorkflowExecution>,
}

/// Describe workflow execution response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescribeWorkflowExecutionResponse {
    pub execution_configuration: Option<WorkflowExecutionConfiguration>,
    pub workflow_execution_info: Option<WorkflowExecutionInfo>,
    pub pending_children: Vec<PendingChildExecutionInfo>,
    pub pending_decision: Option<PendingDecisionInfo>,
    pub pending_activities: Vec<PendingActivityInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExecutionConfiguration {
    pub task_list: Option<TaskList>,
    pub execution_start_to_close_timeout_seconds: i32,
    pub task_start_to_close_timeout_seconds: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExecutionInfo {
    pub execution: Option<WorkflowExecution>,
    pub workflow_type: Option<WorkflowType>,
    pub start_time: Option<i64>,
    pub close_time: Option<i64>,
    pub close_status: Option<WorkflowExecutionCloseStatus>,
    pub history_length: i64,
    pub parent_domain_id: Option<String>,
    pub parent_execution: Option<WorkflowExecution>,
    pub execution_time: Option<i64>,
    pub memo: Option<Memo>,
    pub search_attributes: Option<SearchAttributes>,
    pub auto_reset_points: Option<ResetPoints>,
    pub task_list: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResetPoints {
    pub points: Vec<ResetPointInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResetPointInfo {
    pub binary_checksum: String,
    pub run_id: String,
    pub first_decision_completed_id: i64,
    pub created_time_nano: i64,
    pub expiring_time_nano: i64,
    pub resettable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingChildExecutionInfo {
    pub workflow_id: String,
    pub run_id: String,
    pub workflow_type_name: String,
    pub initiated_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingDecisionInfo {
    pub state: Option<PendingDecisionState>,
    pub scheduled_timestamp: Option<i64>,
    pub started_timestamp: Option<i64>,
    pub attempt: i32,
    pub original_scheduled_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum PendingDecisionState {
    Scheduled = 0,
    Started = 1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingActivityInfo {
    pub activity_id: String,
    pub activity_type: Option<ActivityType>,
    pub state: Option<PendingActivityState>,
    pub scheduled_timestamp: Option<i64>,
    pub last_started_timestamp: Option<i64>,
    pub last_heartbeat_timestamp: Option<i64>,
    pub attempt: i32,
    pub maximum_attempts: i32,
    pub scheduled_timestamp_of_this_attempt: Option<i64>,
    pub heartbeat_details: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum PendingActivityState {
    Scheduled = 0,
    Started = 1,
    CancelRequested = 2,
}

/// List open workflow executions request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListOpenWorkflowExecutionsRequest {
    pub domain: String,
    pub maximum_page_size: i32,
    pub next_page_token: Option<Vec<u8>>,
    pub start_time_filter: Option<StartTimeFilter>,
    pub execution_filter: Option<WorkflowExecutionFilter>,
    pub type_filter: Option<WorkflowTypeFilter>,
    pub status_filter: Option<WorkflowExecutionCloseStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartTimeFilter {
    pub earliest_time: Option<i64>,
    pub latest_time: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowExecutionFilter {
    pub workflow_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowTypeFilter {
    pub name: String,
}

/// List open workflow executions response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListOpenWorkflowExecutionsResponse {
    pub executions: Vec<WorkflowExecutionInfo>,
    pub next_page_token: Option<Vec<u8>>,
}

/// List closed workflow executions request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListClosedWorkflowExecutionsRequest {
    pub domain: String,
    pub maximum_page_size: i32,
    pub next_page_token: Option<Vec<u8>>,
    pub start_time_filter: Option<StartTimeFilter>,
    pub execution_filter: Option<WorkflowExecutionFilter>,
    pub type_filter: Option<WorkflowTypeFilter>,
    pub status_filter: Option<WorkflowExecutionCloseStatus>,
}

/// List closed workflow executions response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListClosedWorkflowExecutionsResponse {
    pub executions: Vec<WorkflowExecutionInfo>,
    pub next_page_token: Option<Vec<u8>>,
}

/// Domain management types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegisterDomainRequest {
    pub name: String,
    pub description: Option<String>,
    pub owner_email: String,
    pub workflow_execution_retention_period_in_days: i32,
    pub emit_metric: Option<bool>,
    pub clusters: Vec<ClusterReplicationConfiguration>,
    pub active_cluster_name: String,
    pub data: std::collections::HashMap<String, String>,
    pub security_token: Option<String>,
    pub is_global_domain: Option<bool>,
    pub history_archival_status: Option<ArchivalStatus>,
    pub history_archival_uri: Option<String>,
    pub visibility_archival_status: Option<ArchivalStatus>,
    pub visibility_archival_uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterReplicationConfiguration {
    pub cluster_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ArchivalStatus {
    Disabled = 0,
    Enabled = 1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescribeDomainRequest {
    pub name: Option<String>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescribeDomainResponse {
    pub domain_info: Option<DomainInfo>,
    pub configuration: Option<DomainConfiguration>,
    pub replication_configuration: Option<DomainReplicationConfiguration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainInfo {
    pub name: String,
    pub status: Option<DomainStatus>,
    pub description: String,
    pub owner_email: String,
    pub data: std::collections::HashMap<String, String>,
    pub uuid: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum DomainStatus {
    Registered = 0,
    Deprecated = 1,
    Deleted = 2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainConfiguration {
    pub workflow_execution_retention_period_in_days: i32,
    pub emit_metric: bool,
    pub history_archival_status: Option<ArchivalStatus>,
    pub history_archival_uri: String,
    pub visibility_archival_status: Option<ArchivalStatus>,
    pub visibility_archival_uri: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainReplicationConfiguration {
    pub active_cluster_name: String,
    pub clusters: Vec<ClusterReplicationConfiguration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateDomainRequest {
    pub name: Option<String>,
    pub uuid: Option<String>,
    pub updated_info: Option<UpdateDomainInfo>,
    pub configuration: Option<DomainConfiguration>,
    pub replication_configuration: Option<DomainReplicationConfiguration>,
    pub security_token: Option<String>,
    pub delete_bad_binary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateDomainInfo {
    pub description: String,
    pub owner_email: String,
    pub data: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateDomainResponse {
    pub domain_info: Option<DomainInfo>,
    pub configuration: Option<DomainConfiguration>,
    pub replication_configuration: Option<DomainReplicationConfiguration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailoverDomainRequest {
    pub name: String,
    pub clusters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailoverDomainResponse {}

/// Workflow service trait - defines all service methods
#[async_trait::async_trait]
pub trait WorkflowService: Send + Sync {
    type Error: std::error::Error;

    async fn start_workflow_execution(
        &self,
        request: StartWorkflowExecutionRequest,
    ) -> Result<StartWorkflowExecutionResponse, Self::Error>;

    async fn signal_workflow_execution(
        &self,
        request: SignalWorkflowExecutionRequest,
    ) -> Result<SignalWorkflowExecutionResponse, Self::Error>;

    async fn signal_with_start_workflow_execution(
        &self,
        request: SignalWithStartWorkflowExecutionRequest,
    ) -> Result<StartWorkflowExecutionResponse, Self::Error>;

    async fn request_cancel_workflow_execution(
        &self,
        request: RequestCancelWorkflowExecutionRequest,
    ) -> Result<RequestCancelWorkflowExecutionResponse, Self::Error>;

    async fn terminate_workflow_execution(
        &self,
        request: TerminateWorkflowExecutionRequest,
    ) -> Result<TerminateWorkflowExecutionResponse, Self::Error>;

    async fn query_workflow(
        &self,
        request: QueryWorkflowRequest,
    ) -> Result<QueryWorkflowResponse, Self::Error>;

    async fn poll_for_decision_task(
        &self,
        request: PollForDecisionTaskRequest,
    ) -> Result<PollForDecisionTaskResponse, Self::Error>;

    async fn respond_decision_task_completed(
        &self,
        request: RespondDecisionTaskCompletedRequest,
    ) -> Result<RespondDecisionTaskCompletedResponse, Self::Error>;

    async fn respond_decision_task_failed(
        &self,
        request: RespondDecisionTaskFailedRequest,
    ) -> Result<RespondDecisionTaskFailedResponse, Self::Error>;

    async fn poll_for_activity_task(
        &self,
        request: PollForActivityTaskRequest,
    ) -> Result<PollForActivityTaskResponse, Self::Error>;

    async fn record_activity_task_heartbeat(
        &self,
        request: RecordActivityTaskHeartbeatRequest,
    ) -> Result<RecordActivityTaskHeartbeatResponse, Self::Error>;

    async fn respond_activity_task_completed(
        &self,
        request: RespondActivityTaskCompletedRequest,
    ) -> Result<RespondActivityTaskCompletedResponse, Self::Error>;

    async fn respond_activity_task_failed(
        &self,
        request: RespondActivityTaskFailedRequest,
    ) -> Result<RespondActivityTaskFailedResponse, Self::Error>;

    async fn get_workflow_execution_history(
        &self,
        request: GetWorkflowExecutionHistoryRequest,
    ) -> Result<GetWorkflowExecutionHistoryResponse, Self::Error>;

    async fn describe_workflow_execution(
        &self,
        request: DescribeWorkflowExecutionRequest,
    ) -> Result<DescribeWorkflowExecutionResponse, Self::Error>;

    async fn list_open_workflow_executions(
        &self,
        request: ListOpenWorkflowExecutionsRequest,
    ) -> Result<ListOpenWorkflowExecutionsResponse, Self::Error>;

    async fn list_closed_workflow_executions(
        &self,
        request: ListClosedWorkflowExecutionsRequest,
    ) -> Result<ListClosedWorkflowExecutionsResponse, Self::Error>;

    // Domain management
    async fn register_domain(&self, request: RegisterDomainRequest) -> Result<(), Self::Error>;

    async fn describe_domain(
        &self,
        request: DescribeDomainRequest,
    ) -> Result<DescribeDomainResponse, Self::Error>;

    async fn update_domain(
        &self,
        request: UpdateDomainRequest,
    ) -> Result<UpdateDomainResponse, Self::Error>;

    async fn failover_domain(
        &self,
        request: FailoverDomainRequest,
    ) -> Result<FailoverDomainResponse, Self::Error>;
}
