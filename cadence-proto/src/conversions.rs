//! Type conversions between Thrift-generated types and internal types
//!
//! This module provides bidirectional conversions between the types generated
//! by the Thrift compiler and the hand-written types used throughout the codebase.
//! This allows gradual migration from hand-written types to generated types.

use crate::generated::{cadence, shared};
use crate::{shared as internal, workflow_service as internal_service};
use std::convert::{TryFrom, TryInto};
use std::time::Duration;

/// Error type for conversion failures
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid enum value: {0}")]
    InvalidEnum(String),

    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(i64),

    #[error("Conversion error: {0}")]
    Other(String),
}

/// Convert internal WorkflowExecution to thrift-generated WorkflowExecution
impl From<internal::WorkflowExecution> for shared::WorkflowExecution {
    fn from(exec: internal::WorkflowExecution) -> Self {
        shared::WorkflowExecution {
            workflowId: Some(exec.workflow_id),
            runId: exec.run_id,
        }
    }
}

/// Convert thrift-generated WorkflowExecution to internal WorkflowExecution
impl TryFrom<shared::WorkflowExecution> for internal::WorkflowExecution {
    type Error = ConversionError;

    fn try_from(exec: shared::WorkflowExecution) -> Result<Self, Self::Error> {
        Ok(internal::WorkflowExecution {
            workflow_id: exec
                .workflowId
                .ok_or(ConversionError::MissingField("workflowId"))?,
            run_id: exec.runId,
        })
    }
}

/// Convert internal WorkflowType to thrift-generated WorkflowType
impl From<internal::WorkflowType> for shared::WorkflowType {
    fn from(wt: internal::WorkflowType) -> Self {
        shared::WorkflowType {
            name: Some(wt.name),
        }
    }
}

/// Convert thrift-generated WorkflowType to internal WorkflowType
impl TryFrom<shared::WorkflowType> for internal::WorkflowType {
    type Error = ConversionError;

    fn try_from(wt: shared::WorkflowType) -> Result<Self, Self::Error> {
        Ok(internal::WorkflowType {
            name: wt.name.ok_or(ConversionError::MissingField("name"))?,
        })
    }
}

/// Convert internal ActivityType to thrift-generated ActivityType
impl From<internal::ActivityType> for shared::ActivityType {
    fn from(at: internal::ActivityType) -> Self {
        shared::ActivityType {
            name: Some(at.name),
        }
    }
}

/// Convert thrift-generated ActivityType to internal ActivityType
impl TryFrom<shared::ActivityType> for internal::ActivityType {
    type Error = ConversionError;

    fn try_from(at: shared::ActivityType) -> Result<Self, Self::Error> {
        Ok(internal::ActivityType {
            name: at.name.ok_or(ConversionError::MissingField("name"))?,
        })
    }
}

/// Convert internal TaskList to thrift-generated TaskList
impl From<internal::TaskList> for shared::TaskList {
    fn from(tl: internal::TaskList) -> Self {
        shared::TaskList {
            name: Some(tl.name),
            kind: tl.kind.map(|k| k.into()),
        }
    }
}

/// Convert thrift-generated TaskList to internal TaskList
impl TryFrom<shared::TaskList> for internal::TaskList {
    type Error = ConversionError;

    fn try_from(tl: shared::TaskList) -> Result<Self, Self::Error> {
        Ok(internal::TaskList {
            name: tl.name.ok_or(ConversionError::MissingField("name"))?,
            kind: tl.kind.map(TryInto::try_into).transpose()?,
        })
    }
}

/// Convert internal TaskListKind to thrift-generated TaskListKind
impl From<internal::TaskListKind> for shared::TaskListKind {
    fn from(kind: internal::TaskListKind) -> Self {
        match kind {
            internal::TaskListKind::Normal => shared::TaskListKind::NORMAL,
            internal::TaskListKind::Sticky => shared::TaskListKind::STICKY,
        }
    }
}

/// Convert thrift-generated TaskListKind to internal TaskListKind
impl TryFrom<shared::TaskListKind> for internal::TaskListKind {
    type Error = ConversionError;

    fn try_from(kind: shared::TaskListKind) -> Result<Self, Self::Error> {
        match kind {
            shared::TaskListKind::NORMAL => Ok(internal::TaskListKind::Normal),
            shared::TaskListKind::STICKY => Ok(internal::TaskListKind::Sticky),
            _ => Err(ConversionError::InvalidEnum(format!(
                "Unknown TaskListKind: {:?}",
                kind
            ))),
        }
    }
}

/// Convert internal EncodingType to thrift-generated EncodingType
impl From<internal::EncodingType> for shared::EncodingType {
    fn from(et: internal::EncodingType) -> Self {
        match et {
            internal::EncodingType::ThriftRw => shared::EncodingType::ThriftRW,
            internal::EncodingType::Proto3 => shared::EncodingType::JSON,
            internal::EncodingType::Json => shared::EncodingType::JSON,
            internal::EncodingType::Unknown => shared::EncodingType::ThriftRW,
        }
    }
}

/// Convert thrift-generated EncodingType to internal EncodingType
impl TryFrom<shared::EncodingType> for internal::EncodingType {
    type Error = ConversionError;

    fn try_from(et: shared::EncodingType) -> Result<Self, Self::Error> {
        match et {
            shared::EncodingType::ThriftRW => Ok(internal::EncodingType::ThriftRw),
            shared::EncodingType::JSON => Ok(internal::EncodingType::Json),
            _ => Err(ConversionError::InvalidEnum(format!(
                "Unknown EncodingType: {:?}",
                et
            ))),
        }
    }
}

/// Convert internal WorkflowIdReusePolicy to thrift-generated WorkflowIdReusePolicy
impl From<internal::WorkflowIdReusePolicy> for shared::WorkflowIdReusePolicy {
    fn from(policy: internal::WorkflowIdReusePolicy) -> Self {
        match policy {
            internal::WorkflowIdReusePolicy::AllowDuplicate => {
                shared::WorkflowIdReusePolicy::AllowDuplicate
            }
            internal::WorkflowIdReusePolicy::AllowDuplicateFailedOnly => {
                shared::WorkflowIdReusePolicy::AllowDuplicateFailedOnly
            }
            internal::WorkflowIdReusePolicy::RejectDuplicate => {
                shared::WorkflowIdReusePolicy::RejectDuplicate
            }
            internal::WorkflowIdReusePolicy::TerminateIfRunning => {
                shared::WorkflowIdReusePolicy::TerminateIfRunning
            }
        }
    }
}

/// Convert thrift-generated WorkflowIdReusePolicy to internal WorkflowIdReusePolicy
impl TryFrom<shared::WorkflowIdReusePolicy> for internal::WorkflowIdReusePolicy {
    type Error = ConversionError;

    fn try_from(policy: shared::WorkflowIdReusePolicy) -> Result<Self, Self::Error> {
        match policy {
            shared::WorkflowIdReusePolicy::AllowDuplicate => {
                Ok(internal::WorkflowIdReusePolicy::AllowDuplicate)
            }
            shared::WorkflowIdReusePolicy::AllowDuplicateFailedOnly => {
                Ok(internal::WorkflowIdReusePolicy::AllowDuplicateFailedOnly)
            }
            shared::WorkflowIdReusePolicy::RejectDuplicate => {
                Ok(internal::WorkflowIdReusePolicy::RejectDuplicate)
            }
            shared::WorkflowIdReusePolicy::TerminateIfRunning => {
                Ok(internal::WorkflowIdReusePolicy::TerminateIfRunning)
            }
            _ => Err(ConversionError::InvalidEnum(format!(
                "Unknown WorkflowIdReusePolicy: {:?}",
                policy
            ))),
        }
    }
}

/// Convert internal QueryConsistencyLevel to thrift-generated QueryConsistencyLevel
impl From<internal::QueryConsistencyLevel> for shared::QueryConsistencyLevel {
    fn from(level: internal::QueryConsistencyLevel) -> Self {
        match level {
            internal::QueryConsistencyLevel::Eventual => shared::QueryConsistencyLevel::EVENTUAL,
            internal::QueryConsistencyLevel::Strong => shared::QueryConsistencyLevel::STRONG,
        }
    }
}

/// Convert thrift-generated QueryConsistencyLevel to internal QueryConsistencyLevel
impl TryFrom<shared::QueryConsistencyLevel> for internal::QueryConsistencyLevel {
    type Error = ConversionError;

    fn try_from(level: shared::QueryConsistencyLevel) -> Result<Self, Self::Error> {
        match level {
            shared::QueryConsistencyLevel::EVENTUAL => {
                Ok(internal::QueryConsistencyLevel::Eventual)
            }
            shared::QueryConsistencyLevel::STRONG => Ok(internal::QueryConsistencyLevel::Strong),
            _ => Err(ConversionError::InvalidEnum(format!(
                "Unknown QueryConsistencyLevel: {:?}",
                level
            ))),
        }
    }
}

/// Convert internal ParentClosePolicy to thrift-generated ParentClosePolicy
impl From<internal::ParentClosePolicy> for shared::ParentClosePolicy {
    fn from(policy: internal::ParentClosePolicy) -> Self {
        match policy {
            internal::ParentClosePolicy::Abandon => shared::ParentClosePolicy::ABANDON,
            internal::ParentClosePolicy::RequestCancel => shared::ParentClosePolicy::REQUEST_CANCEL,
            internal::ParentClosePolicy::Terminate => shared::ParentClosePolicy::TERMINATE,
        }
    }
}

/// Convert thrift-generated ParentClosePolicy to internal ParentClosePolicy
impl TryFrom<shared::ParentClosePolicy> for internal::ParentClosePolicy {
    type Error = ConversionError;

    fn try_from(policy: shared::ParentClosePolicy) -> Result<Self, Self::Error> {
        match policy {
            shared::ParentClosePolicy::ABANDON => Ok(internal::ParentClosePolicy::Abandon),
            shared::ParentClosePolicy::REQUEST_CANCEL => {
                Ok(internal::ParentClosePolicy::RequestCancel)
            }
            shared::ParentClosePolicy::TERMINATE => Ok(internal::ParentClosePolicy::Terminate),
            _ => Err(ConversionError::InvalidEnum(format!(
                "Unknown ParentClosePolicy: {:?}",
                policy
            ))),
        }
    }
}

/// Convert internal TimeoutType to thrift-generated TimeoutType
impl From<internal::TimeoutType> for shared::TimeoutType {
    fn from(tt: internal::TimeoutType) -> Self {
        match tt {
            internal::TimeoutType::StartToClose => shared::TimeoutType::START_TO_CLOSE,
            internal::TimeoutType::ScheduleToStart => shared::TimeoutType::SCHEDULE_TO_START,
            internal::TimeoutType::ScheduleToClose => shared::TimeoutType::SCHEDULE_TO_CLOSE,
            internal::TimeoutType::Heartbeat => shared::TimeoutType::HEARTBEAT,
        }
    }
}

/// Convert thrift-generated TimeoutType to internal TimeoutType
impl TryFrom<shared::TimeoutType> for internal::TimeoutType {
    type Error = ConversionError;

    fn try_from(tt: shared::TimeoutType) -> Result<Self, Self::Error> {
        match tt {
            shared::TimeoutType::START_TO_CLOSE => Ok(internal::TimeoutType::StartToClose),
            shared::TimeoutType::SCHEDULE_TO_START => Ok(internal::TimeoutType::ScheduleToStart),
            shared::TimeoutType::SCHEDULE_TO_CLOSE => Ok(internal::TimeoutType::ScheduleToClose),
            shared::TimeoutType::HEARTBEAT => Ok(internal::TimeoutType::Heartbeat),
            _ => Err(ConversionError::InvalidEnum(format!(
                "Unknown TimeoutType: {:?}",
                tt
            ))),
        }
    }
}

/// Convert internal RetryPolicy to thrift-generated RetryPolicy
impl From<internal::RetryPolicy> for shared::RetryPolicy {
    fn from(policy: internal::RetryPolicy) -> Self {
        shared::RetryPolicy {
            initialIntervalInSeconds: policy.initial_interval.map(|d| d.as_secs() as i32),
            backoffCoefficient: policy.backoff_coefficient,
            maximumIntervalInSeconds: policy.maximum_interval.map(|d| d.as_secs() as i32),
            maximumAttempts: policy.maximum_attempts,
            expirationIntervalInSeconds: policy.expiration_interval.map(|d| d.as_secs() as i32),
            nonRetryableErrorTypes: policy.non_retryable_error_types,
        }
    }
}

/// Convert thrift-generated RetryPolicy to internal RetryPolicy
impl TryFrom<shared::RetryPolicy> for internal::RetryPolicy {
    type Error = ConversionError;

    fn try_from(policy: shared::RetryPolicy) -> Result<Self, Self::Error> {
        Ok(internal::RetryPolicy {
            initial_interval: policy
                .initialIntervalInSeconds
                .map(|s| Duration::from_secs(s as u64)),
            backoff_coefficient: policy.backoffCoefficient.unwrap_or(2.0),
            maximum_interval: policy
                .maximumIntervalInSeconds
                .map(|s| Duration::from_secs(s as u64)),
            maximum_attempts: policy.maximumAttempts,
            expiration_interval: policy
                .expirationIntervalInSeconds
                .map(|s| Duration::from_secs(s as u64)),
            non_retryable_error_types: policy.nonRetryableErrorTypes.unwrap_or_default(),
        })
    }
}

/// Convert internal HistoryEventFilterType to thrift-generated HistoryEventFilterType
impl From<internal::HistoryEventFilterType> for shared::HistoryEventFilterType {
    fn from(ft: internal::HistoryEventFilterType) -> Self {
        match ft {
            internal::HistoryEventFilterType::AllEvent => shared::HistoryEventFilterType::ALL_EVENT,
            internal::HistoryEventFilterType::CloseEvent => {
                shared::HistoryEventFilterType::CLOSE_EVENT
            }
        }
    }
}

/// Convert thrift-generated HistoryEventFilterType to internal HistoryEventFilterType
impl TryFrom<shared::HistoryEventFilterType> for internal::HistoryEventFilterType {
    type Error = ConversionError;

    fn try_from(ft: shared::HistoryEventFilterType) -> Result<Self, Self::Error> {
        match ft {
            shared::HistoryEventFilterType::ALL_EVENT => {
                Ok(internal::HistoryEventFilterType::AllEvent)
            }
            shared::HistoryEventFilterType::CLOSE_EVENT => {
                Ok(internal::HistoryEventFilterType::CloseEvent)
            }
            _ => Err(ConversionError::InvalidEnum(format!(
                "Unknown HistoryEventFilterType: {:?}",
                ft
            ))),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_execution_conversion() {
        let internal = internal::WorkflowExecution {
            workflow_id: "test-workflow".to_string(),
            run_id: Some("test-run".to_string()),
        };

        let thrift: shared::WorkflowExecution = internal.into();
        assert_eq!(thrift.workflowId, Some("test-workflow".to_string()));
        assert_eq!(thrift.runId, Some("test-run".to_string()));

        let converted_back: internal::WorkflowExecution = thrift.try_into().unwrap();
        assert_eq!(converted_back.workflow_id, "test-workflow");
        assert_eq!(converted_back.run_id, Some("test-run".to_string()));
    }

    #[test]
    fn test_workflow_type_conversion() {
        let internal = internal::WorkflowType {
            name: "TestWorkflow".to_string(),
        };

        let thrift: shared::WorkflowType = internal.into();
        assert_eq!(thrift.name, Some("TestWorkflow".to_string()));

        let converted_back: internal::WorkflowType = thrift.try_into().unwrap();
        assert_eq!(converted_back.name, "TestWorkflow");
    }

    #[test]
    fn test_task_list_conversion() {
        let internal = internal::TaskList {
            name: "test-task-list".to_string(),
            kind: Some(internal::TaskListKind::Normal),
        };

        let thrift: shared::TaskList = internal.into();
        assert_eq!(thrift.name, Some("test-task-list".to_string()));
        assert_eq!(thrift.kind, Some(shared::TaskListKind::NORMAL));

        let converted_back: internal::TaskList = thrift.try_into().unwrap();
        assert_eq!(converted_back.name, "test-task-list");
        assert_eq!(converted_back.kind, Some(internal::TaskListKind::Normal));
    }

    #[test]
    fn test_workflow_id_reuse_policy_conversion() {
        let internal = internal::WorkflowIdReusePolicy::AllowDuplicateFailedOnly;
        let thrift: shared::WorkflowIdReusePolicy = internal.into();
        assert_eq!(
            thrift,
            shared::WorkflowIdReusePolicy::AllowDuplicateFailedOnly
        );

        let converted_back: internal::WorkflowIdReusePolicy = thrift.try_into().unwrap();
        assert_eq!(
            converted_back,
            internal::WorkflowIdReusePolicy::AllowDuplicateFailedOnly
        );
    }

    #[test]
    fn test_missing_field_error() {
        let thrift = shared::WorkflowExecution {
            workflowId: None,
            runId: None,
        };

        let result: Result<internal::WorkflowExecution, ConversionError> = thrift.try_into();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ConversionError::MissingField("workflowId"))
        ));
    }
}
