//! Conversions for shared types (WorkflowExecution, WorkflowType, ActivityType, TaskList, etc.)

use crate::generated as pb;
use crate::shared as api_types;

// ============================================================================
// Basic Shared Type Conversions
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

impl From<api_types::RetryPolicy> for pb::RetryPolicy {
    fn from(rp: api_types::RetryPolicy) -> Self {
        pb::RetryPolicy {
            initial_interval: super::helpers::seconds_to_duration(Some(
                rp.initial_interval_in_seconds,
            )),
            backoff_coefficient: rp.backoff_coefficient,
            maximum_interval: super::helpers::seconds_to_duration(Some(
                rp.maximum_interval_in_seconds,
            )),
            maximum_attempts: rp.maximum_attempts,
            non_retryable_error_reasons: rp.non_retryable_error_types,
            expiration_interval: super::helpers::seconds_to_duration(Some(
                rp.expiration_interval_in_seconds,
            )),
        }
    }
}
