//! Conversion functions between hand-written API types and generated protobuf types.
//!
//! This module provides conversions (From/Into implementations) to bridge between
//! the clean hand-written API in `shared.rs` and `workflow_service.rs` with the
//! auto-generated protobuf types in `generated/`.

use crate::generated as pb;
use crate::shared as api_types;
use crate::workflow_service as api;

// ============================================================================
// Shared Type Conversions
// ============================================================================

impl From<api_types::WorkflowExecution> for pb::WorkflowExecution {
    fn from(we: api_types::WorkflowExecution) -> Self {
        pb::WorkflowExecution {
            workflow_id: we.workflow_id,
            run_id: we.run_id,
        }
    }
}

impl From<pb::WorkflowExecution> for api_types::WorkflowExecution {
    fn from(pb: pb::WorkflowExecution) -> Self {
        api_types::WorkflowExecution {
            workflow_id: pb.workflow_id,
            run_id: pb.run_id,
        }
    }
}

impl From<api_types::WorkflowType> for pb::WorkflowType {
    fn from(wt: api_types::WorkflowType) -> Self {
        pb::WorkflowType { name: wt.name }
    }
}

impl From<pb::WorkflowType> for api_types::WorkflowType {
    fn from(pb: pb::WorkflowType) -> Self {
        api_types::WorkflowType { name: pb.name }
    }
}

impl From<api_types::ActivityType> for pb::ActivityType {
    fn from(at: api_types::ActivityType) -> Self {
        pb::ActivityType { name: at.name }
    }
}

impl From<pb::ActivityType> for api_types::ActivityType {
    fn from(pb: pb::ActivityType) -> Self {
        api_types::ActivityType { name: pb.name }
    }
}

impl From<api_types::TaskList> for pb::TaskList {
    fn from(tl: api_types::TaskList) -> Self {
        pb::TaskList {
            name: tl.name,
            kind: tl.kind as i32,
        }
    }
}

impl From<pb::TaskList> for api_types::TaskList {
    fn from(pb: pb::TaskList) -> Self {
        api_types::TaskList {
            name: pb.name,
            kind: match pb.kind {
                1 => api_types::TaskListKind::Sticky,
                _ => api_types::TaskListKind::Normal,
            },
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert seconds (i32) to prost Duration
fn seconds_to_duration(seconds: Option<i32>) -> Option<prost_types::Duration> {
    seconds.map(|s| prost_types::Duration {
        seconds: s as i64,
        nanos: 0,
    })
}

/// Convert prost Duration to seconds (i32)
#[allow(dead_code)]
fn duration_to_seconds(duration: Option<prost_types::Duration>) -> Option<i32> {
    duration.map(|d| d.seconds as i32)
}

/// Convert Vec<u8> payload to protobuf Payload
fn bytes_to_payload(data: Option<Vec<u8>>) -> Option<pb::Payload> {
    data.map(|d| pb::Payload { data: d })
}

/// Convert protobuf Payload to Vec<u8>
#[allow(dead_code)]
fn payload_to_bytes(payload: Option<pb::Payload>) -> Option<Vec<u8>> {
    payload.map(|p| p.data)
}

/// Convert API Memo type to protobuf Memo
fn api_memo_to_pb(memo: api_types::Memo) -> pb::Memo {
    pb::Memo {
        fields: memo
            .fields
            .into_iter()
            .map(|(k, v)| (k, pb::Payload { data: v }))
            .collect(),
    }
}

/// Convert API SearchAttributes to protobuf SearchAttributes
fn api_search_attributes_to_pb(sa: api_types::SearchAttributes) -> pb::SearchAttributes {
    pb::SearchAttributes {
        indexed_fields: sa
            .indexed_fields
            .into_iter()
            .map(|(k, v)| (k, pb::Payload { data: v }))
            .collect(),
    }
}

/// Convert API Header to protobuf Header
fn api_header_to_pb(header: api_types::Header) -> pb::Header {
    pb::Header {
        fields: header
            .fields
            .into_iter()
            .map(|(k, v)| (k, pb::Payload { data: v }))
            .collect(),
    }
}

// ============================================================================
// Complex Type Conversions
// ============================================================================

impl From<api_types::RetryPolicy> for pb::RetryPolicy {
    fn from(rp: api_types::RetryPolicy) -> Self {
        pb::RetryPolicy {
            initial_interval: seconds_to_duration(Some(rp.initial_interval_in_seconds)),
            backoff_coefficient: rp.backoff_coefficient,
            maximum_interval: seconds_to_duration(Some(rp.maximum_interval_in_seconds)),
            maximum_attempts: rp.maximum_attempts,
            non_retryable_error_reasons: rp.non_retryable_error_types,
            expiration_interval: seconds_to_duration(Some(rp.expiration_interval_in_seconds)),
        }
    }
}

// ============================================================================
// Workflow Service Request/Response Conversions
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
// Activity Task Conversions
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

// ============================================================================
// Workflow Management Conversions
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

// ============================================================================
// Worker Polling Conversions
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
            history: resp.history.map(pb_history_to_api),
            next_page_token: Some(resp.next_page_token),
            query: resp.query.map(pb_workflow_query_to_api),
            workflow_execution_task_list: resp.workflow_execution_task_list.map(Into::into),
            scheduled_timestamp: resp.scheduled_time.and_then(timestamp_to_nanos),
            started_timestamp: resp.started_time.and_then(timestamp_to_nanos),
            queries: Some(
                resp.queries
                    .into_iter()
                    .map(|(k, v)| (k, pb_workflow_query_to_api(v)))
                    .collect(),
            ),
        }
    }
}

// ============================================================================
// Helper conversion functions for complex types
// ============================================================================

/// Convert protobuf Timestamp to nanoseconds
fn timestamp_to_nanos(ts: prost_types::Timestamp) -> Option<i64> {
    Some(ts.seconds * 1_000_000_000 + ts.nanos as i64)
}

/// Convert protobuf Header to API Header
fn pb_header_to_api(h: pb::Header) -> api_types::Header {
    api_types::Header {
        fields: h.fields.into_iter().map(|(k, v)| (k, v.data)).collect(),
    }
}

/// Convert protobuf History to API History
fn pb_history_to_api(h: pb::History) -> api_types::History {
    api_types::History {
        events: h.events.into_iter().map(pb_history_event_to_api).collect(),
    }
}

/// Convert protobuf HistoryEvent to API HistoryEvent
/// Note: This is a simplified conversion that only extracts basic event information.
/// Full conversion of all event attributes would require extensive pattern matching
/// on all possible event types. For now, we return None for attributes.
/// TODO: Implement full event attribute conversion when needed.
fn pb_history_event_to_api(e: pb::HistoryEvent) -> api_types::HistoryEvent {
    // Determine event type from the attributes variant
    let event_type = pb_attributes_to_event_type(&e.attributes);

    api_types::HistoryEvent {
        event_id: e.event_id,
        timestamp: e.event_time.and_then(timestamp_to_nanos).unwrap_or(0),
        event_type,
        version: e.version,
        task_id: e.task_id,
        attributes: e.attributes.and_then(pb_attributes_to_api_attributes),
    }
}

/// Convert protobuf Attributes to API EventAttributes
fn pb_attributes_to_api_attributes(
    attr: pb::history_event::Attributes,
) -> Option<api_types::EventAttributes> {
    use api_types::EventAttributes as ApiAttr;
    use pb::history_event::Attributes;

    match attr {
        Attributes::WorkflowExecutionStartedEventAttributes(a) => Some(
            ApiAttr::WorkflowExecutionStartedEventAttributes(Box::new(a.into())),
        ),
        Attributes::DecisionTaskScheduledEventAttributes(a) => Some(
            ApiAttr::DecisionTaskScheduledEventAttributes(Box::new(a.into())),
        ),
        // Add other conversions as needed
        _ => None,
    }
}

impl From<pb::WorkflowExecutionStartedEventAttributes>
    for api_types::WorkflowExecutionStartedEventAttributes
{
    fn from(pb: pb::WorkflowExecutionStartedEventAttributes) -> Self {
        api_types::WorkflowExecutionStartedEventAttributes {
            workflow_type: pb.workflow_type.map(Into::into),
            parent_workflow_execution: None, // Simplified
            task_list: pb.task_list.map(Into::into),
            input: payload_to_bytes(pb.input).unwrap_or_default(),
            execution_start_to_close_timeout_seconds: duration_to_seconds(
                pb.execution_start_to_close_timeout,
            )
            .unwrap_or(0),
            task_start_to_close_timeout_seconds: duration_to_seconds(
                pb.task_start_to_close_timeout,
            )
            .unwrap_or(0),
            identity: pb.identity,
            continued_execution_run_id: if pb.continued_execution_run_id.is_empty() {
                None
            } else {
                Some(pb.continued_execution_run_id)
            },
            initiator: None,                 // Simplified
            continued_failure_details: None, // Simplified
            last_completion_result: payload_to_bytes(pb.last_completion_result),
            original_execution_run_id: if pb.original_execution_run_id.is_empty() {
                None
            } else {
                Some(pb.original_execution_run_id)
            },
            first_execution_run_id: if pb.first_execution_run_id.is_empty() {
                None
            } else {
                Some(pb.first_execution_run_id)
            },
            retry_policy: None, // Simplified
            attempt: pb.attempt,
            expiration_timestamp: None, // Simplified
            cron_schedule: Some(pb.cron_schedule).filter(|s| !s.is_empty()),
            first_decision_task_backoff_seconds: duration_to_seconds(
                pb.first_decision_task_backoff,
            )
            .unwrap_or(0),
        }
    }
}

impl From<pb::DecisionTaskScheduledEventAttributes>
    for api_types::DecisionTaskScheduledEventAttributes
{
    fn from(pb: pb::DecisionTaskScheduledEventAttributes) -> Self {
        api_types::DecisionTaskScheduledEventAttributes {
            task_list: pb.task_list.map(Into::into),
            start_to_close_timeout_seconds: duration_to_seconds(pb.start_to_close_timeout)
                .unwrap_or(0),
            attempt: pb.attempt,
        }
    }
}

/// Determine EventType from protobuf history_event::Attributes oneof
fn pb_attributes_to_event_type(
    attr: &Option<pb::history_event::Attributes>,
) -> api_types::EventType {
    use pb::history_event::Attributes;

    match attr {
        Some(Attributes::WorkflowExecutionStartedEventAttributes(_)) => {
            api_types::EventType::WorkflowExecutionStarted
        }
        Some(Attributes::WorkflowExecutionCompletedEventAttributes(_)) => {
            api_types::EventType::WorkflowExecutionCompleted
        }
        Some(Attributes::WorkflowExecutionFailedEventAttributes(_)) => {
            api_types::EventType::WorkflowExecutionFailed
        }
        Some(Attributes::WorkflowExecutionTimedOutEventAttributes(_)) => {
            api_types::EventType::WorkflowExecutionTimedOut
        }
        Some(Attributes::DecisionTaskScheduledEventAttributes(_)) => {
            api_types::EventType::DecisionTaskScheduled
        }
        Some(Attributes::DecisionTaskStartedEventAttributes(_)) => {
            api_types::EventType::DecisionTaskStarted
        }
        Some(Attributes::DecisionTaskCompletedEventAttributes(_)) => {
            api_types::EventType::DecisionTaskCompleted
        }
        Some(Attributes::DecisionTaskTimedOutEventAttributes(_)) => {
            api_types::EventType::DecisionTaskTimedOut
        }
        Some(Attributes::DecisionTaskFailedEventAttributes(_)) => {
            api_types::EventType::DecisionTaskFailed
        }
        Some(Attributes::ActivityTaskScheduledEventAttributes(_)) => {
            api_types::EventType::ActivityTaskScheduled
        }
        Some(Attributes::ActivityTaskStartedEventAttributes(_)) => {
            api_types::EventType::ActivityTaskStarted
        }
        Some(Attributes::ActivityTaskCompletedEventAttributes(_)) => {
            api_types::EventType::ActivityTaskCompleted
        }
        Some(Attributes::ActivityTaskFailedEventAttributes(_)) => {
            api_types::EventType::ActivityTaskFailed
        }
        Some(Attributes::ActivityTaskTimedOutEventAttributes(_)) => {
            api_types::EventType::ActivityTaskTimedOut
        }
        Some(Attributes::ActivityTaskCancelRequestedEventAttributes(_)) => {
            api_types::EventType::ActivityTaskCancelRequested
        }
        Some(Attributes::ActivityTaskCanceledEventAttributes(_)) => {
            api_types::EventType::ActivityTaskCanceled
        }
        Some(Attributes::TimerStartedEventAttributes(_)) => api_types::EventType::TimerStarted,
        Some(Attributes::TimerFiredEventAttributes(_)) => api_types::EventType::TimerFired,
        Some(Attributes::TimerCanceledEventAttributes(_)) => api_types::EventType::TimerCanceled,
        Some(Attributes::WorkflowExecutionCancelRequestedEventAttributes(_)) => {
            api_types::EventType::WorkflowExecutionCancelRequested
        }
        Some(Attributes::WorkflowExecutionCanceledEventAttributes(_)) => {
            api_types::EventType::WorkflowExecutionCanceled
        }
        Some(Attributes::RequestCancelExternalWorkflowExecutionInitiatedEventAttributes(_)) => {
            api_types::EventType::RequestCancelExternalWorkflowExecutionInitiated
        }
        Some(Attributes::RequestCancelExternalWorkflowExecutionFailedEventAttributes(_)) => {
            api_types::EventType::RequestCancelExternalWorkflowExecutionFailed
        }
        Some(Attributes::ExternalWorkflowExecutionCancelRequestedEventAttributes(_)) => {
            api_types::EventType::ExternalWorkflowExecutionCancelRequested
        }
        Some(Attributes::MarkerRecordedEventAttributes(_)) => api_types::EventType::MarkerRecorded,
        Some(Attributes::WorkflowExecutionSignaledEventAttributes(_)) => {
            api_types::EventType::WorkflowExecutionSignaled
        }
        Some(Attributes::WorkflowExecutionTerminatedEventAttributes(_)) => {
            api_types::EventType::WorkflowExecutionTerminated
        }
        Some(Attributes::SignalExternalWorkflowExecutionInitiatedEventAttributes(_)) => {
            api_types::EventType::SignalExternalWorkflowExecutionInitiated
        }
        Some(Attributes::SignalExternalWorkflowExecutionFailedEventAttributes(_)) => {
            api_types::EventType::SignalExternalWorkflowExecutionFailed
        }
        Some(Attributes::ExternalWorkflowExecutionSignaledEventAttributes(_)) => {
            api_types::EventType::ExternalWorkflowExecutionSignaled
        }
        Some(Attributes::UpsertWorkflowSearchAttributesEventAttributes(_)) => {
            api_types::EventType::UpsertWorkflowSearchAttributes
        }
        Some(Attributes::StartChildWorkflowExecutionInitiatedEventAttributes(_)) => {
            api_types::EventType::StartChildWorkflowExecutionInitiated
        }
        Some(Attributes::ChildWorkflowExecutionStartedEventAttributes(_)) => {
            api_types::EventType::ChildWorkflowExecutionStarted
        }
        Some(Attributes::ChildWorkflowExecutionCompletedEventAttributes(_)) => {
            api_types::EventType::ChildWorkflowExecutionCompleted
        }
        Some(Attributes::ChildWorkflowExecutionFailedEventAttributes(_)) => {
            api_types::EventType::ChildWorkflowExecutionFailed
        }
        Some(Attributes::ChildWorkflowExecutionTimedOutEventAttributes(_)) => {
            api_types::EventType::ChildWorkflowExecutionTimedOut
        }
        Some(Attributes::ChildWorkflowExecutionCanceledEventAttributes(_)) => {
            api_types::EventType::ChildWorkflowExecutionCanceled
        }
        Some(Attributes::ChildWorkflowExecutionTerminatedEventAttributes(_)) => {
            api_types::EventType::ChildWorkflowExecutionTerminated
        }
        _ => api_types::EventType::WorkflowExecutionStarted, // Default fallback
    }
}

/// Convert protobuf WorkflowQuery to API WorkflowQuery
fn pb_workflow_query_to_api(q: pb::WorkflowQuery) -> api::WorkflowQuery {
    api::WorkflowQuery {
        query_type: q.query_type,
        query_args: payload_to_bytes(q.query_args),
    }
}

// ============================================================================
// Decision Conversions for RespondDecisionTaskCompleted
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
                    header: None,                  // Not in our API type
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

// ============================================================================
// Additional Workflow Operations
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

// SignalWithStartWorkflowExecutionResponse just returns a run_id, same as StartWorkflowExecutionResponse
impl From<pb::SignalWithStartWorkflowExecutionResponse> for api::StartWorkflowExecutionResponse {
    fn from(resp: pb::SignalWithStartWorkflowExecutionResponse) -> Self {
        api::StartWorkflowExecutionResponse {
            run_id: resp.run_id,
        }
    }
}

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
            history: resp.history.map(pb_history_to_api),
            next_page_token: Some(resp.next_page_token),
            archived: resp.archived,
        }
    }
}

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

fn pb_workflow_execution_info_to_api(
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

fn pb_memo_to_api(memo: pb::Memo) -> api_types::Memo {
    api_types::Memo {
        fields: memo.fields.into_iter().map(|(k, v)| (k, v.data)).collect(),
    }
}

fn pb_search_attributes_to_api(sa: pb::SearchAttributes) -> api_types::SearchAttributes {
    api_types::SearchAttributes {
        indexed_fields: sa
            .indexed_fields
            .into_iter()
            .map(|(k, v)| (k, v.data))
            .collect(),
    }
}

/// Convert nanoseconds to protobuf Timestamp
fn nanos_to_timestamp(nanos: i64) -> Option<prost_types::Timestamp> {
    Some(prost_types::Timestamp {
        seconds: nanos / 1_000_000_000,
        nanos: (nanos % 1_000_000_000) as i32,
    })
}

// ==================== List Open Workflow Executions ====================

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

// ==================== List Closed Workflow Executions ====================

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

// ==================== Register Domain ====================

impl From<api::RegisterDomainRequest> for pb::RegisterDomainRequest {
    fn from(req: api::RegisterDomainRequest) -> Self {
        // Determine if this is a global domain
        let is_global = req.is_global_domain.unwrap_or(false);

        // For local domains, ensure active_cluster_name is truly empty
        // Cadence's persistence layer has conditional checks that fail when
        // is_global_domain=false but active_cluster_name is set (even if empty string)
        let active_cluster_name = if is_global || !req.clusters.is_empty() {
            req.active_cluster_name
        } else {
            // For local domains without clusters, use empty string
            String::new()
        };

        pb::RegisterDomainRequest {
            security_token: req.security_token.unwrap_or_default(),
            name: req.name,
            description: req.description.unwrap_or_default(),
            owner_email: req.owner_email,
            workflow_execution_retention_period: seconds_to_duration(Some(
                req.workflow_execution_retention_period_in_days * 86400, // days to seconds
            )),
            clusters: req
                .clusters
                .into_iter()
                .map(|c| pb::ClusterReplicationConfiguration {
                    cluster_name: c.cluster_name,
                })
                .collect(),
            active_cluster_name,
            data: req.data,
            is_global_domain: is_global,
            history_archival_status: req.history_archival_status.map(|s| s as i32).unwrap_or(0),
            history_archival_uri: req.history_archival_uri.unwrap_or_default(),
            visibility_archival_status: req
                .visibility_archival_status
                .map(|s| s as i32)
                .unwrap_or(0),
            visibility_archival_uri: req.visibility_archival_uri.unwrap_or_default(),
            active_clusters: None,
            active_clusters_by_region: std::collections::HashMap::new(),
        }
    }
}

// ==================== Describe Domain ====================

impl From<api::DescribeDomainRequest> for pb::DescribeDomainRequest {
    fn from(req: api::DescribeDomainRequest) -> Self {
        let describe_by = if let Some(uuid) = req.uuid {
            Some(pb::describe_domain_request::DescribeBy::Id(uuid))
        } else {
            req.name.map(pb::describe_domain_request::DescribeBy::Name)
        };

        pb::DescribeDomainRequest { describe_by }
    }
}

impl From<pb::DescribeDomainResponse> for api::DescribeDomainResponse {
    fn from(resp: pb::DescribeDomainResponse) -> Self {
        if let Some(d) = resp.domain {
            let domain_info = Some(api::DomainInfo {
                name: d.name.clone(),
                status: Some(match d.status {
                    1 => api::DomainStatus::Deprecated,
                    2 => api::DomainStatus::Deleted,
                    _ => api::DomainStatus::Registered,
                }),
                description: d.description.clone(),
                owner_email: d.owner_email.clone(),
                data: d.data.clone(),
                uuid: d.id.clone(),
            });

            let configuration = Some(api::DomainConfiguration {
                workflow_execution_retention_period_in_days: duration_to_seconds(
                    d.workflow_execution_retention_period,
                )
                .unwrap_or(0)
                    / 86400, // seconds to days
                emit_metric: d.bad_binaries.is_some(),
                history_archival_status: Some(match d.history_archival_status {
                    1 => api::ArchivalStatus::Enabled,
                    _ => api::ArchivalStatus::Disabled,
                }),
                history_archival_uri: d.history_archival_uri.clone(),
                visibility_archival_status: Some(match d.visibility_archival_status {
                    1 => api::ArchivalStatus::Enabled,
                    _ => api::ArchivalStatus::Disabled,
                }),
                visibility_archival_uri: d.visibility_archival_uri.clone(),
            });

            let replication_configuration = Some(api::DomainReplicationConfiguration {
                active_cluster_name: d.active_cluster_name,
                clusters: d
                    .clusters
                    .into_iter()
                    .map(|c| api::ClusterReplicationConfiguration {
                        cluster_name: c.cluster_name,
                    })
                    .collect(),
            });

            api::DescribeDomainResponse {
                domain_info,
                configuration,
                replication_configuration,
            }
        } else {
            api::DescribeDomainResponse {
                domain_info: None,
                configuration: None,
                replication_configuration: None,
            }
        }
    }
}

// ==================== Update Domain ====================

impl From<api::UpdateDomainRequest> for pb::UpdateDomainRequest {
    fn from(req: api::UpdateDomainRequest) -> Self {
        let mut update_req = pb::UpdateDomainRequest {
            security_token: req.security_token.unwrap_or_default(),
            name: req.name.unwrap_or_default(),
            update_mask: None,
            description: String::new(),
            owner_email: String::new(),
            data: std::collections::HashMap::new(),
            workflow_execution_retention_period: None,
            bad_binaries: None,
            history_archival_status: 0,
            history_archival_uri: String::new(),
            visibility_archival_status: 0,
            visibility_archival_uri: String::new(),
            delete_bad_binary: req.delete_bad_binary.unwrap_or_default(),
            active_cluster_name: String::new(),
            clusters: Vec::new(),
            failover_timeout: None,
            active_clusters: None,
        };

        // Apply updated_info fields
        if let Some(info) = req.updated_info {
            update_req.description = info.description;
            update_req.owner_email = info.owner_email;
            update_req.data = info.data;
        }

        // Apply configuration fields
        if let Some(config) = req.configuration {
            update_req.workflow_execution_retention_period = seconds_to_duration(Some(
                config.workflow_execution_retention_period_in_days * 86400,
            ));
            update_req.history_archival_status = config
                .history_archival_status
                .map(|s| s as i32)
                .unwrap_or(0);
            update_req.history_archival_uri = config.history_archival_uri;
            update_req.visibility_archival_status = config
                .visibility_archival_status
                .map(|s| s as i32)
                .unwrap_or(0);
            update_req.visibility_archival_uri = config.visibility_archival_uri;
        }

        // Apply replication configuration
        if let Some(repl) = req.replication_configuration {
            update_req.active_cluster_name = repl.active_cluster_name;
            update_req.clusters = repl
                .clusters
                .into_iter()
                .map(|c| pb::ClusterReplicationConfiguration {
                    cluster_name: c.cluster_name,
                })
                .collect();
        }

        update_req
    }
}

impl From<pb::UpdateDomainResponse> for api::UpdateDomainResponse {
    fn from(resp: pb::UpdateDomainResponse) -> Self {
        if let Some(d) = resp.domain {
            let domain_info = Some(api::DomainInfo {
                name: d.name.clone(),
                status: Some(match d.status {
                    1 => api::DomainStatus::Deprecated,
                    2 => api::DomainStatus::Deleted,
                    _ => api::DomainStatus::Registered,
                }),
                description: d.description.clone(),
                owner_email: d.owner_email.clone(),
                data: d.data.clone(),
                uuid: d.id.clone(),
            });

            let configuration = Some(api::DomainConfiguration {
                workflow_execution_retention_period_in_days: duration_to_seconds(
                    d.workflow_execution_retention_period,
                )
                .unwrap_or(0)
                    / 86400,
                emit_metric: d.bad_binaries.is_some(),
                history_archival_status: Some(match d.history_archival_status {
                    1 => api::ArchivalStatus::Enabled,
                    _ => api::ArchivalStatus::Disabled,
                }),
                history_archival_uri: d.history_archival_uri.clone(),
                visibility_archival_status: Some(match d.visibility_archival_status {
                    1 => api::ArchivalStatus::Enabled,
                    _ => api::ArchivalStatus::Disabled,
                }),
                visibility_archival_uri: d.visibility_archival_uri.clone(),
            });

            let replication_configuration = Some(api::DomainReplicationConfiguration {
                active_cluster_name: d.active_cluster_name,
                clusters: d
                    .clusters
                    .into_iter()
                    .map(|c| api::ClusterReplicationConfiguration {
                        cluster_name: c.cluster_name,
                    })
                    .collect(),
            });

            api::UpdateDomainResponse {
                domain_info,
                configuration,
                replication_configuration,
            }
        } else {
            api::UpdateDomainResponse {
                domain_info: None,
                configuration: None,
                replication_configuration: None,
            }
        }
    }
}

// ==================== Failover Domain ====================

impl From<api::FailoverDomainRequest> for pb::FailoverDomainRequest {
    fn from(req: api::FailoverDomainRequest) -> Self {
        // Use the first cluster from the list as the active cluster name
        let active_cluster = req.clusters.first().cloned().unwrap_or_default();

        pb::FailoverDomainRequest {
            domain_name: req.name,
            domain_active_cluster_name: active_cluster,
            active_clusters: None,
            reason: String::new(),
        }
    }
}
