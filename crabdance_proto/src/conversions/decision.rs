//! Conversions for decision task operations

use crate::generated as pb;
use crate::shared as api_types;
use crate::workflow_service as api;

use super::helpers::*;

// ============================================================================
// Poll for Decision Task
// ============================================================================

impl From<api::PollForDecisionTaskRequest> for pb::PollForDecisionTaskRequest {
    fn from(req: api::PollForDecisionTaskRequest) -> Self {
        pb::PollForDecisionTaskRequest {
            domain: req.domain,
            task_list: req.task_list.map(Into::into),
            identity: req.identity,
            binary_checksum: req.binary_checksum,
        }
    }
}

impl From<pb::PollForDecisionTaskResponse> for api::PollForDecisionTaskResponse {
    fn from(resp: pb::PollForDecisionTaskResponse) -> Self {
        api::PollForDecisionTaskResponse {
            task_token: resp.task_token,
            workflow_execution: resp.workflow_execution.map(Into::into),
            workflow_type: resp.workflow_type.map(Into::into),
            previous_started_event_id: resp.previous_started_event_id.unwrap_or(0),
            started_event_id: resp.started_event_id,
            attempt: resp.attempt as i32,
            backlog_count_hint: resp.backlog_count_hint,
            history: resp.history.map(super::history::pb_history_to_api),
            next_page_token: Some(resp.next_page_token),
            query: resp.query.map(super::workflow::pb_workflow_query_to_api),
            workflow_execution_task_list: resp.workflow_execution_task_list.map(Into::into),
            scheduled_timestamp: resp.scheduled_time.and_then(timestamp_to_nanos),
            started_timestamp: resp.started_time.and_then(timestamp_to_nanos),
            queries: Some(
                resp.queries
                    .into_iter()
                    .map(|(k, v)| (k, super::workflow::pb_workflow_query_to_api(v)))
                    .collect(),
            ),
        }
    }
}

// ============================================================================
// Respond Decision Task Completed
// ============================================================================

impl From<api::RespondDecisionTaskCompletedRequest> for pb::RespondDecisionTaskCompletedRequest {
    fn from(req: api::RespondDecisionTaskCompletedRequest) -> Self {
        pb::RespondDecisionTaskCompletedRequest {
            task_token: req.task_token,
            decisions: req.decisions.into_iter().map(api_decision_to_pb).collect(),
            execution_context: req.execution_context.unwrap_or_default(),
            identity: req.identity,
            sticky_attributes: req
                .sticky_attributes
                .map(|sa| pb::StickyExecutionAttributes {
                    worker_task_list: sa.worker_task_list.map(Into::into),
                    schedule_to_start_timeout: seconds_to_duration(Some(
                        sa.schedule_to_start_timeout_seconds,
                    )),
                }),
            return_new_decision_task: req.return_new_decision_task,
            force_create_new_decision_task: req.force_create_new_decision_task,
            binary_checksum: req.binary_checksum,
            query_results: req
                .query_results
                .map(|qr| {
                    qr.into_iter()
                        .map(|(k, v)| (k, api_workflow_query_result_to_pb(v)))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

impl From<pb::RespondDecisionTaskCompletedResponse> for api::RespondDecisionTaskCompletedResponse {
    fn from(resp: pb::RespondDecisionTaskCompletedResponse) -> Self {
        api::RespondDecisionTaskCompletedResponse {
            decision_task: resp.decision_task.map(Into::into),
        }
    }
}

// ============================================================================
// Respond Decision Task Failed
// ============================================================================

impl From<api::RespondDecisionTaskFailedRequest> for pb::RespondDecisionTaskFailedRequest {
    fn from(req: api::RespondDecisionTaskFailedRequest) -> Self {
        pb::RespondDecisionTaskFailedRequest {
            task_token: req.task_token,
            cause: req.cause as i32,
            details: bytes_to_payload(req.details),
            identity: req.identity,
            binary_checksum: req.binary_checksum,
        }
    }
}

impl From<pb::RespondDecisionTaskFailedResponse> for api::RespondDecisionTaskFailedResponse {
    fn from(_: pb::RespondDecisionTaskFailedResponse) -> Self {
        api::RespondDecisionTaskFailedResponse {}
    }
}

// ============================================================================
// Decision Attribute Conversions
// ============================================================================

/// Convert API Decision to protobuf Decision
fn api_decision_to_pb(d: api_types::Decision) -> pb::Decision {
    use api_types::DecisionAttributes as ApiAttr;
    use pb::decision::Attributes as PbAttr;

    let attributes = d.attributes.map(|attr| match attr {
        ApiAttr::ScheduleActivityTaskDecisionAttributes(a) => {
            PbAttr::ScheduleActivityTaskDecisionAttributes(
                pb::ScheduleActivityTaskDecisionAttributes {
                    activity_id: a.activity_id,
                    activity_type: a.activity_type.map(Into::into),
                    task_list: a.task_list.map(Into::into),
                    input: bytes_to_payload(a.input),
                    schedule_to_close_timeout: seconds_to_duration(
                        a.schedule_to_close_timeout_seconds,
                    ),
                    schedule_to_start_timeout: seconds_to_duration(
                        a.schedule_to_start_timeout_seconds,
                    ),
                    start_to_close_timeout: seconds_to_duration(a.start_to_close_timeout_seconds),
                    heartbeat_timeout: seconds_to_duration(a.heartbeat_timeout_seconds),
                    retry_policy: a.retry_policy.map(Into::into),
                    header: a.header.map(api_header_to_pb),
                    request_local_dispatch: false, // Default
                    domain: String::new(),         // Not used
                },
            )
        }
        ApiAttr::StartTimerDecisionAttributes(a) => {
            PbAttr::StartTimerDecisionAttributes(pb::StartTimerDecisionAttributes {
                timer_id: a.timer_id,
                start_to_fire_timeout: Some(prost_types::Duration {
                    seconds: a.start_to_fire_timeout_seconds,
                    nanos: 0,
                }),
            })
        }
        ApiAttr::CompleteWorkflowExecutionDecisionAttributes(a) => {
            PbAttr::CompleteWorkflowExecutionDecisionAttributes(
                pb::CompleteWorkflowExecutionDecisionAttributes {
                    result: bytes_to_payload(a.result),
                },
            )
        }
        ApiAttr::FailWorkflowExecutionDecisionAttributes(a) => {
            PbAttr::FailWorkflowExecutionDecisionAttributes(
                pb::FailWorkflowExecutionDecisionAttributes {
                    failure: Some(pb::Failure {
                        reason: a.reason.unwrap_or_default(),
                        details: a.details.unwrap_or_default(),
                    }),
                },
            )
        }
        ApiAttr::CancelTimerDecisionAttributes(a) => {
            PbAttr::CancelTimerDecisionAttributes(pb::CancelTimerDecisionAttributes {
                timer_id: a.timer_id,
            })
        }
        ApiAttr::CancelWorkflowExecutionDecisionAttributes(a) => {
            PbAttr::CancelWorkflowExecutionDecisionAttributes(
                pb::CancelWorkflowExecutionDecisionAttributes {
                    details: bytes_to_payload(a.details),
                },
            )
        }
        ApiAttr::RequestCancelExternalWorkflowExecutionDecisionAttributes(a) => {
            PbAttr::RequestCancelExternalWorkflowExecutionDecisionAttributes(
                pb::RequestCancelExternalWorkflowExecutionDecisionAttributes {
                    domain: a.domain,
                    workflow_execution: a.workflow_execution.map(Into::into),
                    control: a.control.map(|s| s.into_bytes()).unwrap_or_default(),
                    child_workflow_only: a.child_workflow_only,
                },
            )
        }
        ApiAttr::RecordMarkerDecisionAttributes(a) => {
            PbAttr::RecordMarkerDecisionAttributes(pb::RecordMarkerDecisionAttributes {
                marker_name: a.marker_name,
                details: bytes_to_payload(a.details),
                header: a.header.map(api_header_to_pb),
            })
        }
        ApiAttr::ContinueAsNewWorkflowExecutionDecisionAttributes(a) => {
            PbAttr::ContinueAsNewWorkflowExecutionDecisionAttributes(
                pb::ContinueAsNewWorkflowExecutionDecisionAttributes {
                    workflow_type: a.workflow_type.map(Into::into),
                    task_list: a.task_list.map(Into::into),
                    input: bytes_to_payload(a.input),
                    execution_start_to_close_timeout: seconds_to_duration(
                        a.execution_start_to_close_timeout_seconds,
                    ),
                    task_start_to_close_timeout: seconds_to_duration(
                        a.task_start_to_close_timeout_seconds,
                    ),
                    backoff_start_interval: seconds_to_duration(
                        a.backoff_start_interval_in_seconds,
                    ),
                    retry_policy: a.retry_policy.map(Into::into),
                    initiator: a.initiator.map(|i| i as i32).unwrap_or(0),
                    failure: a.failure_details.map(|details| pb::Failure {
                        reason: String::new(),
                        details,
                    }),
                    last_completion_result: bytes_to_payload(a.last_completion_result),
                    cron_schedule: a.cron_schedule.unwrap_or_default(),
                    header: a.header.map(api_header_to_pb),
                    memo: a.memo.map(api_memo_to_pb),
                    search_attributes: a.search_attributes.map(api_search_attributes_to_pb),
                    jitter_start: None,                    // Not in our API type
                    cron_overlap_policy: 0,                // Default
                    active_cluster_selection_policy: None, // Not in our API type
                },
            )
        }
        ApiAttr::StartChildWorkflowExecutionDecisionAttributes(a) => {
            PbAttr::StartChildWorkflowExecutionDecisionAttributes(
                pb::StartChildWorkflowExecutionDecisionAttributes {
                    domain: a.domain,
                    workflow_id: a.workflow_id,
                    workflow_type: a.workflow_type.map(Into::into),
                    task_list: a.task_list.map(Into::into),
                    input: bytes_to_payload(a.input),
                    execution_start_to_close_timeout: seconds_to_duration(
                        a.execution_start_to_close_timeout_seconds,
                    ),
                    task_start_to_close_timeout: seconds_to_duration(
                        a.task_start_to_close_timeout_seconds,
                    ),
                    parent_close_policy: a.parent_close_policy.map(|p| p as i32).unwrap_or(0),
                    control: a.control.map(|s| s.into_bytes()).unwrap_or_default(),
                    workflow_id_reuse_policy: a
                        .workflow_id_reuse_policy
                        .map(|p| p as i32)
                        .unwrap_or(0),
                    retry_policy: a.retry_policy.map(Into::into),
                    cron_schedule: a.cron_schedule.unwrap_or_default(),
                    header: a.header.map(api_header_to_pb),
                    memo: a.memo.map(api_memo_to_pb),
                    search_attributes: a.search_attributes.map(api_search_attributes_to_pb),
                    cron_overlap_policy: 0,                // Default
                    active_cluster_selection_policy: None, // Not in our API type
                },
            )
        }
        ApiAttr::SignalExternalWorkflowExecutionDecisionAttributes(a) => {
            PbAttr::SignalExternalWorkflowExecutionDecisionAttributes(
                pb::SignalExternalWorkflowExecutionDecisionAttributes {
                    domain: a.domain,
                    workflow_execution: a.workflow_execution.map(Into::into),
                    signal_name: a.signal_name,
                    input: bytes_to_payload(a.input),
                    control: a.control.map(|s| s.into_bytes()).unwrap_or_default(),
                    child_workflow_only: a.child_workflow_only,
                },
            )
        }
        ApiAttr::UpsertWorkflowSearchAttributesDecisionAttributes(a) => {
            PbAttr::UpsertWorkflowSearchAttributesDecisionAttributes(
                pb::UpsertWorkflowSearchAttributesDecisionAttributes {
                    search_attributes: a.search_attributes.map(api_search_attributes_to_pb),
                },
            )
        }
    });

    pb::Decision { attributes }
}

/// Convert API WorkflowQueryResult to protobuf WorkflowQueryResult
fn api_workflow_query_result_to_pb(qr: api::WorkflowQueryResult) -> pb::WorkflowQueryResult {
    pb::WorkflowQueryResult {
        result_type: qr.query_result_type as i32,
        answer: bytes_to_payload(qr.answer),
        error_message: qr.error_message.unwrap_or_default(),
    }
}
