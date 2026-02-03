//! Conversions for activity task operations

use crate::generated as pb;
use crate::workflow_service as api;

use super::helpers::*;

// ============================================================================
// Poll for Activity Task
// ============================================================================

impl From<api::PollForActivityTaskRequest> for pb::PollForActivityTaskRequest {
    fn from(req: api::PollForActivityTaskRequest) -> Self {
        pb::PollForActivityTaskRequest {
            domain: req.domain,
            task_list: req.task_list.map(Into::into),
            identity: req.identity,
            task_list_metadata: req.task_list_metadata.map(|m| pb::TaskListMetadata {
                max_tasks_per_second: m.max_tasks_per_second,
            }),
        }
    }
}

impl From<pb::PollForActivityTaskResponse> for api::PollForActivityTaskResponse {
    fn from(resp: pb::PollForActivityTaskResponse) -> Self {
        api::PollForActivityTaskResponse {
            task_token: resp.task_token,
            workflow_execution: resp.workflow_execution.map(Into::into),
            activity_id: resp.activity_id,
            activity_type: resp.activity_type.map(Into::into),
            input: payload_to_bytes(resp.input),
            scheduled_timestamp: resp.scheduled_time.and_then(timestamp_to_nanos),
            started_timestamp: resp.started_time.and_then(timestamp_to_nanos),
            schedule_to_close_timeout_seconds: duration_to_seconds(resp.schedule_to_close_timeout),
            start_to_close_timeout_seconds: duration_to_seconds(resp.start_to_close_timeout),
            heartbeat_timeout_seconds: duration_to_seconds(resp.heartbeat_timeout),
            attempt: resp.attempt,
            scheduled_timestamp_of_this_attempt: resp
                .scheduled_time_of_this_attempt
                .and_then(timestamp_to_nanos),
            heartbeat_details: payload_to_bytes(resp.heartbeat_details),
            workflow_type: resp.workflow_type.map(Into::into),
            workflow_domain: Some(resp.workflow_domain),
            header: resp.header.map(pb_header_to_api),
        }
    }
}

// ============================================================================
// Record Activity Task Heartbeat
// ============================================================================

impl From<api::RecordActivityTaskHeartbeatRequest> for pb::RecordActivityTaskHeartbeatRequest {
    fn from(req: api::RecordActivityTaskHeartbeatRequest) -> Self {
        pb::RecordActivityTaskHeartbeatRequest {
            task_token: req.task_token,
            details: bytes_to_payload(req.details),
            identity: req.identity,
        }
    }
}

impl From<pb::RecordActivityTaskHeartbeatResponse> for api::RecordActivityTaskHeartbeatResponse {
    fn from(resp: pb::RecordActivityTaskHeartbeatResponse) -> Self {
        api::RecordActivityTaskHeartbeatResponse {
            cancel_requested: resp.cancel_requested,
        }
    }
}

// ============================================================================
// Respond Activity Task Completed
// ============================================================================

impl From<api::RespondActivityTaskCompletedRequest> for pb::RespondActivityTaskCompletedRequest {
    fn from(req: api::RespondActivityTaskCompletedRequest) -> Self {
        pb::RespondActivityTaskCompletedRequest {
            task_token: req.task_token,
            result: bytes_to_payload(req.result),
            identity: req.identity,
        }
    }
}

impl From<pb::RespondActivityTaskCompletedResponse> for api::RespondActivityTaskCompletedResponse {
    fn from(_: pb::RespondActivityTaskCompletedResponse) -> Self {
        api::RespondActivityTaskCompletedResponse {}
    }
}

// ============================================================================
// Respond Activity Task Failed
// ============================================================================

impl From<api::RespondActivityTaskFailedRequest> for pb::RespondActivityTaskFailedRequest {
    fn from(req: api::RespondActivityTaskFailedRequest) -> Self {
        pb::RespondActivityTaskFailedRequest {
            task_token: req.task_token,
            failure: Some(pb::Failure {
                reason: req.reason.unwrap_or_default(),
                details: req.details.unwrap_or_default(),
            }),
            identity: req.identity,
        }
    }
}

impl From<pb::RespondActivityTaskFailedResponse> for api::RespondActivityTaskFailedResponse {
    fn from(_: pb::RespondActivityTaskFailedResponse) -> Self {
        api::RespondActivityTaskFailedResponse {}
    }
}
