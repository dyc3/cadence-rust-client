//! Conversions for workflow history and events

use crate::generated as pb;
use crate::shared as api_types;
use std::convert::TryFrom;

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
fn pb_history_event_to_api(e: pb::HistoryEvent) -> api_types::HistoryEvent {
    api_types::HistoryEvent {
        event_id: e.event_id,
        timestamp: e.event_time.map(|t| t.seconds).unwrap_or(0),
        event_type: pb_attributes_to_event_type(&e.attributes),
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
        Attributes::WorkflowExecutionCompletedEventAttributes(a) => Some(
            ApiAttr::WorkflowExecutionCompletedEventAttributes(Box::new(a.into())),
        ),
        Attributes::WorkflowExecutionFailedEventAttributes(a) => Some(
            ApiAttr::WorkflowExecutionFailedEventAttributes(Box::new(a.into())),
        ),
        Attributes::WorkflowExecutionTimedOutEventAttributes(a) => Some(
            ApiAttr::WorkflowExecutionTimedOutEventAttributes(Box::new(a.into())),
        ),
        Attributes::DecisionTaskStartedEventAttributes(a) => Some(
            ApiAttr::DecisionTaskStartedEventAttributes(Box::new(a.into())),
        ),
        Attributes::DecisionTaskCompletedEventAttributes(a) => Some(
            ApiAttr::DecisionTaskCompletedEventAttributes(Box::new(a.into())),
        ),
        Attributes::TimerStartedEventAttributes(a) => {
            Some(ApiAttr::TimerStartedEventAttributes(Box::new(a.into())))
        }
        Attributes::TimerFiredEventAttributes(a) => {
            Some(ApiAttr::TimerFiredEventAttributes(Box::new(a.into())))
        }
        Attributes::TimerCanceledEventAttributes(a) => {
            Some(ApiAttr::TimerCanceledEventAttributes(Box::new(a.into())))
        }
        Attributes::WorkflowExecutionCancelRequestedEventAttributes(a) => Some(
            ApiAttr::WorkflowExecutionCancelRequestedEventAttributes(Box::new(a.into())),
        ),
        Attributes::WorkflowExecutionCanceledEventAttributes(a) => Some(
            ApiAttr::WorkflowExecutionCanceledEventAttributes(Box::new(a.into())),
        ),
        Attributes::MarkerRecordedEventAttributes(a) => {
            Some(ApiAttr::MarkerRecordedEventAttributes(Box::new(a.into())))
        }
        Attributes::WorkflowExecutionSignaledEventAttributes(a) => Some(
            ApiAttr::WorkflowExecutionSignaledEventAttributes(Box::new(a.into())),
        ),
        Attributes::SignalExternalWorkflowExecutionInitiatedEventAttributes(a) => Some(
            ApiAttr::SignalExternalWorkflowExecutionInitiatedEventAttributes(Box::new(a.into())),
        ),
        Attributes::SignalExternalWorkflowExecutionFailedEventAttributes(a) => {
            Some(ApiAttr::SignalExternalWorkflowExecutionFailedEventAttributes(Box::new(a.into())))
        }
        Attributes::ExternalWorkflowExecutionSignaledEventAttributes(a) => Some(
            ApiAttr::ExternalWorkflowExecutionSignaledEventAttributes(Box::new(a.into())),
        ),
        Attributes::RequestCancelExternalWorkflowExecutionInitiatedEventAttributes(a) => Some(
            ApiAttr::RequestCancelExternalWorkflowExecutionInitiatedEventAttributes(Box::new(
                a.into(),
            )),
        ),
        Attributes::RequestCancelExternalWorkflowExecutionFailedEventAttributes(a) => Some(
            ApiAttr::RequestCancelExternalWorkflowExecutionFailedEventAttributes(Box::new(
                a.into(),
            )),
        ),
        Attributes::ExternalWorkflowExecutionCancelRequestedEventAttributes(a) => Some(
            ApiAttr::ExternalWorkflowExecutionCancelRequestedEventAttributes(Box::new(a.into())),
        ),
        Attributes::StartChildWorkflowExecutionInitiatedEventAttributes(a) => {
            Some(ApiAttr::StartChildWorkflowExecutionInitiatedEventAttributes(Box::new(a.into())))
        }
        Attributes::ChildWorkflowExecutionStartedEventAttributes(a) => Some(
            ApiAttr::ChildWorkflowExecutionStartedEventAttributes(Box::new(a.into())),
        ),
        Attributes::ChildWorkflowExecutionCompletedEventAttributes(a) => Some(
            ApiAttr::ChildWorkflowExecutionCompletedEventAttributes(Box::new(a.into())),
        ),
        Attributes::ChildWorkflowExecutionFailedEventAttributes(a) => Some(
            ApiAttr::ChildWorkflowExecutionFailedEventAttributes(Box::new(a.into())),
        ),
        Attributes::ChildWorkflowExecutionCanceledEventAttributes(a) => Some(
            ApiAttr::ChildWorkflowExecutionCanceledEventAttributes(Box::new(a.into())),
        ),
        Attributes::ChildWorkflowExecutionTimedOutEventAttributes(a) => Some(
            ApiAttr::ChildWorkflowExecutionTimedOutEventAttributes(Box::new(a.into())),
        ),
        Attributes::ChildWorkflowExecutionTerminatedEventAttributes(a) => Some(
            ApiAttr::ChildWorkflowExecutionTerminatedEventAttributes(Box::new(a.into())),
        ),
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
        let initiator = match pb.initiator {
            0 => Some(api_types::ContinueAsNewInitiator::Decider),
            1 => Some(api_types::ContinueAsNewInitiator::Retry),
            2 => Some(api_types::ContinueAsNewInitiator::CronSchedule),
            3 => Some(api_types::ContinueAsNewInitiator::Workflow),
            _ => None,
        };

        api_types::WorkflowExecutionStartedEventAttributes {
            workflow_type: pb.workflow_type.map(Into::into),
            parent_workflow_execution: pb
                .parent_execution_info
                .and_then(|info| info.workflow_execution)
                .map(Into::into),
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
            initiator,
            continued_failure_details: failure_to_details(pb.continued_failure),
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
            retry_policy: pb.retry_policy.map(Into::into),
            attempt: pb.attempt,
            expiration_timestamp: pb.expiration_time.and_then(timestamp_to_nanos),
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
            retry_policy: pb.retry_policy.map(Into::into),
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
            last_failure_details: failure_to_details(pb.last_failure),
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
            details: payload_to_bytes(pb.details),
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

impl From<pb::WorkflowExecutionCompletedEventAttributes>
    for api_types::WorkflowExecutionCompletedEventAttributes
{
    fn from(pb: pb::WorkflowExecutionCompletedEventAttributes) -> Self {
        api_types::WorkflowExecutionCompletedEventAttributes {
            result: payload_to_bytes(pb.result),
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
        }
    }
}

impl From<pb::WorkflowExecutionFailedEventAttributes>
    for api_types::WorkflowExecutionFailedEventAttributes
{
    fn from(pb: pb::WorkflowExecutionFailedEventAttributes) -> Self {
        api_types::WorkflowExecutionFailedEventAttributes {
            reason: pb.failure.as_ref().map(|f| f.reason.clone()),
            details: failure_to_details(pb.failure),
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
        }
    }
}

impl From<pb::WorkflowExecutionTimedOutEventAttributes>
    for api_types::WorkflowExecutionTimedOutEventAttributes
{
    fn from(pb: pb::WorkflowExecutionTimedOutEventAttributes) -> Self {
        use api_types::TimeoutType;
        let timeout_type = match pb.timeout_type {
            1 => TimeoutType::StartToClose,
            _ => TimeoutType::StartToClose, // Default fallback
        };
        api_types::WorkflowExecutionTimedOutEventAttributes {
            timeout_type: Some(timeout_type),
        }
    }
}

impl From<pb::WorkflowExecutionCanceledEventAttributes>
    for api_types::WorkflowExecutionCanceledEventAttributes
{
    fn from(pb: pb::WorkflowExecutionCanceledEventAttributes) -> Self {
        api_types::WorkflowExecutionCanceledEventAttributes {
            details: payload_to_bytes(pb.details),
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
            identity: String::new(), // Not present in protobuf
        }
    }
}

impl From<pb::DecisionTaskStartedEventAttributes>
    for api_types::DecisionTaskStartedEventAttributes
{
    fn from(pb: pb::DecisionTaskStartedEventAttributes) -> Self {
        api_types::DecisionTaskStartedEventAttributes {
            scheduled_event_id: pb.scheduled_event_id,
            identity: pb.identity,
            request_id: pb.request_id,
        }
    }
}

impl From<pb::DecisionTaskCompletedEventAttributes>
    for api_types::DecisionTaskCompletedEventAttributes
{
    fn from(pb: pb::DecisionTaskCompletedEventAttributes) -> Self {
        api_types::DecisionTaskCompletedEventAttributes {
            scheduled_event_id: pb.scheduled_event_id,
            started_event_id: pb.started_event_id,
            identity: pb.identity,
            binary_checksum: pb.binary_checksum,
        }
    }
}

impl From<pb::TimerStartedEventAttributes> for api_types::TimerStartedEventAttributes {
    fn from(pb: pb::TimerStartedEventAttributes) -> Self {
        api_types::TimerStartedEventAttributes {
            timer_id: pb.timer_id,
            start_to_fire_timeout_seconds: duration_to_seconds(pb.start_to_fire_timeout)
                .unwrap_or(0) as i64,
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
        }
    }
}

impl From<pb::TimerFiredEventAttributes> for api_types::TimerFiredEventAttributes {
    fn from(pb: pb::TimerFiredEventAttributes) -> Self {
        api_types::TimerFiredEventAttributes {
            timer_id: pb.timer_id,
            started_event_id: pb.started_event_id,
        }
    }
}

impl From<pb::MarkerRecordedEventAttributes> for api_types::MarkerRecordedEventAttributes {
    fn from(pb: pb::MarkerRecordedEventAttributes) -> Self {
        api_types::MarkerRecordedEventAttributes {
            marker_name: pb.marker_name,
            details: payload_to_bytes(pb.details),
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
            header: pb.header.map(pb_header_to_api),
        }
    }
}

impl From<pb::SignalExternalWorkflowExecutionInitiatedEventAttributes>
    for api_types::SignalExternalWorkflowExecutionInitiatedEventAttributes
{
    fn from(pb: pb::SignalExternalWorkflowExecutionInitiatedEventAttributes) -> Self {
        api_types::SignalExternalWorkflowExecutionInitiatedEventAttributes {
            workflow_id: pb
                .workflow_execution
                .as_ref()
                .map(|we| we.workflow_id.clone())
                .unwrap_or_default(),
            run_id: pb
                .workflow_execution
                .as_ref()
                .map(|we| we.run_id.clone())
                .filter(|s| !s.is_empty()),
            signal_name: pb.signal_name,
            input: payload_to_bytes(pb.input),
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
            control: if pb.control.is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(&pb.control).into_owned())
            },
            child_workflow_only: pb.child_workflow_only,
            domain: if pb.domain.is_empty() {
                None
            } else {
                Some(pb.domain)
            },
        }
    }
}

impl From<pb::SignalExternalWorkflowExecutionFailedEventAttributes>
    for api_types::SignalExternalWorkflowExecutionFailedEventAttributes
{
    fn from(pb: pb::SignalExternalWorkflowExecutionFailedEventAttributes) -> Self {
        let cause_enum = pb::SignalExternalWorkflowExecutionFailedCause::try_from(pb.cause)
            .unwrap_or(pb::SignalExternalWorkflowExecutionFailedCause::Invalid);

        api_types::SignalExternalWorkflowExecutionFailedEventAttributes {
            cause: Some(cause_enum.as_str_name().to_string()),
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
            initiated_event_id: pb.initiated_event_id,
            workflow_execution: pb.workflow_execution.map(Into::into),
            control: if pb.control.is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(&pb.control).into_owned())
            },
        }
    }
}

impl From<pb::ExternalWorkflowExecutionSignaledEventAttributes>
    for api_types::ExternalWorkflowExecutionSignaledEventAttributes
{
    fn from(pb: pb::ExternalWorkflowExecutionSignaledEventAttributes) -> Self {
        api_types::ExternalWorkflowExecutionSignaledEventAttributes {
            initiated_event_id: pb.initiated_event_id,
            domain: if pb.domain.is_empty() {
                None
            } else {
                Some(pb.domain)
            },
            workflow_execution: pb.workflow_execution.map(Into::into),
            control: if pb.control.is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(&pb.control).into_owned())
            },
        }
    }
}

impl From<pb::RequestCancelExternalWorkflowExecutionInitiatedEventAttributes>
    for api_types::RequestCancelExternalWorkflowExecutionInitiatedEventAttributes
{
    fn from(pb: pb::RequestCancelExternalWorkflowExecutionInitiatedEventAttributes) -> Self {
        api_types::RequestCancelExternalWorkflowExecutionInitiatedEventAttributes {
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
            domain: if pb.domain.is_empty() {
                None
            } else {
                Some(pb.domain)
            },
            workflow_execution: pb.workflow_execution.map(Into::into),
            control: if pb.control.is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(&pb.control).into_owned())
            },
            child_workflow_only: pb.child_workflow_only,
        }
    }
}

impl From<pb::RequestCancelExternalWorkflowExecutionFailedEventAttributes>
    for api_types::RequestCancelExternalWorkflowExecutionFailedEventAttributes
{
    fn from(pb: pb::RequestCancelExternalWorkflowExecutionFailedEventAttributes) -> Self {
        let cause_enum = pb::CancelExternalWorkflowExecutionFailedCause::try_from(pb.cause)
            .unwrap_or(pb::CancelExternalWorkflowExecutionFailedCause::Invalid);

        api_types::RequestCancelExternalWorkflowExecutionFailedEventAttributes {
            cause: Some(cause_enum.as_str_name().to_string()),
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
            initiated_event_id: pb.initiated_event_id,
            workflow_execution: pb.workflow_execution.map(Into::into),
            control: if pb.control.is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(&pb.control).into_owned())
            },
        }
    }
}

impl From<pb::StartChildWorkflowExecutionInitiatedEventAttributes>
    for api_types::StartChildWorkflowExecutionInitiatedEventAttributes
{
    fn from(pb: pb::StartChildWorkflowExecutionInitiatedEventAttributes) -> Self {
        let workflow_id_reuse_policy = match pb.workflow_id_reuse_policy {
            0 => api_types::WorkflowIdReusePolicy::AllowDuplicateFailedOnly,
            1 => api_types::WorkflowIdReusePolicy::AllowDuplicate,
            2 => api_types::WorkflowIdReusePolicy::RejectDuplicate,
            3 => api_types::WorkflowIdReusePolicy::TerminateIfRunning,
            _ => api_types::WorkflowIdReusePolicy::AllowDuplicateFailedOnly,
        };

        let parent_close_policy = match pb.parent_close_policy {
            0 => api_types::ParentClosePolicy::Terminate,
            1 => api_types::ParentClosePolicy::RequestCancel,
            2 => api_types::ParentClosePolicy::Abandon,
            _ => api_types::ParentClosePolicy::Terminate,
        };

        api_types::StartChildWorkflowExecutionInitiatedEventAttributes {
            domain: pb.domain,
            workflow_id: pb.workflow_id,
            workflow_type: pb.workflow_type.map(Into::into),
            task_list: pb.task_list.map(Into::into),
            input: payload_to_bytes(pb.input),
            execution_start_to_close_timeout_seconds: duration_to_seconds(
                pb.execution_start_to_close_timeout,
            )
            .unwrap_or(0),
            task_start_to_close_timeout_seconds: duration_to_seconds(
                pb.task_start_to_close_timeout,
            )
            .unwrap_or(0),
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
            workflow_id_reuse_policy: Some(workflow_id_reuse_policy),
            retry_policy: pb.retry_policy.map(Into::into),
            cron_schedule: Some(pb.cron_schedule).filter(|s| !s.is_empty()),
            header: pb.header.map(pb_header_to_api),
            parent_close_policy: Some(parent_close_policy),
            control: if pb.control.is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(&pb.control).into_owned())
            },
        }
    }
}

impl From<pb::ChildWorkflowExecutionStartedEventAttributes>
    for api_types::ChildWorkflowExecutionStartedEventAttributes
{
    fn from(pb: pb::ChildWorkflowExecutionStartedEventAttributes) -> Self {
        api_types::ChildWorkflowExecutionStartedEventAttributes {
            workflow_execution: pb.workflow_execution.map(Into::into),
            workflow_type: pb.workflow_type.map(Into::into),
            initiated_event_id: pb.initiated_event_id,
        }
    }
}

impl From<pb::ChildWorkflowExecutionCompletedEventAttributes>
    for api_types::ChildWorkflowExecutionCompletedEventAttributes
{
    fn from(pb: pb::ChildWorkflowExecutionCompletedEventAttributes) -> Self {
        api_types::ChildWorkflowExecutionCompletedEventAttributes {
            workflow_execution: pb.workflow_execution.map(Into::into),
            workflow_type: pb.workflow_type.map(Into::into),
            result: payload_to_bytes(pb.result),
            initiated_event_id: pb.initiated_event_id,
            started_event_id: pb.started_event_id,
        }
    }
}

impl From<pb::ChildWorkflowExecutionFailedEventAttributes>
    for api_types::ChildWorkflowExecutionFailedEventAttributes
{
    fn from(pb: pb::ChildWorkflowExecutionFailedEventAttributes) -> Self {
        api_types::ChildWorkflowExecutionFailedEventAttributes {
            workflow_execution: pb.workflow_execution.map(Into::into),
            workflow_type: pb.workflow_type.map(Into::into),
            reason: pb.failure.as_ref().map(|f| f.reason.clone()),
            details: failure_to_details(pb.failure),
            initiated_event_id: pb.initiated_event_id,
            started_event_id: pb.started_event_id,
        }
    }
}

impl From<pb::ChildWorkflowExecutionCanceledEventAttributes>
    for api_types::ChildWorkflowExecutionCanceledEventAttributes
{
    fn from(pb: pb::ChildWorkflowExecutionCanceledEventAttributes) -> Self {
        api_types::ChildWorkflowExecutionCanceledEventAttributes {
            workflow_execution: pb.workflow_execution.map(Into::into),
            workflow_type: pb.workflow_type.map(Into::into),
            details: payload_to_bytes(pb.details),
            initiated_event_id: pb.initiated_event_id,
            started_event_id: pb.started_event_id,
            domain: if pb.domain.is_empty() {
                None
            } else {
                Some(pb.domain)
            },
        }
    }
}

impl From<pb::ChildWorkflowExecutionTimedOutEventAttributes>
    for api_types::ChildWorkflowExecutionTimedOutEventAttributes
{
    fn from(pb: pb::ChildWorkflowExecutionTimedOutEventAttributes) -> Self {
        use api_types::TimeoutType;
        let timeout_type = match pb.timeout_type {
            1 => TimeoutType::StartToClose,
            2 => TimeoutType::ScheduleToStart,
            3 => TimeoutType::ScheduleToClose,
            4 => TimeoutType::Heartbeat,
            _ => TimeoutType::StartToClose,
        };

        api_types::ChildWorkflowExecutionTimedOutEventAttributes {
            workflow_execution: pb.workflow_execution.map(Into::into),
            workflow_type: pb.workflow_type.map(Into::into),
            timeout_type,
            initiated_event_id: pb.initiated_event_id,
            started_event_id: pb.started_event_id,
            domain: if pb.domain.is_empty() {
                None
            } else {
                Some(pb.domain)
            },
        }
    }
}

impl From<pb::ChildWorkflowExecutionTerminatedEventAttributes>
    for api_types::ChildWorkflowExecutionTerminatedEventAttributes
{
    fn from(pb: pb::ChildWorkflowExecutionTerminatedEventAttributes) -> Self {
        api_types::ChildWorkflowExecutionTerminatedEventAttributes {
            workflow_execution: pb.workflow_execution.map(Into::into),
            workflow_type: pb.workflow_type.map(Into::into),
            initiated_event_id: pb.initiated_event_id,
            started_event_id: pb.started_event_id,
            domain: if pb.domain.is_empty() {
                None
            } else {
                Some(pb.domain)
            },
        }
    }
}

impl From<pb::TimerCanceledEventAttributes> for api_types::TimerCanceledEventAttributes {
    fn from(pb: pb::TimerCanceledEventAttributes) -> Self {
        api_types::TimerCanceledEventAttributes {
            timer_id: pb.timer_id,
            started_event_id: pb.started_event_id,
            decision_task_completed_event_id: pb.decision_task_completed_event_id,
            identity: pb.identity,
        }
    }
}

impl From<pb::WorkflowExecutionCancelRequestedEventAttributes>
    for api_types::WorkflowExecutionCancelRequestedEventAttributes
{
    fn from(pb: pb::WorkflowExecutionCancelRequestedEventAttributes) -> Self {
        let external_workflow_execution = pb
            .external_execution_info
            .as_ref()
            .and_then(|info| info.workflow_execution.clone())
            .map(Into::into);

        api_types::WorkflowExecutionCancelRequestedEventAttributes {
            cause: Some(pb.cause).filter(|c| !c.is_empty()),
            external_initiated_execution: None, // Not present in protobuf
            external_workflow_execution,
            identity: pb.identity,
        }
    }
}

impl From<pb::WorkflowExecutionSignaledEventAttributes>
    for api_types::WorkflowExecutionSignaledEventAttributes
{
    fn from(pb: pb::WorkflowExecutionSignaledEventAttributes) -> Self {
        api_types::WorkflowExecutionSignaledEventAttributes {
            signal_name: pb.signal_name,
            input: payload_to_bytes(pb.input),
            identity: pb.identity,
        }
    }
}

impl From<pb::ExternalWorkflowExecutionCancelRequestedEventAttributes>
    for api_types::ExternalWorkflowExecutionCancelRequestedEventAttributes
{
    fn from(pb: pb::ExternalWorkflowExecutionCancelRequestedEventAttributes) -> Self {
        api_types::ExternalWorkflowExecutionCancelRequestedEventAttributes {
            initiated_event_id: pb.initiated_event_id,
            domain: if pb.domain.is_empty() {
                None
            } else {
                Some(pb.domain)
            },
            workflow_execution: pb.workflow_execution.map(Into::into),
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

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_workflow_execution_completed_conversion() {
        let pb = pb::WorkflowExecutionCompletedEventAttributes {
            result: Some(pb::Payload {
                data: b"test_result".to_vec(),
            }),
            decision_task_completed_event_id: 123,
        };

        let api: api_types::WorkflowExecutionCompletedEventAttributes = pb.into();

        assert_eq!(api.result, Some(b"test_result".to_vec()));
        assert_eq!(api.decision_task_completed_event_id, 123);
    }

    #[test]
    fn test_timer_started_conversion() {
        let pb = pb::TimerStartedEventAttributes {
            timer_id: "timer-1".to_string(),
            start_to_fire_timeout: Some(prost_types::Duration {
                seconds: 5,
                nanos: 0,
            }),
            decision_task_completed_event_id: 456,
        };

        let api: api_types::TimerStartedEventAttributes = pb.into();

        assert_eq!(api.timer_id, "timer-1");
        assert_eq!(api.start_to_fire_timeout_seconds, 5);
        assert_eq!(api.decision_task_completed_event_id, 456);
    }

    #[test]
    fn test_marker_recorded_conversion() {
        let mut header_fields = HashMap::new();
        header_fields.insert(
            "key1".to_string(),
            pb::Payload {
                data: b"val1".to_vec(),
            },
        );

        let pb = pb::MarkerRecordedEventAttributes {
            marker_name: "test-marker".to_string(),
            details: Some(pb::Payload {
                data: b"details".to_vec(),
            }),
            decision_task_completed_event_id: 789,
            header: Some(pb::Header {
                fields: header_fields,
            }),
        };

        let api: api_types::MarkerRecordedEventAttributes = pb.into();

        assert_eq!(api.marker_name, "test-marker");
        assert_eq!(api.details, Some(b"details".to_vec()));
        assert_eq!(api.decision_task_completed_event_id, 789);

        let header = api.header.unwrap();
        assert_eq!(header.fields.get("key1").unwrap(), &b"val1".to_vec());
    }

    #[test]
    fn test_start_child_workflow_initiated_conversion() {
        let pb = pb::StartChildWorkflowExecutionInitiatedEventAttributes {
            domain: "test-domain".to_string(),
            workflow_id: "child-wf-1".to_string(),
            workflow_type: Some(pb::WorkflowType {
                name: "ChildType".to_string(),
            }),
            task_list: Some(pb::TaskList {
                name: "child-task-list".to_string(),
                kind: 0,
            }),
            input: Some(pb::Payload {
                data: b"input".to_vec(),
            }),
            execution_start_to_close_timeout: Some(prost_types::Duration {
                seconds: 100,
                nanos: 0,
            }),
            task_start_to_close_timeout: Some(prost_types::Duration {
                seconds: 200,
                nanos: 0,
            }),
            decision_task_completed_event_id: 321,
            workflow_id_reuse_policy: 2, // RejectDuplicate
            retry_policy: None,
            cron_schedule: "0 * * * *".to_string(),
            header: None,
            parent_close_policy: 1, // RequestCancel
            control: b"control-data".to_vec(),
            memo: None,
            search_attributes: None,
            delay_start: None,
            jitter_start: None,
            first_run_at: None,
            cron_overlap_policy: 0,
            active_cluster_selection_policy: None,
        };

        let api: api_types::StartChildWorkflowExecutionInitiatedEventAttributes = pb.into();

        assert_eq!(api.domain, "test-domain");
        assert_eq!(api.workflow_id, "child-wf-1");
        assert_eq!(api.workflow_type.unwrap().name, "ChildType");
        assert_eq!(api.task_list.unwrap().name, "child-task-list");
        assert_eq!(api.input, Some(b"input".to_vec()));
        assert_eq!(api.execution_start_to_close_timeout_seconds, 100);
        assert_eq!(api.task_start_to_close_timeout_seconds, 200);
        assert_eq!(
            api.workflow_id_reuse_policy,
            Some(api_types::WorkflowIdReusePolicy::RejectDuplicate)
        );
        assert_eq!(api.cron_schedule, Some("0 * * * *".to_string()));
        assert_eq!(
            api.parent_close_policy,
            Some(api_types::ParentClosePolicy::RequestCancel)
        );
        assert_eq!(api.control, Some("control-data".to_string()));
    }

    #[test]
    fn test_attributes_to_event_type() {
        // Test a few event type mappings
        let attr = Some(
            pb::history_event::Attributes::WorkflowExecutionStartedEventAttributes(
                pb::WorkflowExecutionStartedEventAttributes::default(),
            ),
        );
        assert_eq!(
            pb_attributes_to_event_type(&attr),
            api_types::EventType::WorkflowExecutionStarted
        );

        let attr = Some(
            pb::history_event::Attributes::ActivityTaskFailedEventAttributes(
                pb::ActivityTaskFailedEventAttributes::default(),
            ),
        );
        assert_eq!(
            pb_attributes_to_event_type(&attr),
            api_types::EventType::ActivityTaskFailed
        );

        let attr = Some(pb::history_event::Attributes::TimerFiredEventAttributes(
            pb::TimerFiredEventAttributes::default(),
        ));
        assert_eq!(
            pb_attributes_to_event_type(&attr),
            api_types::EventType::TimerFired
        );
    }
}
