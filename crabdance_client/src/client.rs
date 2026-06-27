//! Client implementation for Cadence workflow orchestration service.
//!
//! This module provides the main client interface for starting workflows,
//! querying workflow state, sending signals, and managing workflow executions.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use crabdance_core::{
    CadenceError, CadenceResult, EncodedValue, QueryConsistencyLevel, RetryPolicy,
    WorkflowExecution, WorkflowIdReusePolicy,
};
use crabdance_proto::shared::*;
use crabdance_proto::workflow_service::*;
use crabdance_proto::{
    PendingActivityInfo as ProtoPendingActivityInfo,
    PendingActivityState as ProtoPendingActivityState,
    PendingChildExecutionInfo as ProtoPendingChildExecutionInfo,
    WorkflowExecutionConfiguration as ProtoWorkflowExecutionConfiguration,
    WorkflowExecutionInfo as ProtoWorkflowExecutionInfo,
};
use uuid::Uuid;

use crate::auth::BoxedAuthProvider;
use crate::error::ClientError;

/// Query type constants
pub const QUERY_TYPE_STACK_TRACE: &str = "__stack_trace";
pub const QUERY_TYPE_OPEN_SESSIONS: &str = "__open_sessions";
pub const QUERY_TYPE_QUERY_TYPES: &str = "__query_types";

/// Client trait for workflow operations
#[async_trait]
pub trait Client: Send + Sync {
    /// Start a workflow execution
    async fn start_workflow(
        &self,
        options: StartWorkflowOptions,
        workflow_type: &str,
        args: Option<&[u8]>,
    ) -> CadenceResult<WorkflowExecution>;

    /// Start a workflow execution asynchronously (queued)
    async fn start_workflow_async(
        &self,
        options: StartWorkflowOptions,
        workflow_type: &str,
        args: Option<&[u8]>,
    ) -> CadenceResult<WorkflowExecutionAsync>;

    /// Execute a workflow and wait for completion
    async fn execute_workflow(
        &self,
        options: StartWorkflowOptions,
        workflow_type: &str,
        args: Option<&[u8]>,
    ) -> CadenceResult<Box<dyn WorkflowRun>>;

    /// Get an existing workflow execution handle
    fn get_workflow(&self, workflow_id: &str, run_id: Option<&str>) -> Box<dyn WorkflowRun>;

    /// Signal a running workflow
    async fn signal_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        signal_name: &str,
        arg: Option<&[u8]>,
    ) -> CadenceResult<()>;

    /// Signal with start - signal a running workflow or start it if not running
    async fn signal_with_start_workflow(
        &self,
        workflow_id: &str,
        signal_name: &str,
        signal_arg: Option<&[u8]>,
        options: StartWorkflowOptions,
        workflow_type: &str,
        workflow_args: Option<&[u8]>,
    ) -> CadenceResult<WorkflowExecution>;

    /// Signal with start asynchronously
    async fn signal_with_start_workflow_async(
        &self,
        workflow_id: &str,
        signal_name: &str,
        signal_arg: Option<&[u8]>,
        options: StartWorkflowOptions,
        workflow_type: &str,
        workflow_args: Option<&[u8]>,
    ) -> CadenceResult<WorkflowExecutionAsync>;

    /// Cancel a workflow execution
    async fn cancel_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        reason: Option<&str>,
    ) -> CadenceResult<()>;

    /// Cancel a workflow execution with options (e.g. an explicit cancel reason).
    ///
    /// Mirrors Go's `Client.CancelWorkflow(ctx, id, runID, opts...)` with
    /// `WithCancelReason`.
    async fn cancel_workflow_with_options(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        options: CancelWorkflowOptions,
    ) -> CadenceResult<()>;

    /// Terminate a workflow execution
    async fn terminate_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        reason: Option<&str>,
        details: Option<&[u8]>,
    ) -> CadenceResult<()>;

    /// Get workflow execution history
    async fn get_workflow_history(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        is_long_poll: bool,
        filter_type: HistoryEventFilterType,
    ) -> CadenceResult<HistoryEventIterator>;

    /// Get workflow history with options
    async fn get_workflow_history_with_options(
        &self,
        request: GetWorkflowHistoryWithOptionsRequest,
    ) -> CadenceResult<HistoryEventIterator>;

    /// Complete an activity (for async completion)
    async fn complete_activity(
        &self,
        task_token: &[u8],
        result: Option<&[u8]>,
        error: Option<CadenceError>,
    ) -> CadenceResult<()>;

    /// Complete an activity by ID
    async fn complete_activity_by_id(
        &self,
        domain: &str,
        workflow_id: &str,
        run_id: Option<&str>,
        activity_id: &str,
        result: Option<&[u8]>,
        error: Option<CadenceError>,
    ) -> CadenceResult<()>;

    /// Record activity heartbeat
    async fn record_activity_heartbeat(
        &self,
        task_token: &[u8],
        details: Option<&[u8]>,
    ) -> CadenceResult<bool>;

    /// Record activity heartbeat by ID
    async fn record_activity_heartbeat_by_id(
        &self,
        domain: &str,
        workflow_id: &str,
        run_id: Option<&str>,
        activity_id: &str,
        details: Option<&[u8]>,
    ) -> CadenceResult<bool>;

    /// List closed workflow executions
    async fn list_closed_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse>;

    /// List open workflow executions
    async fn list_open_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse>;

    /// List workflow executions with query
    async fn list_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse>;

    /// List archived workflow executions with query.
    ///
    /// Mirrors Go's `Client.ListArchivedWorkflow`.
    async fn list_archived_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse>;

    /// Scan workflow executions
    async fn scan_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse>;

    /// Count workflow executions
    async fn count_workflows(
        &self,
        request: CountWorkflowExecutionsRequest,
    ) -> CadenceResult<CountWorkflowExecutionsResponse>;

    /// Get search attributes
    async fn get_search_attributes(&self) -> CadenceResult<SearchAttributesResponse>;

    /// Query a workflow
    async fn query_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        query_type: &str,
        args: Option<&[u8]>,
    ) -> CadenceResult<EncodedValue>;

    /// Query workflow with options
    async fn query_workflow_with_options(
        &self,
        request: QueryWorkflowWithOptionsRequest,
    ) -> CadenceResult<QueryWorkflowWithOptionsResponse>;

    /// Reset a workflow execution
    async fn reset_workflow(
        &self,
        request: ResetWorkflowExecutionRequest,
    ) -> CadenceResult<ResetWorkflowExecutionResponse>;

    /// Describe workflow execution
    async fn describe_workflow_execution(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
    ) -> CadenceResult<DescribeWorkflowExecutionResponse>;

    /// Describe workflow execution with options
    async fn describe_workflow_execution_with_options(
        &self,
        request: DescribeWorkflowExecutionWithOptionsRequest,
    ) -> CadenceResult<DescribeWorkflowExecutionResponse>;

    /// Describe task list
    async fn describe_task_list(
        &self,
        task_list: &str,
        task_list_type: TaskListType,
    ) -> CadenceResult<DescribeTaskListResponse>;

    /// Refresh workflow tasks
    async fn refresh_workflow_tasks(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
    ) -> CadenceResult<()>;
}

/// Options for starting a workflow
#[derive(Debug, Clone)]
pub struct StartWorkflowOptions {
    pub id: String,
    pub task_list: String,
    pub execution_start_to_close_timeout: Option<Duration>,
    pub task_start_to_close_timeout: Option<Duration>,
    pub identity: Option<String>,
    pub workflow_id_reuse_policy: WorkflowIdReusePolicy,
    pub retry_policy: Option<RetryPolicy>,
    pub cron_schedule: Option<String>,
    pub memo: Option<HashMap<String, Vec<u8>>>,
    pub search_attributes: Option<HashMap<String, Vec<u8>>>,
    pub header: Option<HashMap<String, Vec<u8>>>,
    pub delay_start: Option<Duration>,
    /// Amount to jitter the workflow start by. For example, with `10s` the
    /// workflow starts at a random point in the 0-10s window. Mirrors Go's
    /// `StartWorkflowOptions.JitterStart`.
    pub jitter_start: Option<Duration>,
    /// Specific wall-clock time for the first run to start at. When set it
    /// overrides `delay_start`/`jitter_start` for the first run. Mirrors Go's
    /// `StartWorkflowOptions.FirstRunAt`.
    pub first_run_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Policy for handling cron workflow overlaps when a previous run is still
    /// running. Mirrors Go's `StartWorkflowOptions.CronOverlapPolicy`.
    pub cron_overlap_policy: CronOverlapPolicy,
    /// Policy for selecting the active cluster to start the execution on for
    /// active-active domains. Mirrors Go's
    /// `StartWorkflowOptions.ActiveClusterSelectionPolicy`.
    pub active_cluster_selection_policy: Option<ActiveClusterSelectionPolicy>,
}

impl Default for StartWorkflowOptions {
    fn default() -> Self {
        Self {
            id: String::new(),
            task_list: String::new(),
            execution_start_to_close_timeout: None,
            task_start_to_close_timeout: None,
            identity: None,
            workflow_id_reuse_policy: WorkflowIdReusePolicy::AllowDuplicateFailedOnly,
            retry_policy: None,
            cron_schedule: None,
            memo: None,
            search_attributes: None,
            header: None,
            delay_start: None,
            jitter_start: None,
            first_run_at: None,
            cron_overlap_policy: CronOverlapPolicy::default(),
            active_cluster_selection_policy: None,
        }
    }
}

/// Policy for handling cron workflow overlaps when the previous scheduled run is
/// still running by the time the next run is scheduled.
///
/// Mirrors the Go/Thrift `CronOverlapPolicy`. The default is [`Skipped`], which
/// matches Go's zero value.
///
/// [`Skipped`]: CronOverlapPolicy::Skipped
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CronOverlapPolicy {
    /// Skip the next scheduled run if the previous one is still running.
    #[default]
    Skipped,
    /// Buffer (at most) one run and start it immediately after the previous,
    /// overrunning run completes.
    BufferOne,
}

impl CronOverlapPolicy {
    /// Map to the protobuf `CronOverlapPolicy` discriminant
    /// (`1`=Skipped, `2`=BufferOne; `0`=Invalid is never emitted by the client).
    fn to_proto(self) -> i32 {
        match self {
            CronOverlapPolicy::Skipped => 1,
            CronOverlapPolicy::BufferOne => 2,
        }
    }
}

/// Policy for selecting the active cluster to start a workflow execution on for
/// active-active domains. Mirrors Go's `ActiveClusterSelectionPolicy`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveClusterSelectionPolicy {
    /// Cluster attribute identifying the sub-group whose active cluster should
    /// host the workflow. When absent the domain's active cluster is used.
    pub cluster_attribute: Option<ClusterAttribute>,
}

/// A cluster attribute (scope + name) used by [`ActiveClusterSelectionPolicy`].
/// Mirrors Go's `ClusterAttribute`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClusterAttribute {
    pub scope: String,
    pub name: String,
}

/// Options for cancelling a workflow execution. Mirrors the variadic options of
/// Go's `Client.CancelWorkflow` (e.g. `WithCancelReason`).
#[derive(Debug, Clone, Default)]
pub struct CancelWorkflowOptions {
    /// Explicit, human-readable cancellation reason recorded as the request
    /// `cause`. Mirrors Go's `WithCancelReason`.
    pub cause: Option<String>,
}

/// Async workflow execution (returned by async start)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowExecutionAsync {
    pub workflow_id: String,
}

/// Workflow run handle for retrieving results
#[async_trait]
pub trait WorkflowRun: Send + Sync {
    /// Get the workflow ID. Mirrors Go's `WorkflowRun.GetID`.
    fn get_id(&self) -> &str;
    /// Get the run ID. Mirrors Go's `WorkflowRun.GetRunID`.
    fn run_id(&self) -> &str;
    /// Get the workflow execution result, blocking until the workflow closes.
    /// Mirrors Go's `WorkflowRun.Get`.
    async fn get(&self) -> CadenceResult<Option<Vec<u8>>>;
    /// Get the workflow execution result, returning a timeout error if the
    /// workflow has not closed within `timeout`.
    async fn get_with_timeout(&self, timeout: Duration) -> CadenceResult<Option<Vec<u8>>>;
}

/// History event iterator
pub struct HistoryEventIterator {
    events: Vec<HistoryEvent>,
    position: usize,
    next_page_token: Option<Vec<u8>>,
    // Reference to client to fetch more pages
    client: Arc<WorkflowClient>,
    workflow_id: String,
    run_id: Option<String>,
    filter_type: HistoryEventFilterType,
}

impl HistoryEventIterator {
    pub fn new(
        events: Vec<HistoryEvent>,
        next_page_token: Option<Vec<u8>>,
        client: Arc<WorkflowClient>,
        workflow_id: String,
        run_id: Option<String>,
        filter_type: HistoryEventFilterType,
    ) -> Self {
        Self {
            events,
            position: 0,
            next_page_token,
            client,
            workflow_id,
            run_id,
            filter_type,
        }
    }

    pub fn has_next(&self) -> bool {
        self.position < self.events.len() || self.next_page_token.is_some()
    }

    pub async fn next(&mut self) -> CadenceResult<Option<HistoryEvent>> {
        if self.position < self.events.len() {
            let event = self.events[self.position].clone();
            self.position += 1;
            return Ok(Some(event));
        }

        if let Some(token) = &self.next_page_token {
            // Fetch next page
            let request = GetWorkflowExecutionHistoryRequest {
                domain: self.client.domain.clone(),
                execution: Some(make_proto_execution(
                    self.workflow_id.clone(),
                    self.run_id.clone().unwrap_or_default(),
                )),
                page_size: 1000,
                next_page_token: Some(token.clone()),
                wait_for_new_event: false,
                history_event_filter_type: Some(self.filter_type),
                skip_archival: true,
                query_consistency_level: None,
            };

            let response = self
                .client
                .service
                .get_workflow_execution_history(request)
                .await
                .map_err(|e| CadenceError::Other(e.to_string()))?;

            if let Some(history) = response.history {
                self.events = history.events;
                self.position = 0;
                self.next_page_token = response.next_page_token;

                if !self.events.is_empty() {
                    let event = self.events[self.position].clone();
                    self.position += 1;
                    return Ok(Some(event));
                }
            } else {
                self.next_page_token = None;
            }
        }

        Ok(None)
    }
}

/// Request for listing workflow executions
#[derive(Debug, Clone)]
pub struct ListWorkflowExecutionsRequest {
    pub maximum_page_size: i32,
    pub next_page_token: Option<Vec<u8>>,
    pub start_time_filter: Option<StartTimeFilter>,
    pub execution_filter: Option<WorkflowExecutionFilter>,
    pub type_filter: Option<WorkflowTypeFilter>,
    pub status_filter: Option<WorkflowExecutionCloseStatus>,
    /// Visibility query string used by the query-based APIs (`list_workflows`,
    /// `scan_workflows`, `list_archived_workflows`). Ignored by the filter-based
    /// `list_open_workflows`/`list_closed_workflows`. Mirrors the `Query` field
    /// of Go's `ListWorkflowExecutionsRequest`.
    pub query: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StartTimeFilter {
    pub earliest_time: chrono::DateTime<chrono::Utc>,
    pub latest_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionFilter {
    pub workflow_id: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowTypeFilter {
    pub name: String,
}

/// Response for listing workflow executions
#[derive(Debug, Clone)]
pub struct ListWorkflowExecutionsResponse {
    pub executions: Vec<WorkflowExecutionInfo>,
    pub next_page_token: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionInfo {
    pub execution: WorkflowExecution,
    pub workflow_type: String,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub close_time: Option<chrono::DateTime<chrono::Utc>>,
    pub close_status: Option<WorkflowExecutionCloseStatus>,
    pub history_length: i64,
}

/// Request for counting workflow executions
#[derive(Debug, Clone)]
pub struct CountWorkflowExecutionsRequest {
    pub query: String,
}

/// Response for counting workflow executions
#[derive(Debug, Clone)]
pub struct CountWorkflowExecutionsResponse {
    pub count: i64,
}

/// Response for getting search attributes
#[derive(Debug, Clone)]
pub struct SearchAttributesResponse {
    pub keys: HashMap<String, String>,
}

/// Request for querying workflow with options
#[derive(Debug, Clone)]
pub struct QueryWorkflowWithOptionsRequest {
    pub workflow_id: String,
    pub run_id: Option<String>,
    pub query_type: String,
    pub query_args: Option<Vec<u8>>,
    pub query_consistency_level: QueryConsistencyLevel,
    pub query_reject_condition: Option<QueryRejectCondition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryRejectCondition {
    NotOpen,
    NotCompletedCleanly,
}

/// Response for querying workflow with options
#[derive(Debug, Clone)]
pub struct QueryWorkflowWithOptionsResponse {
    pub query_result: Option<Vec<u8>>,
    pub query_rejected: Option<QueryRejected>,
}

#[derive(Debug, Clone)]
pub struct QueryRejected {
    pub close_status: WorkflowExecutionCloseStatus,
}

/// Request for getting workflow history with options
#[derive(Debug, Clone)]
pub struct GetWorkflowHistoryWithOptionsRequest {
    pub workflow_id: String,
    pub run_id: Option<String>,
    pub page_size: i32,
    pub wait_for_new_event: bool,
    pub history_event_filter_type: HistoryEventFilterType,
    pub skip_archival: bool,
}

/// Request for describing workflow execution with options
#[derive(Debug, Clone)]
pub struct DescribeWorkflowExecutionWithOptionsRequest {
    pub workflow_id: String,
    pub run_id: Option<String>,
}

/// Response for describing workflow execution
#[derive(Debug, Clone)]
pub struct DescribeWorkflowExecutionResponse {
    pub execution_configuration: WorkflowExecutionConfiguration,
    pub workflow_execution_info: WorkflowExecutionInfo,
    pub pending_activities: Vec<PendingActivityInfo>,
    pub pending_children: Vec<PendingChildExecutionInfo>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionConfiguration {
    pub task_list: String,
    pub execution_start_to_close_timeout: Duration,
    pub task_start_to_close_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct PendingActivityInfo {
    pub activity_id: String,
    pub activity_type: String,
    pub state: PendingActivityState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingActivityState {
    Scheduled,
    Started,
    CancelRequested,
}

#[derive(Debug, Clone)]
pub struct PendingChildExecutionInfo {
    pub workflow_id: String,
    pub run_id: String,
    pub workflow_type: String,
}

/// Request for resetting workflow execution
#[derive(Debug, Clone)]
pub struct ResetWorkflowExecutionRequest {
    pub domain: String,
    pub workflow_execution: WorkflowExecution,
    pub decision_finish_event_id: i64,
    pub request_id: String,
    pub reason: Option<String>,
}

/// Response for resetting workflow execution
#[derive(Debug, Clone)]
pub struct ResetWorkflowExecutionResponse {
    pub run_id: String,
}

/// Response for describing task list
#[derive(Debug, Clone)]
pub struct DescribeTaskListResponse {
    pub pollers: Vec<PollersInfo>,
}

#[derive(Debug, Clone)]
pub struct PollersInfo {
    pub identity: String,
    pub last_access_time: chrono::DateTime<chrono::Utc>,
}

/// Client implementation
pub struct WorkflowClient {
    pub(crate) service: Arc<crate::grpc::GrpcWorkflowServiceClient>,
    pub(crate) domain: String,
    pub(crate) options: ClientOptions,
}

impl WorkflowClient {
    /// Create a new WorkflowClient from an existing gRPC service
    pub fn new(
        service: Arc<crate::grpc::GrpcWorkflowServiceClient>,
        domain: String,
        options: ClientOptions,
    ) -> Self {
        Self {
            service,
            domain,
            options,
        }
    }

    /// Connect to a Cadence server and create a new WorkflowClient
    ///
    /// # Arguments
    /// * `endpoint` - The gRPC endpoint URL (e.g., "http://localhost:7833")
    /// * `domain` - The Cadence domain to use
    /// * `options` - Client configuration options including optional auth provider
    pub async fn connect(
        endpoint: impl Into<String>,
        domain: impl Into<String>,
        options: ClientOptions,
    ) -> Result<Self, ClientError> {
        let domain_str = domain.into();
        let service = Arc::new(
            crate::grpc::GrpcWorkflowServiceClient::connect(
                endpoint,
                domain_str.clone(),
                options.auth_provider.clone(),
            )
            .await?,
        );

        Ok(Self {
            service,
            domain: domain_str,
            options,
        })
    }
}

/// Client configuration options
#[derive(Clone)]
pub struct ClientOptions {
    /// Client identity string
    pub identity: String,
    /// Optional authentication provider for JWT/OAuth
    pub auth_provider: Option<BoxedAuthProvider>,
    /// Metrics scope for telemetry
    pub metrics_scope: Option<Arc<dyn MetricsScope>>,
    /// Logger for client operations
    pub logger: Option<Arc<dyn Logger>>,
    /// Feature flags for experimental features
    pub feature_flags: FeatureFlags,
}

impl std::fmt::Debug for ClientOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientOptions")
            .field("identity", &self.identity)
            .field("metrics_scope", &self.metrics_scope.is_some())
            .field("logger", &self.logger.is_some())
            .field("feature_flags", &self.feature_flags)
            .finish()
    }
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            identity: format!(
                "cadence-rust-client@{}-pid-{}",
                std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()),
                std::process::id()
            ),
            auth_provider: None,
            metrics_scope: None,
            logger: None,
            feature_flags: FeatureFlags::default(),
        }
    }
}

#[async_trait]
impl Client for WorkflowClient {
    async fn start_workflow(
        &self,
        options: StartWorkflowOptions,
        workflow_type: &str,
        args: Option<&[u8]>,
    ) -> CadenceResult<WorkflowExecution> {
        let run = self
            .execute_workflow(options.clone(), workflow_type, args)
            .await?;
        Ok(WorkflowExecution::new(options.id, run.run_id()))
    }

    async fn start_workflow_async(
        &self,
        options: StartWorkflowOptions,
        workflow_type: &str,
        args: Option<&[u8]>,
    ) -> CadenceResult<WorkflowExecutionAsync> {
        // Same as start_workflow but we return WorkflowExecutionAsync
        // Actually execute_workflow calls start_workflow_execution internally
        // So I should implement the shared logic in a helper or directly here.

        let request_id = Uuid::new_v4().to_string();
        let workflow_id = options.id.clone();
        let extensions = build_start_extensions(&options);
        let request = StartWorkflowExecutionRequest {
            domain: self.domain.clone(),
            workflow_id: workflow_id.clone(),
            workflow_type: Some(WorkflowType {
                name: workflow_type.to_string(),
            }),
            task_list: Some(TaskList {
                name: options.task_list,
                kind: TaskListKind::Normal,
            }),
            input: args.map(|a| a.to_vec()),
            execution_start_to_close_timeout_seconds: options
                .execution_start_to_close_timeout
                .map(|d| d.as_secs() as i32),
            task_start_to_close_timeout_seconds: options
                .task_start_to_close_timeout
                .map(|d| d.as_secs() as i32),
            identity: self.options.identity.clone(),
            request_id,
            workflow_id_reuse_policy: Some(convert_workflow_id_reuse_policy(
                options.workflow_id_reuse_policy,
            )),
            retry_policy: options.retry_policy.map(convert_retry_policy),
            cron_schedule: options.cron_schedule,
            memo: options.memo.map(|m| Memo { fields: m }),
            search_attributes: options
                .search_attributes
                .map(|s| SearchAttributes { indexed_fields: s }),
            header: options.header.map(|h| Header { fields: h }),
            delay_start_seconds: options.delay_start.map(|d| d.as_secs() as i32),
            jitter_start_seconds: options.jitter_start.map(|d| d.as_secs() as i32),
            first_execution_run_id: None,
            first_decision_task_backoff_seconds: None,
            partition_config: None,
        };

        let _response = self
            .service
            .start_workflow_execution_with_extensions(request, extensions)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(WorkflowExecutionAsync { workflow_id })
    }

    async fn execute_workflow(
        &self,
        options: StartWorkflowOptions,
        workflow_type: &str,
        args: Option<&[u8]>,
    ) -> CadenceResult<Box<dyn WorkflowRun>> {
        let request_id = Uuid::new_v4().to_string();
        let workflow_id = options.id.clone();
        let extensions = build_start_extensions(&options);

        let request = StartWorkflowExecutionRequest {
            domain: self.domain.clone(),
            workflow_id: workflow_id.clone(),
            workflow_type: Some(WorkflowType {
                name: workflow_type.to_string(),
            }),
            task_list: Some(TaskList {
                name: options.task_list,
                kind: TaskListKind::Normal,
            }),
            input: args.map(|a| a.to_vec()),
            execution_start_to_close_timeout_seconds: options
                .execution_start_to_close_timeout
                .map(|d| d.as_secs() as i32),
            task_start_to_close_timeout_seconds: options
                .task_start_to_close_timeout
                .map(|d| d.as_secs() as i32),
            identity: self.options.identity.clone(),
            request_id,
            workflow_id_reuse_policy: Some(convert_workflow_id_reuse_policy(
                options.workflow_id_reuse_policy,
            )),
            retry_policy: options.retry_policy.map(convert_retry_policy),
            cron_schedule: options.cron_schedule,
            memo: options.memo.map(|m| Memo { fields: m }),
            search_attributes: options
                .search_attributes
                .map(|s| SearchAttributes { indexed_fields: s }),
            header: options.header.map(|h| Header { fields: h }),
            delay_start_seconds: options.delay_start.map(|d| d.as_secs() as i32),
            jitter_start_seconds: options.jitter_start.map(|d| d.as_secs() as i32),
            first_execution_run_id: None,
            first_decision_task_backoff_seconds: None,
            partition_config: None,
        };

        let response = self
            .service
            .start_workflow_execution_with_extensions(request, extensions)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(Box::new(WorkflowRunImpl {
            client: Arc::new(WorkflowClient {
                service: self.service.clone(),
                domain: self.domain.clone(),
                options: self.options.clone(),
            }),
            workflow_id,
            run_id: response.run_id,
        }))
    }

    fn get_workflow(&self, workflow_id: &str, run_id: Option<&str>) -> Box<dyn WorkflowRun> {
        Box::new(WorkflowRunImpl {
            client: Arc::new(WorkflowClient {
                service: self.service.clone(),
                domain: self.domain.clone(),
                options: self.options.clone(),
            }),
            workflow_id: workflow_id.to_string(),
            run_id: run_id.unwrap_or_default().to_string(),
        })
    }

    async fn signal_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        signal_name: &str,
        arg: Option<&[u8]>,
    ) -> CadenceResult<()> {
        let request = SignalWorkflowExecutionRequest {
            domain: self.domain.clone(),
            workflow_execution: Some(make_proto_execution(
                workflow_id.to_string(),
                run_id.unwrap_or_default().to_string(),
            )),
            signal_name: signal_name.to_string(),
            input: arg.map(|a| a.to_vec()),
            identity: self.options.identity.clone(),
            request_id: Uuid::new_v4().to_string(),
            control: None,
        };

        self.service
            .signal_workflow_execution(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;
        Ok(())
    }

    async fn signal_with_start_workflow(
        &self,
        workflow_id: &str,
        signal_name: &str,
        signal_arg: Option<&[u8]>,
        options: StartWorkflowOptions,
        workflow_type: &str,
        workflow_args: Option<&[u8]>,
    ) -> CadenceResult<WorkflowExecution> {
        let request = SignalWithStartWorkflowExecutionRequest {
            domain: self.domain.clone(),
            workflow_id: workflow_id.to_string(),
            workflow_type: Some(WorkflowType {
                name: workflow_type.to_string(),
            }),
            task_list: Some(TaskList {
                name: options.task_list,
                kind: TaskListKind::Normal,
            }),
            input: workflow_args.map(|a| a.to_vec()),
            execution_start_to_close_timeout_seconds: options
                .execution_start_to_close_timeout
                .map(|d| d.as_secs() as i32),
            task_start_to_close_timeout_seconds: options
                .task_start_to_close_timeout
                .map(|d| d.as_secs() as i32),
            identity: self.options.identity.clone(),
            request_id: Uuid::new_v4().to_string(),
            workflow_id_reuse_policy: Some(convert_workflow_id_reuse_policy(
                options.workflow_id_reuse_policy,
            )),
            signal_name: signal_name.to_string(),
            signal_input: signal_arg.map(|a| a.to_vec()),
            retry_policy: options.retry_policy.map(convert_retry_policy),
            cron_schedule: options.cron_schedule,
            memo: options.memo.map(|m| Memo { fields: m }),
            search_attributes: options
                .search_attributes
                .map(|s| SearchAttributes { indexed_fields: s }),
            header: options.header.map(|h| Header { fields: h }),
        };

        let response = self
            .service
            .signal_with_start_workflow_execution(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(WorkflowExecution::new(workflow_id, response.run_id))
    }

    async fn signal_with_start_workflow_async(
        &self,
        workflow_id: &str,
        signal_name: &str,
        signal_arg: Option<&[u8]>,
        options: StartWorkflowOptions,
        workflow_type: &str,
        workflow_args: Option<&[u8]>,
    ) -> CadenceResult<WorkflowExecutionAsync> {
        // Reuse logic from signal_with_start_workflow, or just copy paste to return Async struct
        let _exec = self
            .signal_with_start_workflow(
                workflow_id,
                signal_name,
                signal_arg,
                options,
                workflow_type,
                workflow_args,
            )
            .await?;
        Ok(WorkflowExecutionAsync {
            workflow_id: workflow_id.to_string(),
        })
    }

    async fn cancel_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        reason: Option<&str>,
    ) -> CadenceResult<()> {
        self.cancel_workflow_with_options(
            workflow_id,
            run_id,
            CancelWorkflowOptions {
                cause: reason.map(|s| s.to_string()),
            },
        )
        .await
    }

    async fn cancel_workflow_with_options(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        options: CancelWorkflowOptions,
    ) -> CadenceResult<()> {
        let request = RequestCancelWorkflowExecutionRequest {
            domain: self.domain.clone(),
            workflow_execution: Some(make_proto_execution(
                workflow_id.to_string(),
                run_id.unwrap_or_default().to_string(),
            )),
            identity: self.options.identity.clone(),
            request_id: Uuid::new_v4().to_string(),
            cause: options.cause,
            first_execution_run_id: None,
        };
        self.service
            .request_cancel_workflow_execution(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;
        Ok(())
    }

    async fn terminate_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        reason: Option<&str>,
        details: Option<&[u8]>,
    ) -> CadenceResult<()> {
        let request = TerminateWorkflowExecutionRequest {
            domain: self.domain.clone(),
            workflow_execution: Some(make_proto_execution(
                workflow_id.to_string(),
                run_id.unwrap_or_default().to_string(),
            )),
            reason: reason.map(|s| s.to_string()),
            details: details.map(|d| d.to_vec()),
            identity: self.options.identity.clone(),
            first_execution_run_id: None,
        };
        self.service
            .terminate_workflow_execution(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;
        Ok(())
    }

    async fn get_workflow_history(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        is_long_poll: bool,
        filter_type: HistoryEventFilterType,
    ) -> CadenceResult<HistoryEventIterator> {
        let request = GetWorkflowExecutionHistoryRequest {
            domain: self.domain.clone(),
            execution: Some(make_proto_execution(
                workflow_id.to_string(),
                run_id.unwrap_or_default().to_string(),
            )),
            page_size: 1000,
            next_page_token: None,
            wait_for_new_event: is_long_poll,
            history_event_filter_type: Some(filter_type),
            skip_archival: true,
            query_consistency_level: None,
        };

        let response = self
            .service
            .get_workflow_execution_history(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(HistoryEventIterator::new(
            response.history.map(|h| h.events).unwrap_or_default(),
            response.next_page_token,
            Arc::new(WorkflowClient {
                service: self.service.clone(),
                domain: self.domain.clone(),
                options: self.options.clone(),
            }),
            workflow_id.to_string(),
            run_id.map(|s| s.to_string()),
            filter_type,
        ))
    }

    async fn get_workflow_history_with_options(
        &self,
        request: GetWorkflowHistoryWithOptionsRequest,
    ) -> CadenceResult<HistoryEventIterator> {
        let req = GetWorkflowExecutionHistoryRequest {
            domain: self.domain.clone(),
            execution: Some(make_proto_execution(
                request.workflow_id.clone(),
                request.run_id.clone().unwrap_or_default(),
            )),
            page_size: request.page_size,
            next_page_token: None,
            wait_for_new_event: request.wait_for_new_event,
            history_event_filter_type: Some(request.history_event_filter_type),
            skip_archival: request.skip_archival,
            query_consistency_level: None,
        };

        let response = self
            .service
            .get_workflow_execution_history(req)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(HistoryEventIterator::new(
            response.history.map(|h| h.events).unwrap_or_default(),
            response.next_page_token,
            Arc::new(WorkflowClient {
                service: self.service.clone(),
                domain: self.domain.clone(),
                options: self.options.clone(),
            }),
            request.workflow_id,
            request.run_id,
            request.history_event_filter_type,
        ))
    }

    async fn complete_activity(
        &self,
        task_token: &[u8],
        result: Option<&[u8]>,
        error: Option<CadenceError>,
    ) -> CadenceResult<()> {
        if let Some(err) = error {
            let request = RespondActivityTaskFailedRequest {
                task_token: task_token.to_vec(),
                reason: Some(err.to_string()),
                details: None,
                identity: self.options.identity.clone(),
            };
            self.service
                .respond_activity_task_failed(request)
                .await
                .map_err(|e| CadenceError::Other(e.to_string()))?;
        } else {
            let request = RespondActivityTaskCompletedRequest {
                task_token: task_token.to_vec(),
                result: result.map(|r| r.to_vec()),
                identity: self.options.identity.clone(),
            };
            self.service
                .respond_activity_task_completed(request)
                .await
                .map_err(|e| CadenceError::Other(e.to_string()))?;
        }
        Ok(())
    }

    async fn complete_activity_by_id(
        &self,
        domain: &str,
        workflow_id: &str,
        run_id: Option<&str>,
        activity_id: &str,
        result: Option<&[u8]>,
        error: Option<CadenceError>,
    ) -> CadenceResult<()> {
        if let Some(err) = error {
            let request = RespondActivityTaskFailedByIdRequest {
                domain: domain.to_string(),
                workflow_id: workflow_id.to_string(),
                run_id: run_id.map(|s| s.to_string()),
                activity_id: activity_id.to_string(),
                reason: Some(err.to_string()),
                details: None,
                identity: self.options.identity.clone(),
            };
            self.service
                .respond_activity_task_failed_by_id(request)
                .await
                .map_err(|e| CadenceError::Other(e.to_string()))?;
        } else {
            let request = RespondActivityTaskCompletedByIdRequest {
                domain: domain.to_string(),
                workflow_id: workflow_id.to_string(),
                run_id: run_id.map(|s| s.to_string()),
                activity_id: activity_id.to_string(),
                result: result.map(|r| r.to_vec()),
                identity: self.options.identity.clone(),
            };
            self.service
                .respond_activity_task_completed_by_id(request)
                .await
                .map_err(|e| CadenceError::Other(e.to_string()))?;
        }
        Ok(())
    }

    async fn record_activity_heartbeat(
        &self,
        task_token: &[u8],
        details: Option<&[u8]>,
    ) -> CadenceResult<bool> {
        let request = RecordActivityTaskHeartbeatRequest {
            task_token: task_token.to_vec(),
            details: details.map(|d| d.to_vec()),
            identity: self.options.identity.clone(),
        };
        let response = self
            .service
            .record_activity_task_heartbeat(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;
        Ok(response.cancel_requested)
    }

    async fn record_activity_heartbeat_by_id(
        &self,
        domain: &str,
        workflow_id: &str,
        run_id: Option<&str>,
        activity_id: &str,
        details: Option<&[u8]>,
    ) -> CadenceResult<bool> {
        let request = RecordActivityTaskHeartbeatByIdRequest {
            domain: domain.to_string(),
            workflow_id: workflow_id.to_string(),
            run_id: run_id.map(|s| s.to_string()),
            activity_id: activity_id.to_string(),
            details: details.map(|d| d.to_vec()),
            identity: self.options.identity.clone(),
        };
        let response = self
            .service
            .record_activity_task_heartbeat_by_id(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;
        Ok(response.cancel_requested)
    }

    async fn list_closed_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse> {
        // Map ListWorkflowExecutionsRequest to ListClosedWorkflowExecutionsRequest
        let req = ListClosedWorkflowExecutionsRequest {
            domain: self.domain.clone(),
            maximum_page_size: request.maximum_page_size,
            next_page_token: request.next_page_token,
            start_time_filter: request.start_time_filter.map(|f| {
                crabdance_proto::workflow_service::StartTimeFilter {
                    earliest_time: Some(f.earliest_time.timestamp_nanos_opt().unwrap_or(0)),
                    latest_time: Some(f.latest_time.timestamp_nanos_opt().unwrap_or(0)),
                }
            }),
            execution_filter: request.execution_filter.map(|f| {
                crabdance_proto::workflow_service::WorkflowExecutionFilter {
                    workflow_id: f.workflow_id,
                }
            }),
            type_filter: request
                .type_filter
                .map(|f| crabdance_proto::workflow_service::WorkflowTypeFilter { name: f.name }),
            status_filter: request.status_filter,
        };

        let response = self
            .service
            .list_closed_workflow_executions(req)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(ListWorkflowExecutionsResponse {
            executions: response
                .executions
                .into_iter()
                .map(convert_workflow_execution_info)
                .collect(),
            next_page_token: response.next_page_token,
        })
    }

    async fn list_open_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse> {
        let req = ListOpenWorkflowExecutionsRequest {
            domain: self.domain.clone(),
            maximum_page_size: request.maximum_page_size,
            next_page_token: request.next_page_token,
            start_time_filter: request.start_time_filter.map(|f| {
                crabdance_proto::workflow_service::StartTimeFilter {
                    earliest_time: Some(f.earliest_time.timestamp_nanos_opt().unwrap_or(0)),
                    latest_time: Some(f.latest_time.timestamp_nanos_opt().unwrap_or(0)),
                }
            }),
            execution_filter: request.execution_filter.map(|f| {
                crabdance_proto::workflow_service::WorkflowExecutionFilter {
                    workflow_id: f.workflow_id,
                }
            }),
            type_filter: request
                .type_filter
                .map(|f| crabdance_proto::workflow_service::WorkflowTypeFilter { name: f.name }),
            status_filter: request.status_filter,
        };

        let response = self
            .service
            .list_open_workflow_executions(req)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(ListWorkflowExecutionsResponse {
            executions: response
                .executions
                .into_iter()
                .map(convert_workflow_execution_info)
                .collect(),
            next_page_token: response.next_page_token,
        })
    }

    async fn list_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse> {
        // Mirrors Go's ListWorkflow: the unified, query-based visibility API
        // covering both open and closed executions.
        let req = crabdance_proto::generated::ListWorkflowExecutionsRequest {
            domain: self.domain.clone(),
            page_size: request.maximum_page_size,
            next_page_token: request.next_page_token.unwrap_or_default(),
            query: request.query.unwrap_or_default(),
        };

        let response = self
            .service
            .list_workflow_executions(req)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(ListWorkflowExecutionsResponse {
            executions: response
                .executions
                .into_iter()
                .map(convert_workflow_execution_info)
                .collect(),
            next_page_token: response.next_page_token,
        })
    }

    async fn list_archived_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse> {
        // Mirrors Go's ListArchivedWorkflow: query-based visibility over the
        // archival store.
        let req = crabdance_proto::generated::ListArchivedWorkflowExecutionsRequest {
            domain: self.domain.clone(),
            page_size: request.maximum_page_size,
            next_page_token: request.next_page_token.unwrap_or_default(),
            query: request.query.unwrap_or_default(),
        };

        let response = self
            .service
            .list_archived_workflow_executions(req)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(ListWorkflowExecutionsResponse {
            executions: response
                .executions
                .into_iter()
                .map(convert_workflow_execution_info)
                .collect(),
            next_page_token: response.next_page_token,
        })
    }

    async fn scan_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse> {
        // Mirrors Go's ScanWorkflow: query-based visibility scan with paging.
        let req = ScanWorkflowExecutionsRequest {
            domain: self.domain.clone(),
            page_size: request.maximum_page_size,
            next_page_token: request.next_page_token,
            query: request.query,
        };

        let response = self
            .service
            .scan_workflow_executions(req)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(ListWorkflowExecutionsResponse {
            executions: response
                .executions
                .into_iter()
                .map(convert_workflow_execution_info)
                .collect(),
            next_page_token: response.next_page_token,
        })
    }

    async fn count_workflows(
        &self,
        request: CountWorkflowExecutionsRequest,
    ) -> CadenceResult<CountWorkflowExecutionsResponse> {
        let req = crabdance_proto::workflow_service::CountWorkflowExecutionsRequest {
            domain: self.domain.clone(),
            query: Some(request.query),
        };

        let response = self
            .service
            .count_workflow_executions(req)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(CountWorkflowExecutionsResponse {
            count: response.count,
        })
    }

    async fn get_search_attributes(&self) -> CadenceResult<SearchAttributesResponse> {
        let request = GetSearchAttributesRequest {};
        let response = self
            .service
            .get_search_attributes(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        // Convert IndexedValueType enum to string representation
        let keys = response
            .keys
            .into_iter()
            .map(|(k, v)| (k, format!("{:?}", v)))
            .collect();

        Ok(SearchAttributesResponse { keys })
    }

    async fn query_workflow(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
        query_type: &str,
        args: Option<&[u8]>,
    ) -> CadenceResult<EncodedValue> {
        // Mirror Go: the simple QueryWorkflow delegates to the options-based
        // form. Where Go's QueryWorkflowWithOptions returns the rejection in the
        // response, the simple form has no place to carry it, so we surface a
        // rejection as an error (the result would otherwise be silently empty).
        let response = self
            .query_workflow_with_options(QueryWorkflowWithOptionsRequest {
                workflow_id: workflow_id.to_string(),
                run_id: run_id.map(|s| s.to_string()),
                query_type: query_type.to_string(),
                query_args: args.map(|a| a.to_vec()),
                query_consistency_level: QueryConsistencyLevel::Unspecified,
                query_reject_condition: None,
            })
            .await?;

        if let Some(rejected) = response.query_rejected {
            return Err(CadenceError::Other(format!(
                "query rejected: workflow close status {:?}",
                rejected.close_status
            )));
        }

        match response.query_result {
            Some(result) => Ok(EncodedValue::new(result)),
            None => Err(CadenceError::Other("query returned no result".to_string())),
        }
    }

    async fn query_workflow_with_options(
        &self,
        request: QueryWorkflowWithOptionsRequest,
    ) -> CadenceResult<QueryWorkflowWithOptionsResponse> {
        let pb_request = QueryWorkflowRequest {
            domain: self.domain.clone(),
            execution: Some(make_proto_execution(
                request.workflow_id.clone(),
                request.run_id.clone().unwrap_or_default(),
            )),
            query: Some(WorkflowQuery {
                query_type: request.query_type.clone(),
                query_args: request.query_args.clone(),
            }),
            query_consistency_level: convert_query_consistency_level(
                request.query_consistency_level,
            ),
            query_reject_condition: request
                .query_reject_condition
                .map(convert_query_reject_condition),
        };

        let response = self
            .service
            .query_workflow(pb_request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        // Surface the server-side query rejection (close-status mismatch)
        // instead of dropping it, matching Go's QueryWorkflowWithOptions.
        Ok(QueryWorkflowWithOptionsResponse {
            query_result: response.query_result,
            query_rejected: map_query_rejected(response.query_rejected),
        })
    }

    async fn reset_workflow(
        &self,
        request: ResetWorkflowExecutionRequest,
    ) -> CadenceResult<ResetWorkflowExecutionResponse> {
        let req = crabdance_proto::workflow_service::ResetWorkflowExecutionRequest {
            domain: if request.domain.is_empty() {
                self.domain.clone()
            } else {
                request.domain
            },
            workflow_execution: Some(make_proto_execution(
                request.workflow_execution.workflow_id,
                request.workflow_execution.run_id,
            )),
            reason: request.reason.unwrap_or_default(),
            decision_finish_event_id: request.decision_finish_event_id,
            request_id: Some(request.request_id),
            skip_signal_reapply: false,
        };

        let response = self
            .service
            .reset_workflow_execution(req)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(ResetWorkflowExecutionResponse {
            run_id: response.run_id,
        })
    }

    async fn describe_workflow_execution(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
    ) -> CadenceResult<DescribeWorkflowExecutionResponse> {
        let request = DescribeWorkflowExecutionRequest {
            domain: self.domain.clone(),
            execution: Some(make_proto_execution(
                workflow_id.to_string(),
                run_id.unwrap_or_default().to_string(),
            )),
            query_consistency_level: None,
        };
        let response = self
            .service
            .describe_workflow_execution(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(DescribeWorkflowExecutionResponse {
            execution_configuration: convert_execution_configuration(
                response.execution_configuration.unwrap(),
            ),
            workflow_execution_info: convert_workflow_execution_info(
                response.workflow_execution_info.unwrap(),
            ),
            pending_activities: response
                .pending_activities
                .into_iter()
                .map(convert_pending_activity)
                .collect(),
            pending_children: response
                .pending_children
                .into_iter()
                .map(convert_pending_child)
                .collect(),
        })
    }

    async fn describe_workflow_execution_with_options(
        &self,
        request: DescribeWorkflowExecutionWithOptionsRequest,
    ) -> CadenceResult<DescribeWorkflowExecutionResponse> {
        self.describe_workflow_execution(&request.workflow_id, request.run_id.as_deref())
            .await
    }

    async fn describe_task_list(
        &self,
        task_list: &str,
        task_list_type: TaskListType,
    ) -> CadenceResult<DescribeTaskListResponse> {
        let request = crabdance_proto::workflow_service::DescribeTaskListRequest {
            domain: self.domain.clone(),
            task_list: Some(TaskList {
                name: task_list.to_string(),
                kind: TaskListKind::Normal,
            }),
            task_list_type,
            include_task_list_status: true,
        };

        let response = self
            .service
            .describe_task_list(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        // Convert proto response to client response
        Ok(DescribeTaskListResponse {
            pollers: response
                .pollers
                .into_iter()
                .map(|p| PollersInfo {
                    identity: p.identity,
                    last_access_time: p
                        .last_access_time
                        .and_then(|nanos| {
                            chrono::DateTime::from_timestamp(
                                nanos / 1_000_000_000,
                                (nanos % 1_000_000_000) as u32,
                            )
                        })
                        .unwrap_or_default(),
                })
                .collect(),
        })
    }

    async fn refresh_workflow_tasks(
        &self,
        workflow_id: &str,
        run_id: Option<&str>,
    ) -> CadenceResult<()> {
        let request = RefreshWorkflowTasksRequest {
            domain: self.domain.clone(),
            execution: Some(make_proto_execution(
                workflow_id.to_string(),
                run_id.unwrap_or_default().to_string(),
            )),
        };

        self.service
            .refresh_workflow_tasks(request)
            .await
            .map_err(|e| CadenceError::Other(e.to_string()))?;

        Ok(())
    }
}

pub trait MetricsScope: Send + Sync {
    fn counter(&self, name: &str) -> Box<dyn Counter>;
    fn timer(&self, name: &str) -> Box<dyn Timer>;
    fn gauge(&self, name: &str) -> Box<dyn Gauge>;
}

pub trait Counter: Send + Sync {
    fn inc(&self, delta: i64);
}

pub trait Timer: Send + Sync {
    fn record(&self, duration: Duration);
}

pub trait Gauge: Send + Sync {
    fn update(&self, value: f64);
}

pub trait Logger: Send + Sync {
    fn debug(&self, msg: &str);
    fn info(&self, msg: &str);
    fn warn(&self, msg: &str);
    fn error(&self, msg: &str);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FeatureFlags {
    pub enable_execution_cache: bool,
    pub enable_async_workflow_consistency: bool,
}

struct WorkflowRunImpl {
    client: Arc<WorkflowClient>,
    workflow_id: String,
    run_id: String,
}

#[async_trait]
impl WorkflowRun for WorkflowRunImpl {
    fn get_id(&self) -> &str {
        &self.workflow_id
    }

    fn run_id(&self) -> &str {
        &self.run_id
    }

    async fn get(&self) -> CadenceResult<Option<Vec<u8>>> {
        loop {
            let request = GetWorkflowExecutionHistoryRequest {
                domain: self.client.domain.clone(),
                execution: Some(make_proto_execution(
                    self.workflow_id.clone(),
                    self.run_id.clone(),
                )),
                page_size: 100,
                next_page_token: None,
                wait_for_new_event: true,
                history_event_filter_type: Some(HistoryEventFilterType::CloseEvent),
                skip_archival: true,
                query_consistency_level: None,
            };

            let response = self
                .client
                .service
                .get_workflow_execution_history(request)
                .await
                .map_err(|e| CadenceError::Other(e.to_string()))?;

            if let Some(history) = response.history {
                for event in history.events {
                    match event.event_type {
                        EventType::WorkflowExecutionCompleted => {
                            if let Some(
                                EventAttributes::WorkflowExecutionCompletedEventAttributes(attr),
                            ) = event.attributes
                            {
                                return Ok(attr.result);
                            }
                        }
                        EventType::WorkflowExecutionFailed => {
                            if let Some(EventAttributes::WorkflowExecutionFailedEventAttributes(
                                attr,
                            )) = event.attributes
                            {
                                return Err(CadenceError::WorkflowExecutionFailed(
                                    attr.reason.unwrap_or_default(),
                                    attr.details.unwrap_or_default(),
                                ));
                            }
                        }
                        EventType::WorkflowExecutionTimedOut => {
                            return Err(CadenceError::WorkflowExecutionTimedOut);
                        }
                        EventType::WorkflowExecutionCanceled => {
                            return Err(CadenceError::WorkflowExecutionCancelled);
                        }
                        EventType::WorkflowExecutionTerminated => {
                            return Err(CadenceError::WorkflowExecutionTerminated);
                        }
                        _ => {}
                    }
                }
            }

            // If we are here, we didn't find the close event, but wait_for_new_event returned.
            // Loop again.
        }
    }

    async fn get_with_timeout(&self, timeout: Duration) -> CadenceResult<Option<Vec<u8>>> {
        // Bound the blocking long-poll loop in `get()` by a client-side deadline.
        match tokio::time::timeout(timeout, self.get()).await {
            Ok(result) => result,
            Err(_) => Err(CadenceError::Other(format!(
                "timed out waiting for workflow result after {timeout:?}"
            ))),
        }
    }
}

// Helpers

/// Build the gRPC-level start extensions (`first_run_at`, `cron_overlap_policy`,
/// `active_cluster_selection_policy`) from idiomatic [`StartWorkflowOptions`].
fn build_start_extensions(options: &StartWorkflowOptions) -> crate::grpc::StartWorkflowExtensions {
    crate::grpc::StartWorkflowExtensions {
        first_run_at_unix_nanos: options.first_run_at.and_then(|t| t.timestamp_nanos_opt()),
        cron_overlap_policy: options.cron_overlap_policy.to_proto(),
        active_cluster_selection_policy: options.active_cluster_selection_policy.as_ref().map(
            |policy| crabdance_proto::generated::ActiveClusterSelectionPolicy {
                cluster_attribute: policy.cluster_attribute.as_ref().map(|attr| {
                    crabdance_proto::generated::ClusterAttribute {
                        scope: attr.scope.clone(),
                        name: attr.name.clone(),
                    }
                }),
                ..Default::default()
            },
        ),
    }
}

/// Map the api-level query rejection onto the client-facing [`QueryRejected`].
///
/// The api wrapper carries an optional close status; the client surface uses a
/// concrete status, defaulting to `Completed` when the server omits it.
fn map_query_rejected(
    rejected: Option<crabdance_proto::workflow_service::QueryRejected>,
) -> Option<QueryRejected> {
    rejected.map(|r| QueryRejected {
        close_status: r
            .close_status
            .unwrap_or(WorkflowExecutionCloseStatus::Completed),
    })
}

/// Map a [`QueryConsistencyLevel`] to the proto-level value. `Unspecified` maps
/// to `None`, letting the server apply its default.
fn convert_query_consistency_level(
    level: QueryConsistencyLevel,
) -> Option<crabdance_proto::shared::QueryConsistencyLevel> {
    match level {
        QueryConsistencyLevel::Unspecified => None,
        QueryConsistencyLevel::Eventual => {
            Some(crabdance_proto::shared::QueryConsistencyLevel::Eventual)
        }
        QueryConsistencyLevel::Strong => {
            Some(crabdance_proto::shared::QueryConsistencyLevel::Strong)
        }
    }
}

/// Map the client-side [`QueryRejectCondition`] to the proto-level value.
fn convert_query_reject_condition(
    condition: QueryRejectCondition,
) -> crabdance_proto::shared::QueryRejectCondition {
    match condition {
        QueryRejectCondition::NotOpen => crabdance_proto::shared::QueryRejectCondition::NotOpen,
        QueryRejectCondition::NotCompletedCleanly => {
            crabdance_proto::shared::QueryRejectCondition::NotCompletedCleanly
        }
    }
}

fn convert_retry_policy(policy: RetryPolicy) -> crabdance_proto::shared::RetryPolicy {
    crabdance_proto::shared::RetryPolicy {
        initial_interval_in_seconds: policy.initial_interval.as_secs() as i32,
        backoff_coefficient: policy.backoff_coefficient,
        maximum_interval_in_seconds: policy.maximum_interval.as_secs() as i32,
        maximum_attempts: policy.maximum_attempts,
        non_retryable_error_types: policy.non_retryable_error_types,
        expiration_interval_in_seconds: policy.expiration_interval.as_secs() as i32,
    }
}

fn convert_workflow_id_reuse_policy(
    policy: WorkflowIdReusePolicy,
) -> crabdance_proto::shared::WorkflowIdReusePolicy {
    match policy {
        WorkflowIdReusePolicy::AllowDuplicateFailedOnly => {
            crabdance_proto::shared::WorkflowIdReusePolicy::AllowDuplicateFailedOnly
        }
        WorkflowIdReusePolicy::AllowDuplicate => {
            crabdance_proto::shared::WorkflowIdReusePolicy::AllowDuplicate
        }
        WorkflowIdReusePolicy::RejectDuplicate => {
            crabdance_proto::shared::WorkflowIdReusePolicy::RejectDuplicate
        }
        WorkflowIdReusePolicy::TerminateIfRunning => {
            crabdance_proto::shared::WorkflowIdReusePolicy::TerminateIfRunning
        }
    }
}

fn convert_execution_configuration(
    config: ProtoWorkflowExecutionConfiguration,
) -> WorkflowExecutionConfiguration {
    WorkflowExecutionConfiguration {
        task_list: config.task_list.map(|t| t.name).unwrap_or_default(),
        execution_start_to_close_timeout: Duration::from_secs(
            config.execution_start_to_close_timeout_seconds as u64,
        ),
        task_start_to_close_timeout: Duration::from_secs(
            config.task_start_to_close_timeout_seconds as u64,
        ),
    }
}

fn convert_workflow_execution_info(info: ProtoWorkflowExecutionInfo) -> WorkflowExecutionInfo {
    WorkflowExecutionInfo {
        execution: info
            .execution
            .map(|e| WorkflowExecution::new(e.workflow_id, e.run_id))
            .unwrap_or(WorkflowExecution::new("", "")),
        workflow_type: info.workflow_type.map(|t| t.name).unwrap_or_default(),
        start_time: info.start_time.map(chrono::DateTime::from_timestamp_nanos),
        close_time: info.close_time.map(chrono::DateTime::from_timestamp_nanos),
        close_status: info.close_status,
        history_length: info.history_length,
    }
}

fn convert_pending_activity(info: ProtoPendingActivityInfo) -> PendingActivityInfo {
    PendingActivityInfo {
        activity_id: info.activity_id,
        activity_type: info.activity_type.map(|t| t.name).unwrap_or_default(),
        state: match info.state {
            Some(ProtoPendingActivityState::Scheduled) => PendingActivityState::Scheduled,
            Some(ProtoPendingActivityState::Started) => PendingActivityState::Started,
            Some(ProtoPendingActivityState::CancelRequested) => {
                PendingActivityState::CancelRequested
            }
            None => PendingActivityState::Scheduled, // Default
        },
    }
}

fn convert_pending_child(info: ProtoPendingChildExecutionInfo) -> PendingChildExecutionInfo {
    PendingChildExecutionInfo {
        workflow_id: info.workflow_id,
        run_id: info.run_id,
        workflow_type: info.workflow_type_name,
    }
}

fn make_proto_execution(
    workflow_id: String,
    run_id: String,
) -> crabdance_proto::shared::WorkflowExecution {
    crabdance_proto::shared::WorkflowExecution {
        workflow_id,
        run_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- StartWorkflowOptions / extension fields (#22) -------------------

    #[test]
    fn test_start_workflow_options_default_sets_new_parity_fields() {
        let opts = StartWorkflowOptions::default();
        assert!(opts.jitter_start.is_none());
        assert!(opts.first_run_at.is_none());
        assert_eq!(opts.cron_overlap_policy, CronOverlapPolicy::Skipped);
        assert!(opts.active_cluster_selection_policy.is_none());
    }

    #[test]
    fn test_cron_overlap_policy_default_is_skipped() {
        assert_eq!(CronOverlapPolicy::default(), CronOverlapPolicy::Skipped);
    }

    #[test]
    fn test_cron_overlap_policy_to_proto_maps_skipped_and_buffer_one() {
        // Proto discriminants: Skipped=1, BufferOne=2 (0=Invalid is never sent).
        assert_eq!(CronOverlapPolicy::Skipped.to_proto(), 1);
        assert_eq!(CronOverlapPolicy::BufferOne.to_proto(), 2);
    }

    #[test]
    fn test_build_start_extensions_maps_first_run_at_to_unix_nanos() {
        let when = chrono::DateTime::from_timestamp(1_700_000_000, 123).unwrap();
        let opts = StartWorkflowOptions {
            first_run_at: Some(when),
            ..Default::default()
        };
        let ext = build_start_extensions(&opts);
        assert_eq!(
            ext.first_run_at_unix_nanos,
            Some(1_700_000_000 * 1_000_000_000 + 123)
        );
    }

    #[test]
    fn test_build_start_extensions_defaults_to_skipped_cron_overlap_policy() {
        let ext = build_start_extensions(&StartWorkflowOptions::default());
        assert_eq!(ext.first_run_at_unix_nanos, None);
        assert_eq!(ext.cron_overlap_policy, 1); // Skipped
        assert!(ext.active_cluster_selection_policy.is_none());
    }

    #[test]
    fn test_build_start_extensions_maps_buffer_one_cron_overlap_policy() {
        let opts = StartWorkflowOptions {
            cron_overlap_policy: CronOverlapPolicy::BufferOne,
            ..Default::default()
        };
        assert_eq!(build_start_extensions(&opts).cron_overlap_policy, 2);
    }

    #[test]
    fn test_build_start_extensions_maps_active_cluster_selection_policy() {
        let opts = StartWorkflowOptions {
            active_cluster_selection_policy: Some(ActiveClusterSelectionPolicy {
                cluster_attribute: Some(ClusterAttribute {
                    scope: "region".to_string(),
                    name: "us-west".to_string(),
                }),
            }),
            ..Default::default()
        };
        let ext = build_start_extensions(&opts);
        let policy = ext
            .active_cluster_selection_policy
            .expect("policy should be present");
        let attr = policy
            .cluster_attribute
            .expect("cluster attribute should be present");
        assert_eq!(attr.scope, "region");
        assert_eq!(attr.name, "us-west");
    }

    // ---- Query consistency / reject condition mapping (#17) --------------

    #[test]
    fn test_convert_query_consistency_level_unspecified_is_none() {
        assert!(convert_query_consistency_level(QueryConsistencyLevel::Unspecified).is_none());
    }

    #[test]
    fn test_convert_query_consistency_level_maps_eventual_and_strong() {
        assert_eq!(
            convert_query_consistency_level(QueryConsistencyLevel::Eventual),
            Some(crabdance_proto::shared::QueryConsistencyLevel::Eventual)
        );
        assert_eq!(
            convert_query_consistency_level(QueryConsistencyLevel::Strong),
            Some(crabdance_proto::shared::QueryConsistencyLevel::Strong)
        );
    }

    #[test]
    fn test_convert_query_reject_condition_maps_both_variants() {
        assert_eq!(
            convert_query_reject_condition(QueryRejectCondition::NotOpen),
            crabdance_proto::shared::QueryRejectCondition::NotOpen
        );
        assert_eq!(
            convert_query_reject_condition(QueryRejectCondition::NotCompletedCleanly),
            crabdance_proto::shared::QueryRejectCondition::NotCompletedCleanly
        );
    }

    // ---- query_rejected surfacing (#17) ---------------------------------

    #[test]
    fn test_map_query_rejected_none_passes_through() {
        assert!(map_query_rejected(None).is_none());
    }

    #[test]
    fn test_map_query_rejected_surfaces_close_status() {
        let api = crabdance_proto::workflow_service::QueryRejected {
            close_status: Some(WorkflowExecutionCloseStatus::Failed),
        };
        let mapped = map_query_rejected(Some(api)).expect("rejection should be surfaced");
        assert_eq!(mapped.close_status, WorkflowExecutionCloseStatus::Failed);
    }

    #[test]
    fn test_map_query_rejected_defaults_missing_close_status_to_completed() {
        let api = crabdance_proto::workflow_service::QueryRejected { close_status: None };
        let mapped = map_query_rejected(Some(api)).expect("rejection should be surfaced");
        assert_eq!(mapped.close_status, WorkflowExecutionCloseStatus::Completed);
    }

    // ---- Integration tests (require a running Cadence server) ------------
    //
    // Mirror the connection pattern in `crabdance/tests/grpc_integration.rs`.
    // Run with: cargo test -p crabdance_client -- --ignored --test-threads=1

    #[cfg(feature = "integration")]
    const CADENCE_GRPC_ENDPOINT: &str = "http://localhost:7833";
    #[cfg(feature = "integration")]
    const TEST_DOMAIN: &str = "test-domain";

    #[cfg(feature = "integration")]
    async fn connect_test_client() -> WorkflowClient {
        WorkflowClient::connect(CADENCE_GRPC_ENDPOINT, TEST_DOMAIN, ClientOptions::default())
            .await
            .expect("should connect to local Cadence server")
    }

    #[cfg(feature = "integration")]
    fn empty_visibility_request(query: &str) -> ListWorkflowExecutionsRequest {
        ListWorkflowExecutionsRequest {
            maximum_page_size: 10,
            next_page_token: None,
            start_time_filter: None,
            execution_filter: None,
            type_filter: None,
            status_filter: None,
            query: Some(query.to_string()),
        }
    }

    #[tokio::test]
    #[cfg(feature = "integration")]
    async fn test_list_workflows_returns_executions_for_query() {
        let client = connect_test_client().await;
        let resp = client
            .list_workflows(empty_visibility_request("WorkflowType = 'nonexistent'"))
            .await
            .expect("list_workflows should succeed");
        assert!(resp.executions.is_empty());
    }

    #[tokio::test]
    #[cfg(feature = "integration")]
    async fn test_list_archived_workflows_queries_archival() {
        let client = connect_test_client().await;
        let resp = client
            .list_archived_workflows(empty_visibility_request("WorkflowType = 'nonexistent'"))
            .await
            .expect("list_archived_workflows should succeed");
        assert!(resp.executions.is_empty());
    }

    #[tokio::test]
    #[cfg(feature = "integration")]
    async fn test_scan_workflows_passes_query_and_paging() {
        let client = connect_test_client().await;
        let resp = client
            .scan_workflows(empty_visibility_request("WorkflowType = 'nonexistent'"))
            .await
            .expect("scan_workflows should succeed");
        assert!(resp.executions.is_empty());
    }

    #[tokio::test]
    #[cfg(feature = "integration")]
    async fn test_count_workflows_returns_count() {
        let client = connect_test_client().await;
        let resp = client
            .count_workflows(CountWorkflowExecutionsRequest {
                query: "WorkflowType = 'nonexistent'".to_string(),
            })
            .await
            .expect("count_workflows should succeed");
        assert_eq!(resp.count, 0);
    }

    #[tokio::test]
    #[cfg(feature = "integration")]
    async fn test_cancel_workflow_with_options_records_cause() {
        let client = connect_test_client().await;
        // Cancellation of a non-existent workflow is expected to fail; the test
        // exercises that the cause-carrying request reaches the server.
        let result = client
            .cancel_workflow_with_options(
                "nonexistent-workflow-id",
                None,
                CancelWorkflowOptions {
                    cause: Some("integration-test cancel reason".to_string()),
                },
            )
            .await;
        assert!(result.is_err());
    }
}
