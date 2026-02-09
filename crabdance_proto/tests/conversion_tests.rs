use crabdance_proto::generated::{self as pb};
use crabdance_proto::shared::{self as api};
use crabdance_proto::workflow_service::ResetPointInfo as ApiResetPointInfo;
use std::collections::HashMap;

#[test]
fn test_workflow_execution_started_conversion() {
    let pb_attributes = pb::WorkflowExecutionStartedEventAttributes {
        workflow_type: Some(pb::WorkflowType {
            name: "test-workflow".to_string(),
        }),
        parent_execution_info: Some(pb::ParentExecutionInfo {
            domain_id: "parent-domain".to_string(),
            domain_name: "parent-domain-name".to_string(),
            workflow_execution: Some(pb::WorkflowExecution {
                workflow_id: "parent-wf-id".to_string(),
                run_id: "parent-run-id".to_string(),
            }),
            initiated_id: 123,
        }),
        task_list: Some(pb::TaskList {
            name: "test-task-list".to_string(),
            kind: 0,
        }),
        input: Some(pb::Payload {
            data: b"test-input".to_vec(),
        }),
        first_scheduled_time: Some(prost_types::Timestamp {
            seconds: 100,
            nanos: 0,
        }),
        execution_start_to_close_timeout: Some(prost_types::Duration {
            seconds: 100,
            nanos: 0,
        }),
        task_start_to_close_timeout: Some(prost_types::Duration {
            seconds: 50,
            nanos: 0,
        }),
        identity: "test-identity".to_string(),
        continued_execution_run_id: "prev-run-id".to_string(),
        initiator: 2, // CronSchedule
        continued_failure: Some(pb::Failure {
            reason: "timeout".to_string(),
            details: b"logs".to_vec(),
        }),
        last_completion_result: Some(pb::Payload {
            data: b"last-result".to_vec(),
        }),
        original_execution_run_id: "orig-run-id".to_string(),
        first_execution_run_id: "first-run-id".to_string(),
        retry_policy: Some(pb::RetryPolicy {
            initial_interval: Some(prost_types::Duration {
                seconds: 1,
                nanos: 0,
            }),
            backoff_coefficient: 2.0,
            maximum_interval: Some(prost_types::Duration {
                seconds: 10,
                nanos: 0,
            }),
            maximum_attempts: 5,
            non_retryable_error_reasons: vec!["bad-error".to_string()],
            expiration_interval: Some(prost_types::Duration {
                seconds: 60,
                nanos: 0,
            }),
        }),
        attempt: 3,
        expiration_time: Some(prost_types::Timestamp {
            seconds: 999999999,
            nanos: 0,
        }),
        cron_schedule: "0 * * * *".to_string(),
        first_decision_task_backoff: Some(prost_types::Duration {
            seconds: 5,
            nanos: 0,
        }),
        header: None,
        search_attributes: None,
        memo: None,
        prev_auto_reset_points: None,
        partition_config: HashMap::new(),
        request_id: "".to_string(),
        cron_overlap_policy: 0,
        active_cluster_selection_policy: None,
    };

    let api_attributes = api::WorkflowExecutionStartedEventAttributes::from(pb_attributes.clone());

    assert_eq!(api_attributes.workflow_type.unwrap().name, "test-workflow");
    assert_eq!(
        api_attributes
            .parent_workflow_execution
            .as_ref()
            .unwrap()
            .workflow_id,
        "parent-wf-id"
    );
    assert_eq!(
        api_attributes
            .parent_workflow_execution
            .as_ref()
            .unwrap()
            .run_id,
        "parent-run-id"
    );
    assert_eq!(api_attributes.task_list.unwrap().name, "test-task-list");
    assert_eq!(api_attributes.input, b"test-input");
    assert_eq!(api_attributes.execution_start_to_close_timeout_seconds, 100);
    assert_eq!(api_attributes.task_start_to_close_timeout_seconds, 50);
    assert_eq!(api_attributes.identity, "test-identity");
    assert_eq!(
        api_attributes.continued_execution_run_id,
        Some("prev-run-id".to_string())
    );

    // Check Enum conversion
    match api_attributes.initiator {
        Some(api::ContinueAsNewInitiator::CronSchedule) => {}
        _ => panic!("Incorrect initiator conversion"),
    }

    assert_eq!(
        api_attributes.continued_failure_details,
        Some(b"logs".to_vec())
    );
    assert_eq!(
        api_attributes.last_completion_result,
        Some(b"last-result".to_vec())
    );
    assert_eq!(
        api_attributes.original_execution_run_id,
        Some("orig-run-id".to_string())
    );
    assert_eq!(
        api_attributes.first_execution_run_id,
        Some("first-run-id".to_string())
    );

    // Check Retry Policy
    let policy = api_attributes.retry_policy.unwrap();
    assert_eq!(policy.initial_interval_in_seconds, 1);
    assert_eq!(policy.backoff_coefficient, 2.0);
    assert_eq!(policy.maximum_interval_in_seconds, 10);
    assert_eq!(policy.maximum_attempts, 5);
    assert_eq!(
        policy.non_retryable_error_types,
        vec!["bad-error".to_string()]
    );
    assert_eq!(policy.expiration_interval_in_seconds, 60);

    assert_eq!(api_attributes.attempt, 3);
    assert_eq!(
        api_attributes.expiration_timestamp,
        Some(999_999_999_000_000_000)
    );
    assert_eq!(api_attributes.cron_schedule, Some("0 * * * *".to_string()));
    assert_eq!(api_attributes.first_decision_task_backoff_seconds, 5);
}

#[test]
fn test_child_workflow_execution_started_conversion() {
    let pb_attributes = pb::ChildWorkflowExecutionStartedEventAttributes {
        domain: "child-domain".to_string(),
        workflow_execution: Some(pb::WorkflowExecution {
            workflow_id: "child-wf-id".to_string(),
            run_id: "child-run-id".to_string(),
        }),
        workflow_type: Some(pb::WorkflowType {
            name: "child-workflow".to_string(),
        }),
        initiated_event_id: 10,
        header: None,
    };

    let api_attributes = api::ChildWorkflowExecutionStartedEventAttributes::from(pb_attributes);

    assert_eq!(
        api_attributes
            .workflow_execution
            .as_ref()
            .unwrap()
            .workflow_id,
        "child-wf-id"
    );
    assert_eq!(
        api_attributes.workflow_execution.as_ref().unwrap().run_id,
        "child-run-id"
    );
    assert_eq!(api_attributes.workflow_type.unwrap().name, "child-workflow");
    assert_eq!(api_attributes.initiated_event_id, 10);
}

#[test]
fn test_reset_point_info_conversion() {
    let pb_info = pb::ResetPointInfo {
        binary_checksum: "checksum-123".to_string(),
        run_id: "run-abc".to_string(),
        first_decision_completed_id: 50,
        created_time: Some(prost_types::Timestamp {
            seconds: 1600000000,
            nanos: 0,
        }),
        expiring_time: None,
        resettable: true,
    };

    let api_info = ApiResetPointInfo::from(pb_info);

    assert_eq!(api_info.binary_checksum, "checksum-123");
    assert_eq!(api_info.run_id, "run-abc");
    assert_eq!(api_info.first_decision_completed_id, 50);
    assert_eq!(api_info.created_time_nano, 1600000000000000000);
    assert_eq!(api_info.expiring_time_nano, 0);
    assert!(api_info.resettable);
}
