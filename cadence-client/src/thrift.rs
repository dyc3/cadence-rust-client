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
        request: QueryWorkflowRequest,
    ) -> Result<QueryWorkflowResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::QueryWorkflowRequest {
                domain: Some(domain),
                execution: request.execution.map(|we| thrift_types::WorkflowExecution {
                    workflow_id: Some(we.workflow_id),
                    run_id: Some(we.run_id),
                }),
                query: request.query.map(|q| Box::new(thrift_types::WorkflowQuery {
                    query_type: Some(q.query_type),
                    query_args: q.query_args,
                })),
                query_reject_condition: None,
                query_consistency_level: None,
            };
            
            let response = thrift_client
                .query_workflow(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(QueryWorkflowResponse {
                query_result: response.query_result,
                query_rejected: response.query_rejected.map(|qr| QueryRejected {
                    close_status: qr.close_status.and_then(|cs| match cs {
                        thrift_types::WorkflowExecutionCloseStatus::COMPLETED => Some(WorkflowExecutionCloseStatus::Completed),
                        thrift_types::WorkflowExecutionCloseStatus::FAILED => Some(WorkflowExecutionCloseStatus::Failed),
                        thrift_types::WorkflowExecutionCloseStatus::CANCELED => Some(WorkflowExecutionCloseStatus::Canceled),
                        thrift_types::WorkflowExecutionCloseStatus::TERMINATED => Some(WorkflowExecutionCloseStatus::Terminated),
                        thrift_types::WorkflowExecutionCloseStatus::CONTINUED_AS_NEW => Some(WorkflowExecutionCloseStatus::ContinuedAsNew),
                        thrift_types::WorkflowExecutionCloseStatus::TIMED_OUT => Some(WorkflowExecutionCloseStatus::TimedOut),
                        _ => None,
                    }),
                }),
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn poll_for_decision_task(
        &self,
        request: PollForDecisionTaskRequest,
    ) -> Result<PollForDecisionTaskResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::PollForDecisionTaskRequest {
                domain: Some(domain),
                task_list: request.task_list.map(|tl| thrift_types::TaskList {
                    name: Some(tl.name),
                    kind: Some(match tl.kind {
                        TaskListKind::Normal => thrift_types::TaskListKind::NORMAL,
                        TaskListKind::Sticky => thrift_types::TaskListKind::STICKY,
                    }),
                }),
                identity: Some(request.identity),
                binary_checksum: Some(request.binary_checksum),
            };
            
            let response = thrift_client
                .poll_for_decision_task(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(PollForDecisionTaskResponse {
                task_token: response.task_token.unwrap_or_default(),
                workflow_execution: response.workflow_execution.and_then(|we| {
                    Some(WorkflowExecution {
                        workflow_id: we.workflow_id?,
                        run_id: we.run_id?,
                    })
                }),
                workflow_type: response.workflow_type.and_then(|wt| {
                    Some(WorkflowType {
                        name: wt.name?,
                    })
                }),
                previous_started_event_id: response.previous_started_event_id.unwrap_or(0),
                started_event_id: response.started_event_id.unwrap_or(0),
                attempt: response.attempt.unwrap_or(0) as i32,
                backlog_count_hint: response.backlog_count_hint.unwrap_or(0),
                history: response.history.map(|h| History {
                    events: h.events.unwrap_or_default().into_iter().filter_map(|_e| {
                        // TODO: Convert HistoryEvent - this is complex, skip for now
                        None
                    }).collect(),
                }),
                next_page_token: response.next_page_token,
                query: response.query.and_then(|q| {
                    Some(WorkflowQuery {
                        query_type: q.query_type?,
                        query_args: q.query_args,
                    })
                }),
                workflow_execution_task_list: response.workflow_execution_task_list.and_then(|tl| {
                    Some(TaskList {
                        name: tl.name?,
                        kind: match tl.kind? {
                            thrift_types::TaskListKind::NORMAL => TaskListKind::Normal,
                            thrift_types::TaskListKind::STICKY => TaskListKind::Sticky,
                            _ => TaskListKind::Normal,
                        },
                    })
                }),
                scheduled_timestamp: response.scheduled_timestamp,
                started_timestamp: response.started_timestamp,
                queries: response.queries.map(|qs| {
                    qs.into_iter().filter_map(|(k, v)| {
                        Some((k, WorkflowQuery {
                            query_type: v.query_type?,
                            query_args: v.query_args,
                        }))
                    }).collect()
                }),
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn respond_decision_task_completed(
        &self,
        request: RespondDecisionTaskCompletedRequest,
    ) -> Result<RespondDecisionTaskCompletedResponse, Self::Error> {
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::RespondDecisionTaskCompletedRequest {
                task_token: Some(request.task_token),
                decisions: Some(request.decisions.into_iter().map(|_d| {
                    // TODO: Convert Decision types - this is complex, use empty for now
                    thrift_types::Decision::default()
                }).collect()),
                execution_context: request.execution_context,
                identity: Some(request.identity),
                sticky_attributes: request.sticky_attributes.map(|sa| thrift_types::StickyExecutionAttributes {
                    worker_task_list: sa.worker_task_list.map(|tl| thrift_types::TaskList {
                        name: Some(tl.name),
                        kind: Some(match tl.kind {
                            TaskListKind::Normal => thrift_types::TaskListKind::NORMAL,
                            TaskListKind::Sticky => thrift_types::TaskListKind::STICKY,
                        }),
                    }),
                    schedule_to_start_timeout_seconds: Some(sa.schedule_to_start_timeout_seconds),
                }),
                return_new_decision_task: Some(request.return_new_decision_task),
                force_create_new_decision_task: Some(request.force_create_new_decision_task),
                binary_checksum: Some(request.binary_checksum),
                query_results: request.query_results.map(|qrs| {
                    qrs.into_iter().map(|(k, v)| {
                        (k, Box::new(thrift_types::WorkflowQueryResult {
                            result_type: Some(match v.query_result_type {
                                QueryResultType::Answered => thrift_types::QueryResultType::ANSWERED,
                                QueryResultType::Failed => thrift_types::QueryResultType::FAILED,
                            }),
                            answer: v.answer,
                            error_message: v.error_message,
                        }))
                    }).collect()
                }),
            };
            
            let response = thrift_client
                .respond_decision_task_completed(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            // Convert response back - simplified for now
            Ok(RespondDecisionTaskCompletedResponse {
                decision_task: response.decision_task.map(|_dt| {
                    // TODO: Full conversion - using empty response for now
                    PollForDecisionTaskResponse {
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
                    }
                }),
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn poll_for_activity_task(
        &self,
        request: PollForActivityTaskRequest,
    ) -> Result<PollForActivityTaskResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::PollForActivityTaskRequest {
                domain: Some(domain),
                task_list: request.task_list.map(|tl| thrift_types::TaskList {
                    name: Some(tl.name),
                    kind: Some(match tl.kind {
                        TaskListKind::Normal => thrift_types::TaskListKind::NORMAL,
                        TaskListKind::Sticky => thrift_types::TaskListKind::STICKY,
                    }),
                }),
                identity: Some(request.identity),
                task_list_metadata: request.task_list_metadata.map(|tlm| thrift_types::TaskListMetadata {
                    max_tasks_per_second: tlm.max_tasks_per_second.map(thrift::OrderedFloat),
                }),
            };
            
            let response = thrift_client
                .poll_for_activity_task(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(PollForActivityTaskResponse {
                task_token: response.task_token.unwrap_or_default(),
                workflow_execution: response.workflow_execution.and_then(|we| {
                    Some(WorkflowExecution {
                        workflow_id: we.workflow_id?,
                        run_id: we.run_id?,
                    })
                }),
                activity_id: response.activity_id.unwrap_or_default(),
                activity_type: response.activity_type.and_then(|at| {
                    Some(ActivityType {
                        name: at.name?,
                    })
                }),
                input: response.input,
                scheduled_timestamp: response.scheduled_timestamp,
                started_timestamp: response.started_timestamp,
                schedule_to_close_timeout_seconds: response.schedule_to_close_timeout_seconds,
                start_to_close_timeout_seconds: response.start_to_close_timeout_seconds,
                heartbeat_timeout_seconds: response.heartbeat_timeout_seconds,
                attempt: response.attempt.unwrap_or(0),
                scheduled_timestamp_of_this_attempt: response.scheduled_timestamp_of_this_attempt,
                heartbeat_details: response.heartbeat_details,
                workflow_type: response.workflow_type.and_then(|wt| {
                    Some(WorkflowType {
                        name: wt.name?,
                    })
                }),
                workflow_domain: response.workflow_domain,
                header: response.header.map(|h| Header {
                    fields: h.fields.unwrap_or_default().into_iter().collect(),
                }),
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn record_activity_task_heartbeat(
        &self,
        request: RecordActivityTaskHeartbeatRequest,
    ) -> Result<RecordActivityTaskHeartbeatResponse, Self::Error> {
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::RecordActivityTaskHeartbeatRequest {
                task_token: Some(request.task_token),
                details: request.details,
                identity: Some(request.identity),
            };
            
            let response = thrift_client
                .record_activity_task_heartbeat(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(RecordActivityTaskHeartbeatResponse {
                cancel_requested: response.cancel_requested.unwrap_or(false),
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn respond_activity_task_completed(
        &self,
        request: RespondActivityTaskCompletedRequest,
    ) -> Result<RespondActivityTaskCompletedResponse, Self::Error> {
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::RespondActivityTaskCompletedRequest {
                task_token: Some(request.task_token),
                result: request.result,
                identity: Some(request.identity),
            };
            
            thrift_client
                .respond_activity_task_completed(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(RespondActivityTaskCompletedResponse {})
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn respond_activity_task_failed(
        &self,
        request: RespondActivityTaskFailedRequest,
    ) -> Result<RespondActivityTaskFailedResponse, Self::Error> {
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::RespondActivityTaskFailedRequest {
                task_token: Some(request.task_token),
                reason: request.reason,
                details: request.details,
                identity: Some(request.identity),
            };
            
            thrift_client
                .respond_activity_task_failed(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(RespondActivityTaskFailedResponse {})
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn get_workflow_execution_history(
        &self,
        request: GetWorkflowExecutionHistoryRequest,
    ) -> Result<GetWorkflowExecutionHistoryResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::GetWorkflowExecutionHistoryRequest {
                domain: Some(domain),
                execution: request.execution.map(|we| thrift_types::WorkflowExecution {
                    workflow_id: Some(we.workflow_id),
                    run_id: Some(we.run_id),
                }),
                maximum_page_size: Some(request.page_size),
                next_page_token: request.next_page_token,
                wait_for_new_event: Some(request.wait_for_new_event),
                history_event_filter_type: request.history_event_filter_type.map(|ft| match ft {
                    HistoryEventFilterType::AllEvent => thrift_types::HistoryEventFilterType::ALL_EVENT,
                    HistoryEventFilterType::CloseEvent => thrift_types::HistoryEventFilterType::CLOSE_EVENT,
                }),
                skip_archival: Some(request.skip_archival),
                query_consistency_level: None,
            };
            
            let response = thrift_client
                .get_workflow_execution_history(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(GetWorkflowExecutionHistoryResponse {
                history: response.history.map(|h| History {
                    events: h.events.unwrap_or_default().into_iter().filter_map(|_e| {
                        // TODO: Convert HistoryEvent - complex type, skip for now
                        None
                    }).collect(),
                }),
                next_page_token: response.next_page_token,
                archived: response.archived.unwrap_or(false),
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn describe_workflow_execution(
        &self,
        request: DescribeWorkflowExecutionRequest,
    ) -> Result<DescribeWorkflowExecutionResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::DescribeWorkflowExecutionRequest {
                domain: Some(domain),
                execution: request.execution.map(|we| thrift_types::WorkflowExecution {
                    workflow_id: Some(we.workflow_id),
                    run_id: Some(we.run_id),
                }),
                query_consistency_level: None,
            };
            
            let response = thrift_client
                .describe_workflow_execution(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(DescribeWorkflowExecutionResponse {
                execution_configuration: response.execution_configuration.and_then(|ec| {
                    Some(WorkflowExecutionConfiguration {
                        task_list: ec.task_list.and_then(|tl| {
                            Some(TaskList {
                                name: tl.name?,
                                kind: match tl.kind? {
                                    thrift_types::TaskListKind::NORMAL => TaskListKind::Normal,
                                    thrift_types::TaskListKind::STICKY => TaskListKind::Sticky,
                                    _ => TaskListKind::Normal,
                                },
                            })
                        }),
                        execution_start_to_close_timeout_seconds: ec.task_start_to_close_timeout_seconds?,
                        task_start_to_close_timeout_seconds: ec.task_start_to_close_timeout_seconds?,
                    })
                }),
                workflow_execution_info: None, // TODO: Complex conversion
                pending_children: vec![],
                pending_decision: None,
                pending_activities: vec![],
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn list_open_workflow_executions(
        &self,
        request: ListOpenWorkflowExecutionsRequest,
    ) -> Result<ListOpenWorkflowExecutionsResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::ListOpenWorkflowExecutionsRequest {
                domain: Some(domain),
                maximum_page_size: Some(request.maximum_page_size),
                next_page_token: request.next_page_token,
                start_time_filter: request.start_time_filter.map(|stf| thrift_types::StartTimeFilter {
                    earliest_time: stf.earliest_time,
                    latest_time: stf.latest_time,
                }),
                execution_filter: request.execution_filter.map(|ef| thrift_types::WorkflowExecutionFilter {
                    workflow_id: Some(ef.workflow_id),
                    run_id: None,
                }),
                type_filter: request.type_filter.map(|tf| thrift_types::WorkflowTypeFilter {
                    name: Some(tf.name),
                }),
            };
            
            let response = thrift_client
                .list_open_workflow_executions(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(ListOpenWorkflowExecutionsResponse {
                executions: vec![], // TODO: Convert WorkflowExecutionInfo
                next_page_token: response.next_page_token,
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn list_closed_workflow_executions(
        &self,
        request: ListClosedWorkflowExecutionsRequest,
    ) -> Result<ListClosedWorkflowExecutionsResponse, Self::Error> {
        let domain = self.domain.clone();
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::ListClosedWorkflowExecutionsRequest {
                domain: Some(domain),
                maximum_page_size: Some(request.maximum_page_size),
                next_page_token: request.next_page_token,
                start_time_filter: request.start_time_filter.map(|stf| thrift_types::StartTimeFilter {
                    earliest_time: stf.earliest_time,
                    latest_time: stf.latest_time,
                }),
                execution_filter: request.execution_filter.map(|ef| thrift_types::WorkflowExecutionFilter {
                    workflow_id: Some(ef.workflow_id),
                    run_id: None,
                }),
                type_filter: request.type_filter.map(|tf| thrift_types::WorkflowTypeFilter {
                    name: Some(tf.name),
                }),
                status_filter: request.status_filter.map(|_sf| thrift_types::WorkflowExecutionCloseStatus::COMPLETED), // TODO: proper conversion
            };
            
            let response = thrift_client
                .list_closed_workflow_executions(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(ListClosedWorkflowExecutionsResponse {
                executions: vec![], // TODO: Convert WorkflowExecutionInfo
                next_page_token: response.next_page_token,
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
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
        request: DescribeDomainRequest,
    ) -> Result<DescribeDomainResponse, Self::Error> {
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::DescribeDomainRequest {
                name: request.name,
                uuid: request.uuid,
            };
            
            let response = thrift_client
                .describe_domain(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(DescribeDomainResponse {
                domain_info: response.domain_info.and_then(|di| {
                    Some(DomainInfo {
                        name: di.name?,
                        status: di.status.and_then(|s| match s {
                            thrift_types::DomainStatus::REGISTERED => Some(DomainStatus::Registered),
                            thrift_types::DomainStatus::DEPRECATED => Some(DomainStatus::Deprecated),
                            thrift_types::DomainStatus::DELETED => Some(DomainStatus::Deleted),
                            _ => None,
                        }),
                        description: di.description?,
                        owner_email: di.owner_email?,
                        data: di.data.unwrap_or_default().into_iter().collect(),
                        uuid: di.uuid?,
                    })
                }),
                configuration: response.configuration.and_then(|c| {
                    Some(DomainConfiguration {
                        workflow_execution_retention_period_in_days: c.workflow_execution_retention_period_in_days?,
                        emit_metric: c.emit_metric?,
                        history_archival_status: c.history_archival_status.and_then(|s| match s {
                            thrift_types::ArchivalStatus::DISABLED => Some(ArchivalStatus::Disabled),
                            thrift_types::ArchivalStatus::ENABLED => Some(ArchivalStatus::Enabled),
                            _ => None,
                        }),
                        history_archival_uri: c.history_archival_u_r_i?,
                        visibility_archival_status: c.visibility_archival_status.and_then(|s| match s {
                            thrift_types::ArchivalStatus::DISABLED => Some(ArchivalStatus::Disabled),
                            thrift_types::ArchivalStatus::ENABLED => Some(ArchivalStatus::Enabled),
                            _ => None,
                        }),
                        visibility_archival_uri: c.visibility_archival_u_r_i?,
                    })
                }),
                replication_configuration: response.replication_configuration.and_then(|rc| {
                    Some(DomainReplicationConfiguration {
                        active_cluster_name: rc.active_cluster_name?,
                        clusters: rc.clusters.unwrap_or_default().into_iter().filter_map(|c| {
                            Some(ClusterReplicationConfiguration {
                                cluster_name: c.cluster_name?,
                            })
                        }).collect(),
                    })
                }),
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn update_domain(
        &self,
        request: UpdateDomainRequest,
    ) -> Result<UpdateDomainResponse, Self::Error> {
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::UpdateDomainRequest {
                name: request.name,
                updated_info: request.updated_info.map(|ui| thrift_types::UpdateDomainInfo {
                    description: Some(ui.description),
                    owner_email: Some(ui.owner_email),
                    data: Some(ui.data.into_iter().collect()),
                }),
                configuration: request.configuration.map(|c| thrift_types::DomainConfiguration {
                    workflow_execution_retention_period_in_days: Some(c.workflow_execution_retention_period_in_days),
                    emit_metric: Some(c.emit_metric),
                    isolationgroups: None,
                    bad_binaries: None,
                    history_archival_status: c.history_archival_status.map(|s| match s {
                        ArchivalStatus::Disabled => thrift_types::ArchivalStatus::DISABLED,
                        ArchivalStatus::Enabled => thrift_types::ArchivalStatus::ENABLED,
                    }),
                    history_archival_u_r_i: Some(c.history_archival_uri),
                    visibility_archival_status: c.visibility_archival_status.map(|s| match s {
                        ArchivalStatus::Disabled => thrift_types::ArchivalStatus::DISABLED,
                        ArchivalStatus::Enabled => thrift_types::ArchivalStatus::ENABLED,
                    }),
                    visibility_archival_u_r_i: Some(c.visibility_archival_uri),
                    async_workflow_configuration: None,
                }),
                replication_configuration: request.replication_configuration.map(|rc| thrift_types::DomainReplicationConfiguration {
                    active_cluster_name: Some(rc.active_cluster_name),
                    clusters: Some(rc.clusters.into_iter().map(|c| thrift_types::ClusterReplicationConfiguration {
                        cluster_name: Some(c.cluster_name),
                    }).collect()),
                    active_clusters: None,
                }),
                security_token: request.security_token,
                delete_bad_binary: request.delete_bad_binary,
                failover_timeout_in_seconds: None,
            };
            
            let _response = thrift_client
                .update_domain(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            // Similar conversion as describe_domain response
            Ok(UpdateDomainResponse {
                domain_info: None, // TODO: Convert response
                configuration: None,
                replication_configuration: None,
            })
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }

    async fn failover_domain(
        &self,
        request: FailoverDomainRequest,
    ) -> Result<FailoverDomainResponse, Self::Error> {
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut thrift_client = Self::create_thrift_client_blocking(&config)?;
            
            let thrift_request = thrift_types::FailoverDomainRequest {
                domain_name: Some(request.name),
                domain_active_cluster_name: request.clusters.first().cloned(),
                active_clusters: None,
                reason: None,
            };
            
            thrift_client
                .failover_domain(thrift_request)
                .map_err(|e| Self::convert_thrift_error(e))?;
            
            Ok(FailoverDomainResponse {})
        })
        .await
        .map_err(|e| CadenceError::Other(format!("Task join error: {}", e)))?
    }
}
