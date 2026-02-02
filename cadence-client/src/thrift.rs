//! Thrift transport implementation for Cadence service.
//!
//! This module provides the actual implementation of the WorkflowService trait
//! using Thrift protocol to communicate with the Cadence server.

use cadence_core::{CadenceError, CadenceResult};
use cadence_proto::workflow_service::*;
use cadence_proto::shared::*;
use cadence_proto::generated::cadence::{TWorkflowServiceSyncClient};
use cadence_proto::generated::shared as thrift_types;
use async_trait::async_trait;
use std::time::Duration;
use thrift::protocol::{TBinaryInputProtocol, TBinaryOutputProtocol};
use thrift::transport::{
    TTcpChannel, TFramedReadTransport, TFramedWriteTransport, TIoChannel,
};

/// Thrift-based workflow service client
/// 
/// This client uses Thrift Binary Protocol over TCP to communicate with Cadence server.
/// Currently, each operation creates a new connection. Future versions will implement
/// connection pooling for better performance.
pub struct ThriftWorkflowServiceClient {
    domain: String,
    config: ClientConfig,
}

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub host: String,
    pub port: u16,
    pub timeout: Duration,
    pub max_retries: u32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 7933,
            timeout: Duration::from_secs(10),
            max_retries: 3,
        }
    }
}

impl ThriftWorkflowServiceClient {
    pub fn new(domain: impl Into<String>, config: ClientConfig) -> Self {
        Self {
            domain: domain.into(),
            config,
        }
    }

    pub async fn connect(&self) -> CadenceResult<()> {
        // Test the connection by creating a TCP connection
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let mut channel = TTcpChannel::new();
        channel
            .open(&addr)
            .map_err(|e| CadenceError::Transport(format!("Failed to connect to {}: {}", addr, e)))?;
        // Connection successful, close it - we'll create new ones for each request
        Ok(())
    }
    
    /// Create a new Thrift client connection (deprecated - use create_thrift_client_blocking)
    /// 
    /// This creates a fresh TCP connection with Thrift Binary Protocol.
    /// In the future, this should use connection pooling for better performance.
    #[allow(dead_code)]
    fn create_client(&self) -> CadenceResult<impl TWorkflowServiceSyncClient> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        
        // Create TCP connection
        let mut channel = TTcpChannel::new();
        channel
            .open(&addr)
            .map_err(|e| CadenceError::Transport(format!("Failed to connect to {}: {}", addr, e)))?;
        
        // Split channel into read and write halves
        let (i_chan, o_chan) = channel
            .split()
            .map_err(|e| CadenceError::Transport(format!("Failed to split channel: {}", e)))?;
        
        // Wrap with framed transport (Cadence uses framed Thrift)
        let i_tran = TFramedReadTransport::new(i_chan);
        let o_tran = TFramedWriteTransport::new(o_chan);
        
        // Wrap with binary protocol (Cadence uses TBinaryProtocol)
        let i_prot = TBinaryInputProtocol::new(i_tran, true);
        let o_prot = TBinaryOutputProtocol::new(o_tran, true);
        
        // Create and return the generated Thrift client
        Ok(cadence_proto::generated::cadence::WorkflowServiceSyncClient::new(i_prot, o_prot))
    }
    
    /// Convert Thrift errors to CadenceError
    fn convert_thrift_error(err: thrift::Error) -> CadenceError {
        use thrift::Error as TE;
        
        match err {
            TE::Transport(transport_err) => {
                CadenceError::Transport(format!("Transport error: {}", transport_err))
            }
            TE::Protocol(protocol_err) => {
                CadenceError::Transport(format!("Protocol error: {}", protocol_err))
            }
            TE::Application(app_err) => {
                CadenceError::Other(format!("Application error: {:?}", app_err))
            }
            TE::User(user_err) => {
                // Thrift user exceptions - these are defined in the IDL
                CadenceError::Other(format!("Service error: {}", user_err))
            }
        }
    }
    
    /// Helper to create a thrift client in a blocking context
    fn create_thrift_client_blocking(config: &ClientConfig) -> CadenceResult<cadence_proto::generated::cadence::WorkflowServiceSyncClient<TBinaryInputProtocol<TFramedReadTransport<thrift::transport::ReadHalf<TTcpChannel>>>, TBinaryOutputProtocol<TFramedWriteTransport<thrift::transport::WriteHalf<TTcpChannel>>>>> {
        let addr = format!("{}:{}", config.host, config.port);
        let mut channel = TTcpChannel::new();
        channel
            .open(&addr)
            .map_err(|e| CadenceError::Transport(format!("Failed to connect to {}: {}", addr, e)))?;
        
        let (i_chan, o_chan) = channel
            .split()
            .map_err(|e| CadenceError::Transport(format!("Failed to split channel: {}", e)))?;
        
        let i_tran = TFramedReadTransport::new(i_chan);
        let o_tran = TFramedWriteTransport::new(o_chan);
        let i_prot = TBinaryInputProtocol::new(i_tran, true);
        let o_prot = TBinaryOutputProtocol::new(o_tran, true);
        Ok(cadence_proto::generated::cadence::WorkflowServiceSyncClient::new(i_prot, o_prot))
    }
}

#[async_trait]
impl WorkflowService for ThriftWorkflowServiceClient {
    type Error = CadenceError;

    async fn start_workflow_execution(
        &self,
        request: StartWorkflowExecutionRequest,
    ) -> Result<StartWorkflowExecutionResponse, Self::Error> {
        // Use tokio::task::spawn_blocking to run sync Thrift calls
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            // Convert internal request to thrift request
            let thrift_request = thrift_types::StartWorkflowExecutionRequest {
                domain: Some(domain.clone()),
                workflow_id: Some(request.workflow_id),
                workflow_type: request.workflow_type.map(|wt| thrift_types::WorkflowType {
                    name: Some(wt.name),
                }),
                task_list: request.task_list.map(|tl| thrift_types::TaskList {
                    name: Some(tl.name),
                    kind: Some(match tl.kind {
                        TaskListKind::Normal => thrift_types::TaskListKind::NORMAL,
                        TaskListKind::Sticky => thrift_types::TaskListKind::STICKY,
                    }),
                }),
                input: request.input,
                execution_start_to_close_timeout_seconds: request.execution_start_to_close_timeout_seconds,
                task_start_to_close_timeout_seconds: request.task_start_to_close_timeout_seconds,
                identity: Some(request.identity),
                request_id: Some(request.request_id),
                workflow_id_reuse_policy: request.workflow_id_reuse_policy.map(|p| match p {
                    WorkflowIdReusePolicy::AllowDuplicate => thrift_types::WorkflowIdReusePolicy::ALLOW_DUPLICATE,
                    WorkflowIdReusePolicy::AllowDuplicateFailedOnly => thrift_types::WorkflowIdReusePolicy::ALLOW_DUPLICATE_FAILED_ONLY,
                    WorkflowIdReusePolicy::RejectDuplicate => thrift_types::WorkflowIdReusePolicy::REJECT_DUPLICATE,
                    WorkflowIdReusePolicy::TerminateIfRunning => thrift_types::WorkflowIdReusePolicy::TERMINATE_IF_RUNNING,
                }),
                retry_policy: None, // TODO: Convert RetryPolicy
                cron_schedule: request.cron_schedule,
                memo: None, // TODO: Convert Memo
                search_attributes: None, // TODO: Convert SearchAttributes
                header: None, // TODO: Convert Header
                delay_start_seconds: request.delay_start_seconds,
                jitter_start_seconds: request.jitter_start_seconds,
                first_run_at_timestamp: None,
                cron_overlap_policy: None,
                active_cluster_selection_policy: None,
            };
            
            let response = thrift_client
                .start_workflow_execution(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(StartWorkflowExecutionResponse {
                run_id: response.run_id.unwrap_or_default(),
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn signal_workflow_execution(
        &self,
        request: SignalWorkflowExecutionRequest,
    ) -> Result<SignalWorkflowExecutionResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::SignalWorkflowExecutionRequest {
                domain: Some(domain),
                workflow_execution: request.workflow_execution.map(|we| thrift_types::WorkflowExecution {
                    workflow_id: Some(we.workflow_id),
                    run_id: Some(we.run_id),
                }),
                signal_name: Some(request.signal_name),
                input: request.input,
                identity: Some(request.identity),
                request_id: Some(request.request_id),
                control: request.control.map(|s| s.into_bytes()),
            };
            
            thrift_client
                .signal_workflow_execution(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(SignalWorkflowExecutionResponse {})
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn signal_with_start_workflow_execution(
        &self,
        request: SignalWithStartWorkflowExecutionRequest,
    ) -> Result<StartWorkflowExecutionResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::SignalWithStartWorkflowExecutionRequest {
                domain: Some(domain),
                workflow_id: Some(request.workflow_id),
                workflow_type: request.workflow_type.map(|wt| thrift_types::WorkflowType {
                    name: Some(wt.name),
                }),
                task_list: request.task_list.map(|tl| thrift_types::TaskList {
                    name: Some(tl.name),
                    kind: Some(match tl.kind {
                        TaskListKind::Normal => thrift_types::TaskListKind::NORMAL,
                        TaskListKind::Sticky => thrift_types::TaskListKind::STICKY,
                    }),
                }),
                input: request.input,
                execution_start_to_close_timeout_seconds: request.execution_start_to_close_timeout_seconds,
                task_start_to_close_timeout_seconds: request.task_start_to_close_timeout_seconds,
                identity: Some(request.identity),
                request_id: Some(request.request_id),
                workflow_id_reuse_policy: request.workflow_id_reuse_policy.map(|p| match p {
                    WorkflowIdReusePolicy::AllowDuplicate => thrift_types::WorkflowIdReusePolicy::ALLOW_DUPLICATE,
                    WorkflowIdReusePolicy::AllowDuplicateFailedOnly => thrift_types::WorkflowIdReusePolicy::ALLOW_DUPLICATE_FAILED_ONLY,
                    WorkflowIdReusePolicy::RejectDuplicate => thrift_types::WorkflowIdReusePolicy::REJECT_DUPLICATE,
                    WorkflowIdReusePolicy::TerminateIfRunning => thrift_types::WorkflowIdReusePolicy::TERMINATE_IF_RUNNING,
                }),
                signal_name: Some(request.signal_name),
                signal_input: request.signal_input,
                control: None,
                retry_policy: request.retry_policy.map(|rp| Box::new(thrift_types::RetryPolicy {
                    initial_interval_in_seconds: Some(rp.initial_interval_in_seconds),
                    backoff_coefficient: Some(thrift::OrderedFloat(rp.backoff_coefficient)),
                    maximum_interval_in_seconds: Some(rp.maximum_interval_in_seconds),
                    maximum_attempts: Some(rp.maximum_attempts),
                    expiration_interval_in_seconds: Some(rp.expiration_interval_in_seconds),
                    non_retriable_error_reasons: Some(rp.non_retryable_error_types),
                })),
                cron_schedule: request.cron_schedule,
                memo: request.memo.map(|m| thrift_types::Memo {
                    fields: Some(m.fields.into_iter().collect()),
                }),
                search_attributes: request.search_attributes.map(|sa| thrift_types::SearchAttributes {
                    indexed_fields: Some(sa.indexed_fields.into_iter().collect()),
                }),
                header: request.header.map(|h| thrift_types::Header {
                    fields: Some(h.fields.into_iter().collect()),
                }),
                delay_start_seconds: None,
                jitter_start_seconds: None,
                first_run_at_timestamp: None,
                cron_overlap_policy: None,
                active_cluster_selection_policy: None,
            };
            
            let response = thrift_client
                .signal_with_start_workflow_execution(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(StartWorkflowExecutionResponse {
                run_id: response.run_id.unwrap_or_default(),
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn request_cancel_workflow_execution(
        &self,
        request: RequestCancelWorkflowExecutionRequest,
    ) -> Result<RequestCancelWorkflowExecutionResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::RequestCancelWorkflowExecutionRequest {
                domain: Some(domain),
                workflow_execution: request.workflow_execution.map(|we| thrift_types::WorkflowExecution {
                    workflow_id: Some(we.workflow_id),
                    run_id: Some(we.run_id),
                }),
                identity: Some(request.identity),
                request_id: Some(request.request_id),
                cause: request.cause,
                first_execution_run_i_d: request.first_execution_run_id,
            };
            
            thrift_client
                .request_cancel_workflow_execution(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(RequestCancelWorkflowExecutionResponse {})
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn terminate_workflow_execution(
        &self,
        request: TerminateWorkflowExecutionRequest,
    ) -> Result<TerminateWorkflowExecutionResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::TerminateWorkflowExecutionRequest {
                domain: Some(domain),
                workflow_execution: request.workflow_execution.map(|we| thrift_types::WorkflowExecution {
                    workflow_id: Some(we.workflow_id),
                    run_id: Some(we.run_id),
                }),
                reason: request.reason,
                details: request.details,
                identity: Some(request.identity),
                first_execution_run_i_d: request.first_execution_run_id,
            };
            
            thrift_client
                .terminate_workflow_execution(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(TerminateWorkflowExecutionResponse {})
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn query_workflow(
        &self,
        _request: QueryWorkflowRequest,
    ) -> Result<QueryWorkflowResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(QueryWorkflowResponse {
            query_result: None,
            query_rejected: None,
        })
    }

    async fn poll_for_decision_task(
        &self,
        _request: PollForDecisionTaskRequest,
    ) -> Result<PollForDecisionTaskResponse, Self::Error> {
        // TODO: Implement Thrift call
        // This is a blocking call that waits for tasks
        Ok(PollForDecisionTaskResponse {
            task_token: vec![],
            workflow_execution: None,
            workflow_type: None,
            previous_started_event_id: 0,
            started_event_id: 0,
            attempt: 0,
            backlog_count_hint: 0,
            history: None,
            next_page_token: None,
            query: None,
            workflow_execution_task_list: None,
            scheduled_timestamp: None,
            started_timestamp: None,
            queries: None,
        })
    }

    async fn respond_decision_task_completed(
        &self,
        _request: RespondDecisionTaskCompletedRequest,
    ) -> Result<RespondDecisionTaskCompletedResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(RespondDecisionTaskCompletedResponse {
            decision_task: None,
        })
    }

    async fn poll_for_activity_task(
        &self,
        _request: PollForActivityTaskRequest,
    ) -> Result<PollForActivityTaskResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(PollForActivityTaskResponse {
            task_token: vec![],
            workflow_execution: None,
            activity_id: String::new(),
            activity_type: None,
            input: None,
            scheduled_timestamp: None,
            started_timestamp: None,
            schedule_to_close_timeout_seconds: None,
            start_to_close_timeout_seconds: None,
            heartbeat_timeout_seconds: None,
            attempt: 0,
            scheduled_timestamp_of_this_attempt: None,
            heartbeat_details: None,
            workflow_type: None,
            workflow_domain: None,
            header: None,
        })
    }

    async fn record_activity_task_heartbeat(
        &self,
        _request: RecordActivityTaskHeartbeatRequest,
    ) -> Result<RecordActivityTaskHeartbeatResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(RecordActivityTaskHeartbeatResponse {
            cancel_requested: false,
        })
    }

    async fn respond_activity_task_completed(
        &self,
        _request: RespondActivityTaskCompletedRequest,
    ) -> Result<RespondActivityTaskCompletedResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(RespondActivityTaskCompletedResponse {})
    }

    async fn respond_activity_task_failed(
        &self,
        _request: RespondActivityTaskFailedRequest,
    ) -> Result<RespondActivityTaskFailedResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(RespondActivityTaskFailedResponse {})
    }

    async fn get_workflow_execution_history(
        &self,
        _request: GetWorkflowExecutionHistoryRequest,
    ) -> Result<GetWorkflowExecutionHistoryResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(GetWorkflowExecutionHistoryResponse {
            history: Some(History { events: vec![] }),
            next_page_token: None,
            archived: false,
        })
    }

    async fn describe_workflow_execution(
        &self,
        _request: DescribeWorkflowExecutionRequest,
    ) -> Result<DescribeWorkflowExecutionResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(DescribeWorkflowExecutionResponse {
            execution_configuration: None,
            workflow_execution_info: None,
            pending_children: vec![],
            pending_decision: None,
            pending_activities: vec![],
        })
    }

    async fn list_open_workflow_executions(
        &self,
        _request: ListOpenWorkflowExecutionsRequest,
    ) -> Result<ListOpenWorkflowExecutionsResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(ListOpenWorkflowExecutionsResponse {
            executions: vec![],
            next_page_token: None,
        })
    }

    async fn list_closed_workflow_executions(
        &self,
        _request: ListClosedWorkflowExecutionsRequest,
    ) -> Result<ListClosedWorkflowExecutionsResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(ListClosedWorkflowExecutionsResponse {
            executions: vec![],
            next_page_token: None,
        })
    }

    async fn register_domain(
        &self,
        request: RegisterDomainRequest,
    ) -> Result<(), Self::Error> {
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            // Convert internal request to thrift request
            let thrift_request = thrift_types::RegisterDomainRequest {
                name: Some(request.name),
                description: request.description,
                owner_email: Some(request.owner_email),
                workflow_execution_retention_period_in_days: Some(request.workflow_execution_retention_period_in_days),
                emit_metric: request.emit_metric,
                clusters: Some(request.clusters.into_iter().map(|c| thrift_types::ClusterReplicationConfiguration {
                    cluster_name: Some(c.cluster_name),
                }).collect()),
                active_cluster_name: Some(request.active_cluster_name),
                active_clusters_by_region: None,
                active_clusters: None,
                data: Some(request.data.into_iter().collect()),
                security_token: request.security_token,
                is_global_domain: request.is_global_domain,
                history_archival_status: request.history_archival_status.map(|s| match s {
                    ArchivalStatus::Disabled => thrift_types::ArchivalStatus::DISABLED,
                    ArchivalStatus::Enabled => thrift_types::ArchivalStatus::ENABLED,
                }),
                history_archival_u_r_i: request.history_archival_uri,
                visibility_archival_status: request.visibility_archival_status.map(|s| match s {
                    ArchivalStatus::Disabled => thrift_types::ArchivalStatus::DISABLED,
                    ArchivalStatus::Enabled => thrift_types::ArchivalStatus::ENABLED,
                }),
                visibility_archival_u_r_i: request.visibility_archival_uri,
            };
            
            thrift_client
                .register_domain(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(())
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn describe_domain(
        &self,
        _request: DescribeDomainRequest,
    ) -> Result<DescribeDomainResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(DescribeDomainResponse {
            domain_info: None,
            configuration: None,
            replication_configuration: None,
        })
    }

    async fn update_domain(
        &self,
        _request: UpdateDomainRequest,
    ) -> Result<UpdateDomainResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(UpdateDomainResponse {
            domain_info: None,
            configuration: None,
            replication_configuration: None,
        })
    }

    async fn failover_domain(
        &self,
        _request: FailoverDomainRequest,
    ) -> Result<FailoverDomainResponse, Self::Error> {
        // TODO: Implement Thrift call
        Ok(FailoverDomainResponse {})
    }
}
