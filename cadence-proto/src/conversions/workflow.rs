//! Conversions for workflow service requests and responses

use crate::generated as pb;
use crate::workflow_service as api;

use super::helpers::*;

// ============================================================================
// Start Workflow Execution
// ============================================================================

impl From<api::StartWorkflowExecutionRequest> for pb::StartWorkflowExecutionRequest {
    fn from(req: api::StartWorkflowExecutionRequest) -> Self {
        pb::StartWorkflowExecutionRequest {
            domain: req.domain,
            workflow_id: req.workflow_id,
            workflow_type: req.workflow_type.map(Into::into),
            task_list: req.task_list.map(Into::into),
            input: bytes_to_payload(req.input),
            execution_start_to_close_timeout: seconds_to_duration(
                req.execution_start_to_close_timeout_seconds,
            ),
            task_start_to_close_timeout: seconds_to_duration(
                req.task_start_to_close_timeout_seconds,
            ),
            identity: req.identity,
            request_id: req.request_id,
            workflow_id_reuse_policy: req.workflow_id_reuse_policy.map(|p| p as i32).unwrap_or(0),
            retry_policy: req.retry_policy.map(Into::into),
            cron_schedule: req.cron_schedule.unwrap_or_default(),
            memo: req.memo.map(api_memo_to_pb),
            search_attributes: req.search_attributes.map(api_search_attributes_to_pb),
            header: req.header.map(api_header_to_pb),
            delay_start: seconds_to_duration(req.delay_start_seconds),
            jitter_start: seconds_to_duration(req.jitter_start_seconds),
            first_run_at: None,                    // Not used in our API yet
            cron_overlap_policy: 0,                // Default
            active_cluster_selection_policy: None, // Not used in our API yet
        }
    }
}

impl From<pb::StartWorkflowExecutionResponse> for api::StartWorkflowExecutionResponse {
    fn from(resp: pb::StartWorkflowExecutionResponse) -> Self {
        api::StartWorkflowExecutionResponse {
            run_id: resp.run_id,
        }
    }
}

// ============================================================================
// Signal Workflow Execution
// ============================================================================

impl From<api::SignalWorkflowExecutionRequest> for pb::SignalWorkflowExecutionRequest {
    fn from(req: api::SignalWorkflowExecutionRequest) -> Self {
        pb::SignalWorkflowExecutionRequest {
            domain: req.domain,
            workflow_execution: req.workflow_execution.map(Into::into),
            identity: req.identity,
            request_id: req.request_id,
            signal_name: req.signal_name,
            signal_input: bytes_to_payload(req.input),
            control: req.control.map(|s| s.into_bytes()).unwrap_or_default(),
        }
    }
}

impl From<pb::SignalWorkflowExecutionResponse> for api::SignalWorkflowExecutionResponse {
    fn from(_: pb::SignalWorkflowExecutionResponse) -> Self {
        api::SignalWorkflowExecutionResponse {}
    }
}

// ============================================================================
// Signal With Start Workflow Execution
// ============================================================================

impl From<api::SignalWithStartWorkflowExecutionRequest>
    for pb::SignalWithStartWorkflowExecutionRequest
{
    fn from(req: api::SignalWithStartWorkflowExecutionRequest) -> Self {
        // Build the embedded start_request
        let start_request = pb::StartWorkflowExecutionRequest {
            domain: req.domain,
            workflow_id: req.workflow_id,
            workflow_type: req.workflow_type.map(Into::into),
            task_list: req.task_list.map(Into::into),
            input: bytes_to_payload(req.input),
            execution_start_to_close_timeout: seconds_to_duration(
                req.execution_start_to_close_timeout_seconds,
            ),
            task_start_to_close_timeout: seconds_to_duration(
                req.task_start_to_close_timeout_seconds,
            ),
            identity: req.identity,
            request_id: req.request_id,
            workflow_id_reuse_policy: req.workflow_id_reuse_policy.map(|p| p as i32).unwrap_or(0),
            retry_policy: req.retry_policy.map(Into::into),
            cron_schedule: req.cron_schedule.unwrap_or_default(),
            memo: req.memo.map(api_memo_to_pb),
            search_attributes: req.search_attributes.map(api_search_attributes_to_pb),
            header: req.header.map(api_header_to_pb),
            delay_start: None,                     // Not in our API type
            jitter_start: None,                    // Not in our API type
            first_run_at: None,                    // Not in our API type
            cron_overlap_policy: 0,                // Default
            active_cluster_selection_policy: None, // Not in our API type
        };

        pb::SignalWithStartWorkflowExecutionRequest {
            start_request: Some(start_request),
            signal_name: req.signal_name,
            signal_input: bytes_to_payload(req.signal_input),
            control: Vec::new(), // Not in our API type
        }
    }
}

impl From<pb::SignalWithStartWorkflowExecutionResponse> for api::StartWorkflowExecutionResponse {
    fn from(resp: pb::SignalWithStartWorkflowExecutionResponse) -> Self {
        api::StartWorkflowExecutionResponse {
            run_id: resp.run_id,
        }
    }
}

// ============================================================================
// Cancel Workflow Execution
// ============================================================================

impl From<api::RequestCancelWorkflowExecutionRequest>
    for pb::RequestCancelWorkflowExecutionRequest
{
    fn from(req: api::RequestCancelWorkflowExecutionRequest) -> Self {
        pb::RequestCancelWorkflowExecutionRequest {
            domain: req.domain,
            workflow_execution: req.workflow_execution.map(Into::into),
            identity: req.identity,
            request_id: req.request_id,
            cause: req.cause.unwrap_or_default(),
            first_execution_run_id: req.first_execution_run_id.unwrap_or_default(),
        }
    }
}

impl From<pb::RequestCancelWorkflowExecutionResponse>
    for api::RequestCancelWorkflowExecutionResponse
{
    fn from(_: pb::RequestCancelWorkflowExecutionResponse) -> Self {
        api::RequestCancelWorkflowExecutionResponse {}
    }
}

// ============================================================================
// Terminate Workflow Execution
// ============================================================================

impl From<api::TerminateWorkflowExecutionRequest> for pb::TerminateWorkflowExecutionRequest {
    fn from(req: api::TerminateWorkflowExecutionRequest) -> Self {
        pb::TerminateWorkflowExecutionRequest {
            domain: req.domain,
            workflow_execution: req.workflow_execution.map(Into::into),
            reason: req.reason.unwrap_or_default(),
            details: bytes_to_payload(req.details),
            identity: req.identity,
            first_execution_run_id: req.first_execution_run_id.unwrap_or_default(),
        }
    }
}

impl From<pb::TerminateWorkflowExecutionResponse> for api::TerminateWorkflowExecutionResponse {
    fn from(_: pb::TerminateWorkflowExecutionResponse) -> Self {
        api::TerminateWorkflowExecutionResponse {}
    }
}

// ============================================================================
// Query Workflow
// ============================================================================

impl From<api::WorkflowQuery> for pb::WorkflowQuery {
    fn from(q: api::WorkflowQuery) -> Self {
        pb::WorkflowQuery {
            query_type: q.query_type,
            query_args: bytes_to_payload(q.query_args),
        }
    }
}

impl From<api::QueryWorkflowRequest> for pb::QueryWorkflowRequest {
    fn from(req: api::QueryWorkflowRequest) -> Self {
        pb::QueryWorkflowRequest {
            domain: req.domain,
            workflow_execution: req.execution.map(Into::into),
            query: req.query.map(Into::into),
            query_reject_condition: 0, // Default - not exposed in our API yet
            query_consistency_level: 0, // Default - not exposed in our API yet
        }
    }
}

impl From<pb::QueryWorkflowResponse> for api::QueryWorkflowResponse {
    fn from(resp: pb::QueryWorkflowResponse) -> Self {
        api::QueryWorkflowResponse {
            query_result: payload_to_bytes(resp.query_result),
            query_rejected: resp.query_rejected.map(|qr| api::QueryRejected {
                close_status: Some(match qr.close_status {
                    1 => api::WorkflowExecutionCloseStatus::Failed,
                    2 => api::WorkflowExecutionCloseStatus::Canceled,
                    3 => api::WorkflowExecutionCloseStatus::Terminated,
                    4 => api::WorkflowExecutionCloseStatus::ContinuedAsNew,
                    5 => api::WorkflowExecutionCloseStatus::TimedOut,
                    _ => api::WorkflowExecutionCloseStatus::Completed,
                }),
            }),
        }
    }
}

/// Convert protobuf WorkflowQuery to API WorkflowQuery
pub(super) fn pb_workflow_query_to_api(q: pb::WorkflowQuery) -> api::WorkflowQuery {
    api::WorkflowQuery {
        query_type: q.query_type,
        query_args: payload_to_bytes(q.query_args),
    }
}

// ============================================================================
// Get Workflow Execution History
// ============================================================================

impl From<api::GetWorkflowExecutionHistoryRequest> for pb::GetWorkflowExecutionHistoryRequest {
    fn from(req: api::GetWorkflowExecutionHistoryRequest) -> Self {
        pb::GetWorkflowExecutionHistoryRequest {
            domain: req.domain,
            workflow_execution: req.execution.map(Into::into),
            page_size: req.page_size,
            next_page_token: req.next_page_token.unwrap_or_default(),
            wait_for_new_event: req.wait_for_new_event,
            history_event_filter_type: req.history_event_filter_type.map(|t| t as i32).unwrap_or(0),
            skip_archival: req.skip_archival,
            query_consistency_level: 0, // Default - not in our API type
        }
    }
}

impl From<pb::GetWorkflowExecutionHistoryResponse> for api::GetWorkflowExecutionHistoryResponse {
    fn from(resp: pb::GetWorkflowExecutionHistoryResponse) -> Self {
        api::GetWorkflowExecutionHistoryResponse {
            history: resp.history.map(super::history::pb_history_to_api),
            next_page_token: Some(resp.next_page_token),
            archived: resp.archived,
        }
    }
}

// ============================================================================
// Describe Workflow Execution
// ============================================================================

impl From<api::DescribeWorkflowExecutionRequest> for pb::DescribeWorkflowExecutionRequest {
    fn from(req: api::DescribeWorkflowExecutionRequest) -> Self {
        pb::DescribeWorkflowExecutionRequest {
            domain: req.domain,
            workflow_execution: req.execution.map(Into::into),
            query_consistency_level: 0, // Default - not in our API type
        }
    }
}

impl From<pb::DescribeWorkflowExecutionResponse> for api::DescribeWorkflowExecutionResponse {
    fn from(resp: pb::DescribeWorkflowExecutionResponse) -> Self {
        api::DescribeWorkflowExecutionResponse {
            execution_configuration: resp
                .execution_configuration
                .map(pb_workflow_execution_config_to_api),
            workflow_execution_info: resp
                .workflow_execution_info
                .map(pb_workflow_execution_info_to_api),
            pending_children: resp
                .pending_children
                .into_iter()
                .map(pb_pending_child_to_api)
                .collect(),
            pending_decision: resp.pending_decision.map(pb_pending_decision_to_api),
            pending_activities: resp
                .pending_activities
                .into_iter()
                .map(pb_pending_activity_to_api)
                .collect(),
        }
    }
}

// Helper conversions for DescribeWorkflowExecutionResponse
fn pb_workflow_execution_config_to_api(
    cfg: pb::WorkflowExecutionConfiguration,
) -> api::WorkflowExecutionConfiguration {
    api::WorkflowExecutionConfiguration {
        task_list: cfg.task_list.map(Into::into),
        execution_start_to_close_timeout_seconds: duration_to_seconds(
            cfg.execution_start_to_close_timeout,
        )
        .unwrap_or(0),
        task_start_to_close_timeout_seconds: duration_to_seconds(cfg.task_start_to_close_timeout)
            .unwrap_or(0),
    }
}

pub(super) fn pb_workflow_execution_info_to_api(
    info: pb::WorkflowExecutionInfo,
) -> api::WorkflowExecutionInfo {
    api::WorkflowExecutionInfo {
        execution: info.workflow_execution.map(Into::into),
        workflow_type: info.r#type.map(Into::into),
        start_time: info.start_time.and_then(timestamp_to_nanos),
        close_time: info.close_time.and_then(timestamp_to_nanos),
        close_status: match info.close_status {
            1 => Some(api::WorkflowExecutionCloseStatus::Completed),
            2 => Some(api::WorkflowExecutionCloseStatus::Failed),
            3 => Some(api::WorkflowExecutionCloseStatus::Canceled),
            4 => Some(api::WorkflowExecutionCloseStatus::Terminated),
            5 => Some(api::WorkflowExecutionCloseStatus::ContinuedAsNew),
            6 => Some(api::WorkflowExecutionCloseStatus::TimedOut),
            _ => None,
        },
        history_length: info.history_length,
        parent_domain_id: info
            .parent_execution_info
            .as_ref()
            .map(|p| p.domain_name.clone()),
        parent_execution: info
            .parent_execution_info
            .and_then(|p| p.workflow_execution)
            .map(Into::into),
        execution_time: info.execution_time.and_then(timestamp_to_nanos),
        memo: info.memo.map(pb_memo_to_api),
        search_attributes: info.search_attributes.map(pb_search_attributes_to_api),
        auto_reset_points: None, // Simplified - full conversion would be complex
        task_list: info.task_list,
    }
}

fn pb_pending_child_to_api(child: pb::PendingChildExecutionInfo) -> api::PendingChildExecutionInfo {
    let (workflow_id, run_id) = child
        .workflow_execution
        .map(|we| (we.workflow_id, we.run_id))
        .unwrap_or_default();

    api::PendingChildExecutionInfo {
        workflow_id,
        run_id,
        workflow_type_name: child.workflow_type_name,
        initiated_id: child.initiated_id,
    }
}

fn pb_pending_decision_to_api(dec: pb::PendingDecisionInfo) -> api::PendingDecisionInfo {
    api::PendingDecisionInfo {
        state: Some(match dec.state {
            1 => api::PendingDecisionState::Started,
            _ => api::PendingDecisionState::Scheduled,
        }),
        scheduled_timestamp: dec.scheduled_time.and_then(timestamp_to_nanos),
        started_timestamp: dec.started_time.and_then(timestamp_to_nanos),
        attempt: dec.attempt,
        original_scheduled_timestamp: dec.original_scheduled_time.and_then(timestamp_to_nanos),
    }
}

fn pb_pending_activity_to_api(act: pb::PendingActivityInfo) -> api::PendingActivityInfo {
    api::PendingActivityInfo {
        activity_id: act.activity_id,
        activity_type: act.activity_type.map(Into::into),
        state: Some(match act.state {
            1 => api::PendingActivityState::Started,
            2 => api::PendingActivityState::CancelRequested,
            _ => api::PendingActivityState::Scheduled,
        }),
        scheduled_timestamp: act.scheduled_time.and_then(timestamp_to_nanos),
        last_started_timestamp: act.last_started_time.and_then(timestamp_to_nanos),
        last_heartbeat_timestamp: act.last_heartbeat_time.and_then(timestamp_to_nanos),
        attempt: act.attempt,
        maximum_attempts: act.maximum_attempts,
        scheduled_timestamp_of_this_attempt: None, // Not available in protobuf
        heartbeat_details: payload_to_bytes(act.heartbeat_details),
    }
}

// ============================================================================
// List Workflow Executions
// ============================================================================

impl From<api::ListOpenWorkflowExecutionsRequest> for pb::ListOpenWorkflowExecutionsRequest {
    fn from(req: api::ListOpenWorkflowExecutionsRequest) -> Self {
        let filters = if let Some(exec_filter) = req.execution_filter {
            Some(
                pb::list_open_workflow_executions_request::Filters::ExecutionFilter(
                    pb::WorkflowExecutionFilter {
                        workflow_id: exec_filter.workflow_id,
                        run_id: String::new(),
                    },
                ),
            )
        } else if let Some(type_filter) = req.type_filter {
            Some(
                pb::list_open_workflow_executions_request::Filters::TypeFilter(
                    pb::WorkflowTypeFilter {
                        name: type_filter.name,
                    },
                ),
            )
        } else {
            None
        };

        pb::ListOpenWorkflowExecutionsRequest {
            domain: req.domain,
            page_size: req.maximum_page_size,
            next_page_token: req.next_page_token.unwrap_or_default(),
            start_time_filter: req.start_time_filter.map(|f| pb::StartTimeFilter {
                earliest_time: f.earliest_time.and_then(nanos_to_timestamp),
                latest_time: f.latest_time.and_then(nanos_to_timestamp),
            }),
            filters,
        }
    }
}

impl From<pb::ListOpenWorkflowExecutionsResponse> for api::ListOpenWorkflowExecutionsResponse {
    fn from(resp: pb::ListOpenWorkflowExecutionsResponse) -> Self {
        api::ListOpenWorkflowExecutionsResponse {
            executions: resp
                .executions
                .into_iter()
                .map(pb_workflow_execution_info_to_api)
                .collect(),
            next_page_token: if resp.next_page_token.is_empty() {
                None
            } else {
                Some(resp.next_page_token)
            },
        }
    }
}

impl From<api::ListClosedWorkflowExecutionsRequest> for pb::ListClosedWorkflowExecutionsRequest {
    fn from(req: api::ListClosedWorkflowExecutionsRequest) -> Self {
        let filters = if let Some(exec_filter) = req.execution_filter {
            Some(
                pb::list_closed_workflow_executions_request::Filters::ExecutionFilter(
                    pb::WorkflowExecutionFilter {
                        workflow_id: exec_filter.workflow_id,
                        run_id: String::new(),
                    },
                ),
            )
        } else if let Some(type_filter) = req.type_filter {
            Some(
                pb::list_closed_workflow_executions_request::Filters::TypeFilter(
                    pb::WorkflowTypeFilter {
                        name: type_filter.name,
                    },
                ),
            )
        } else {
            req.status_filter.map(|status_filter| {
                pb::list_closed_workflow_executions_request::Filters::StatusFilter(
                    pb::StatusFilter {
                        status: status_filter as i32,
                    },
                )
            })
        };

        pb::ListClosedWorkflowExecutionsRequest {
            domain: req.domain,
            page_size: req.maximum_page_size,
            next_page_token: req.next_page_token.unwrap_or_default(),
            start_time_filter: req.start_time_filter.map(|f| pb::StartTimeFilter {
                earliest_time: f.earliest_time.and_then(nanos_to_timestamp),
                latest_time: f.latest_time.and_then(nanos_to_timestamp),
            }),
            filters,
        }
    }
}

impl From<pb::ListClosedWorkflowExecutionsResponse> for api::ListClosedWorkflowExecutionsResponse {
    fn from(resp: pb::ListClosedWorkflowExecutionsResponse) -> Self {
        api::ListClosedWorkflowExecutionsResponse {
            executions: resp
                .executions
                .into_iter()
                .map(pb_workflow_execution_info_to_api)
                .collect(),
            next_page_token: if resp.next_page_token.is_empty() {
                None
            } else {
                Some(resp.next_page_token)
            },
        }
    }
}
