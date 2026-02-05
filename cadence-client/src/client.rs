//! Client implementation for Cadence workflow orchestration service.
//!
//! This module provides the main client interface for starting workflows,
//! querying workflow state, sending signals, and managing workflow executions.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use cadence_core::{
    CadenceError, CadenceResult, EncodedValue, QueryConsistencyLevel, RetryPolicy,
    WorkflowExecution, WorkflowIdReusePolicy,
};
use cadence_proto::shared::*;
use cadence_proto::workflow_service::*;
use cadence_proto::{
    PendingActivityInfo as ProtoPendingActivityInfo,
    PendingActivityState as ProtoPendingActivityState,
    PendingChildExecutionInfo as ProtoPendingChildExecutionInfo,
    WorkflowExecutionConfiguration as ProtoWorkflowExecutionConfiguration,
    WorkflowExecutionInfo as ProtoWorkflowExecutionInfo,
};
use serde_json;
use uuid::Uuid;

use crate::auth::BoxedAuthProvider;

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
        }
    }
}

/// Async workflow execution (returned by async start)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowExecutionAsync {
    pub workflow_id: String,
}

/// Workflow run handle for retrieving results
#[async_trait]
pub trait WorkflowRun: Send + Sync {
    /// Get the run ID
    fn run_id(&self) -> &str;
    /// Get the workflow execution result (blocking)
    async fn get(&self) -> CadenceResult<Option<Vec<u8>>>;
    /// Get with timeout
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
                .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
    pub(crate) service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync>,
    pub(crate) domain: String,
    pub(crate) options: ClientOptions,
}

impl WorkflowClient {
    /// Create a new WorkflowClient from an existing service
    pub fn new(
        service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync>,
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
    ) -> CadenceResult<Self> {
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
    /// Data converter for serialization
    pub data_converter: Arc<dyn DataConverter>,
}

impl std::fmt::Debug for ClientOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientOptions")
            .field("identity", &self.identity)
            .field("metrics_scope", &self.metrics_scope.is_some())
            .field("logger", &self.logger.is_some())
            .field("feature_flags", &self.feature_flags)
            .field("data_converter", &"<dyn DataConverter>")
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
            data_converter: Arc::new(JsonDataConverter),
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
        let request = StartWorkflowExecutionRequest {
            domain: self.domain.clone(),
            workflow_id: options.id.clone(),
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
            jitter_start_seconds: None,
            first_execution_run_id: None,
            first_decision_task_backoff_seconds: None,
            partition_config: None,
        };

        let _response = self
            .service
            .start_workflow_execution(request)
            .await
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

        Ok(WorkflowExecutionAsync {
            workflow_id: options.id,
        })
    }

    async fn execute_workflow(
        &self,
        options: StartWorkflowOptions,
        workflow_type: &str,
        args: Option<&[u8]>,
    ) -> CadenceResult<Box<dyn WorkflowRun>> {
        let request_id = Uuid::new_v4().to_string();
        let workflow_id = options.id.clone();

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
            jitter_start_seconds: None,
            first_execution_run_id: None,
            first_decision_task_backoff_seconds: None,
            partition_config: None,
        };

        let response = self
            .service
            .start_workflow_execution(request)
            .await
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;
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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
        let request = RequestCancelWorkflowExecutionRequest {
            domain: self.domain.clone(),
            workflow_execution: Some(make_proto_execution(
                workflow_id.to_string(),
                run_id.unwrap_or_default().to_string(),
            )),
            identity: self.options.identity.clone(),
            request_id: Uuid::new_v4().to_string(),
            cause: reason.map(|s| s.to_string()),
            first_execution_run_id: None,
        };
        self.service
            .request_cancel_workflow_execution(request)
            .await
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;
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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;
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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
                .map_err(|e| CadenceError::ClientError(e.to_string()))?;
        } else {
            let request = RespondActivityTaskCompletedRequest {
                task_token: task_token.to_vec(),
                result: result.map(|r| r.to_vec()),
                identity: self.options.identity.clone(),
            };
            self.service
                .respond_activity_task_completed(request)
                .await
                .map_err(|e| CadenceError::ClientError(e.to_string()))?;
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
                .map_err(|e| CadenceError::ClientError(e.to_string()))?;
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
                .map_err(|e| CadenceError::ClientError(e.to_string()))?;
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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;
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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;
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
                cadence_proto::workflow_service::StartTimeFilter {
                    earliest_time: Some(f.earliest_time.timestamp_nanos_opt().unwrap_or(0)),
                    latest_time: Some(f.latest_time.timestamp_nanos_opt().unwrap_or(0)),
                }
            }),
            execution_filter: request.execution_filter.map(|f| {
                cadence_proto::workflow_service::WorkflowExecutionFilter {
                    workflow_id: f.workflow_id,
                }
            }),
            type_filter: request
                .type_filter
                .map(|f| cadence_proto::workflow_service::WorkflowTypeFilter { name: f.name }),
            status_filter: request.status_filter,
        };

        let response = self
            .service
            .list_closed_workflow_executions(req)
            .await
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
                cadence_proto::workflow_service::StartTimeFilter {
                    earliest_time: Some(f.earliest_time.timestamp_nanos_opt().unwrap_or(0)),
                    latest_time: Some(f.latest_time.timestamp_nanos_opt().unwrap_or(0)),
                }
            }),
            execution_filter: request.execution_filter.map(|f| {
                cadence_proto::workflow_service::WorkflowExecutionFilter {
                    workflow_id: f.workflow_id,
                }
            }),
            type_filter: request
                .type_filter
                .map(|f| cadence_proto::workflow_service::WorkflowTypeFilter { name: f.name }),
            status_filter: request.status_filter,
        };

        let response = self
            .service
            .list_open_workflow_executions(req)
            .await
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
        // Usually maps to ListWorkflowExecutions (visibility) but we don't have it in trait.
        // Assuming this is just a wrapper for either open or closed, or visibility (which seems missing in trait).
        // I will default to list_open_workflows for now as a placeholder
        self.list_open_workflows(request).await
    }

    async fn scan_workflows(
        &self,
        request: ListWorkflowExecutionsRequest,
    ) -> CadenceResult<ListWorkflowExecutionsResponse> {
        let req = ScanWorkflowExecutionsRequest {
            domain: self.domain.clone(),
            page_size: request.maximum_page_size,
            next_page_token: request.next_page_token,
            query: None, // ListWorkflowExecutionsRequest doesn't have a query field
        };

        let response = self
            .service
            .scan_workflow_executions(req)
            .await
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
        let req = cadence_proto::workflow_service::CountWorkflowExecutionsRequest {
            domain: self.domain.clone(),
            query: Some(request.query),
        };

        let response = self
            .service
            .count_workflow_executions(req)
            .await
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
        let request = QueryWorkflowRequest {
            domain: self.domain.clone(),
            execution: Some(make_proto_execution(
                workflow_id.to_string(),
                run_id.unwrap_or_default().to_string(),
            )),
            query: Some(WorkflowQuery {
                query_type: query_type.to_string(),
                query_args: args.map(|a| a.to_vec()),
            }),
            query_consistency_level: None,
            query_reject_condition: None,
        };

        let response = self
            .service
            .query_workflow(request)
            .await
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

        // TODO: Handle query_rejected
        if let Some(rejected) = response.query_rejected {
            return Err(CadenceError::ClientError(format!(
                "Query rejected: {:?}",
                rejected
            )));
        }

        if let Some(result) = response.query_result {
            // Assuming the result is JSON encoded value
            Ok(EncodedValue::new(result))
        } else {
            Err(CadenceError::ClientError(
                "Query returned no result".to_string(),
            ))
        }
    }

    async fn query_workflow_with_options(
        &self,
        request: QueryWorkflowWithOptionsRequest,
    ) -> CadenceResult<QueryWorkflowWithOptionsResponse> {
        // query_workflow_with_options not in trait
        // Just use query_workflow
        let val = self
            .query_workflow(
                &request.workflow_id,
                request.run_id.as_deref(),
                &request.query_type,
                request.query_args.as_deref(),
            )
            .await?;
        Ok(QueryWorkflowWithOptionsResponse {
            query_result: Some(val.as_bytes().to_vec()),
            query_rejected: None,
        })
    }

    async fn reset_workflow(
        &self,
        request: ResetWorkflowExecutionRequest,
    ) -> CadenceResult<ResetWorkflowExecutionResponse> {
        let req = cadence_proto::workflow_service::ResetWorkflowExecutionRequest {
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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
        let request = cadence_proto::workflow_service::DescribeTaskListRequest {
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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
            .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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

/// DataConverter trait for serializing/deserializing workflow and activity arguments
pub trait DataConverter: Send + Sync {
    /// Serialize a value to bytes
    fn to_data(&self, value: &dyn std::any::Any) -> CadenceResult<Vec<u8>>;
    /// Deserialize bytes into a target value
    #[expect(clippy::wrong_self_convention)]
    fn from_data(&self, data: &[u8], target: &mut dyn std::any::Any) -> CadenceResult<()>;
}

/// JSON data converter implementation
pub struct JsonDataConverter;

impl DataConverter for JsonDataConverter {
    fn to_data(&self, value: &dyn std::any::Any) -> CadenceResult<Vec<u8>> {
        // Try to downcast to common serde-serializable types
        // For most use cases, users should pass serializable types

        // Handle String
        if let Some(s) = value.downcast_ref::<String>() {
            return serde_json::to_vec(s).map_err(|e| CadenceError::Serialization(e.to_string()));
        }

        // Handle &str (this won't work directly with Any, but document it)
        if let Some(s) = value.downcast_ref::<&str>() {
            return serde_json::to_vec(s).map_err(|e| CadenceError::Serialization(e.to_string()));
        }

        // Handle serde_json::Value directly
        if let Some(v) = value.downcast_ref::<serde_json::Value>() {
            return serde_json::to_vec(v).map_err(|e| CadenceError::Serialization(e.to_string()));
        }

        // Handle Vec<u8> - pass through as-is
        if let Some(bytes) = value.downcast_ref::<Vec<u8>>() {
            return Ok(bytes.clone());
        }

        // For other types, we can't serialize without knowing the concrete type
        // The user should either:
        // 1. Serialize to serde_json::Value first
        // 2. Serialize to Vec<u8> first
        // 3. Use the generic encode function from cadence-core
        Err(CadenceError::Serialization(
            "Cannot serialize type - use serde_json::Value or Vec<u8> for dynamic types"
                .to_string(),
        ))
    }

    fn from_data(&self, data: &[u8], target: &mut dyn std::any::Any) -> CadenceResult<()> {
        // Try to downcast target to common deserializable types

        // Handle String
        if let Some(s) = target.downcast_mut::<String>() {
            *s = serde_json::from_slice(data)
                .map_err(|e| CadenceError::Serialization(e.to_string()))?;
            return Ok(());
        }

        // Handle serde_json::Value
        if let Some(v) = target.downcast_mut::<serde_json::Value>() {
            *v = serde_json::from_slice(data)
                .map_err(|e| CadenceError::Serialization(e.to_string()))?;
            return Ok(());
        }

        // Handle Vec<u8> - copy data as-is
        if let Some(bytes) = target.downcast_mut::<Vec<u8>>() {
            *bytes = data.to_vec();
            return Ok(());
        }

        // For other types, we can't deserialize without knowing the concrete type
        Err(CadenceError::Serialization(
            "Cannot deserialize type - use serde_json::Value or Vec<u8> for dynamic types"
                .to_string(),
        ))
    }
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
                .map_err(|e| CadenceError::ClientError(e.to_string()))?;

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
        // TODO: Implement timeout
        tokio::time::timeout(timeout, self.get())
            .await
            .map_err(|_| {
                CadenceError::ClientError("Timeout waiting for workflow result".to_string())
            })?
    }
}

// Helpers
fn convert_retry_policy(policy: RetryPolicy) -> cadence_proto::shared::RetryPolicy {
    cadence_proto::shared::RetryPolicy {
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
) -> cadence_proto::shared::WorkflowIdReusePolicy {
    match policy {
        WorkflowIdReusePolicy::AllowDuplicateFailedOnly => {
            cadence_proto::shared::WorkflowIdReusePolicy::AllowDuplicateFailedOnly
        }
        WorkflowIdReusePolicy::AllowDuplicate => {
            cadence_proto::shared::WorkflowIdReusePolicy::AllowDuplicate
        }
        WorkflowIdReusePolicy::RejectDuplicate => {
            cadence_proto::shared::WorkflowIdReusePolicy::RejectDuplicate
        }
        WorkflowIdReusePolicy::TerminateIfRunning => {
            cadence_proto::shared::WorkflowIdReusePolicy::TerminateIfRunning
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
) -> cadence_proto::shared::WorkflowExecution {
    cadence_proto::shared::WorkflowExecution {
        workflow_id,
        run_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_data_converter_string() {
        let converter = JsonDataConverter;
        let original = "Hello, Cadence!".to_string();

        // Serialize
        let data = converter.to_data(&original).unwrap();

        // Deserialize
        let mut target = String::new();
        converter.from_data(&data, &mut target).unwrap();

        assert_eq!(original, target);
    }

    #[test]
    fn test_json_data_converter_json_value() {
        let converter = JsonDataConverter;
        let original = serde_json::json!({
            "name": "test",
            "value": 42,
            "nested": {
                "flag": true
            }
        });

        // Serialize
        let data = converter.to_data(&original).unwrap();

        // Deserialize
        let mut target = serde_json::Value::Null;
        converter.from_data(&data, &mut target).unwrap();

        assert_eq!(original, target);
    }

    #[test]
    fn test_json_data_converter_bytes() {
        let converter = JsonDataConverter;
        let original = vec![1u8, 2, 3, 4, 5];

        // Serialize (pass through)
        let data = converter.to_data(&original).unwrap();

        // Deserialize (pass through)
        let mut target = Vec::<u8>::new();
        converter.from_data(&data, &mut target).unwrap();

        assert_eq!(original, target);
    }

    #[test]
    fn test_json_data_converter_unsupported_type() {
        let converter = JsonDataConverter;

        // Try to serialize an unsupported type (i32)
        let value = 42i32;
        let result = converter.to_data(&value);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CadenceError::Serialization(_)
        ));
    }
}
