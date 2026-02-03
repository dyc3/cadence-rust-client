//! Conversions for workflow history and events

use crate::generated as pb;
use crate::shared as api_types;

use super::helpers::*;

// ============================================================================
// History Conversions
// ============================================================================

/// Convert protobuf History to API History
pub(super) fn pb_history_to_api(h: pb::History) -> api_types::History {
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
        Attributes::ActivityTaskScheduledEventAttributes(a) => Some(
            ApiAttr::ActivityTaskScheduledEventAttributes(Box::new(a.into())),
        ),
        Attributes::ActivityTaskStartedEventAttributes(a) => Some(
            ApiAttr::ActivityTaskStartedEventAttributes(Box::new(a.into())),
        ),
        Attributes::ActivityTaskCompletedEventAttributes(a) => Some(
            ApiAttr::ActivityTaskCompletedEventAttributes(Box::new(a.into())),
        ),
        Attributes::ActivityTaskFailedEventAttributes(a) => Some(
            ApiAttr::ActivityTaskFailedEventAttributes(Box::new(a.into())),
        ),
        Attributes::ActivityTaskTimedOutEventAttributes(a) => Some(
            ApiAttr::ActivityTaskTimedOutEventAttributes(Box::new(a.into())),
        ),
        // Add other conversions as needed
        _ => None,
    }
}

// ============================================================================
// Event Attribute Conversions
// ============================================================================

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

impl From<pb::ActivityTaskScheduledEventAttributes>
    for api_types::ActivityTaskScheduledEventAttributes
{
    fn from(pb: pb::ActivityTaskScheduledEventAttributes) -> Self {
        api_types::ActivityTaskScheduledEventAttributes {
            activity_id: pb.activity_id,
            activity_type: pb.activity_type.map(Into::into),
            task_list: pb.task_list.map(Into::into),
            input: payload_to_bytes(pb.input),
            schedule_to_close_timeout_seconds: duration_to_seconds(pb.schedule_to_close_timeout),
            schedule_to_start_timeout_seconds: duration_to_seconds(pb.schedule_to_start_timeout),
            start_to_close_timeout_seconds: duration_to_seconds(pb.start_to_close_timeout),
            heartbeat_timeout_seconds: duration_to_seconds(pb.heartbeat_timeout),
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
            retry_policy: None, // Simplified - can be expanded if needed
        }
    }
}

impl From<pb::ActivityTaskStartedEventAttributes>
    for api_types::ActivityTaskStartedEventAttributes
{
    fn from(pb: pb::ActivityTaskStartedEventAttributes) -> Self {
        api_types::ActivityTaskStartedEventAttributes {
            scheduled_event_id: pb.scheduled_event_id,
            identity: pb.identity,
            request_id: pb.request_id,
            attempt: pb.attempt,
            last_failure_details: None, // Simplified - pb.last_failure is Failure type
        }
    }
}

impl From<pb::ActivityTaskCompletedEventAttributes>
    for api_types::ActivityTaskCompletedEventAttributes
{
    fn from(pb: pb::ActivityTaskCompletedEventAttributes) -> Self {
        api_types::ActivityTaskCompletedEventAttributes {
            result: payload_to_bytes(pb.result),
            scheduled_event_id: pb.scheduled_event_id,
            started_event_id: pb.started_event_id,
            identity: pb.identity,
        }
    }
}

impl From<pb::ActivityTaskFailedEventAttributes> for api_types::ActivityTaskFailedEventAttributes {
    fn from(pb: pb::ActivityTaskFailedEventAttributes) -> Self {
        api_types::ActivityTaskFailedEventAttributes {
            reason: pb.failure.as_ref().map(|f| f.reason.clone()),
            details: pb.failure.as_ref().map(|f| f.details.clone()),
            scheduled_event_id: pb.scheduled_event_id,
            started_event_id: pb.started_event_id,
            identity: pb.identity,
        }
    }
}

impl From<pb::ActivityTaskTimedOutEventAttributes>
    for api_types::ActivityTaskTimedOutEventAttributes
{
    fn from(pb: pb::ActivityTaskTimedOutEventAttributes) -> Self {
        use api_types::TimeoutType;

        let timeout_type = match pb.timeout_type {
            1 => TimeoutType::StartToClose,
            2 => TimeoutType::ScheduleToStart,
            3 => TimeoutType::ScheduleToClose,
            4 => TimeoutType::Heartbeat,
            _ => TimeoutType::StartToClose, // Default fallback
        };

        api_types::ActivityTaskTimedOutEventAttributes {
            details: None, // pb.details is Failure type, not Vec<u8>
            scheduled_event_id: pb.scheduled_event_id,
            started_event_id: pb.started_event_id,
            timeout_type,
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

// ============================================================================
// Event Type Mapping
// ============================================================================

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
