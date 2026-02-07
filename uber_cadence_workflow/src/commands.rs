use crate::context::LocalActivityOptions;
use std::time::Duration;
use uber_cadence_core::{ActivityOptions, ChildWorkflowOptions};

/// Workflow command
#[derive(Debug)]
pub enum WorkflowCommand {
    ScheduleActivity(ScheduleActivityCommand),
    ScheduleLocalActivity(ScheduleLocalActivityCommand),
    StartChildWorkflow(StartChildWorkflowCommand),
    StartTimer(StartTimerCommand),
    CancelTimer(CancelTimerCommand),
    CompleteWorkflow(CompleteWorkflowCommand),
    FailWorkflow(FailWorkflowCommand),
    CancelWorkflow(CancelWorkflowCommand),
    ContinueAsNewWorkflow(ContinueAsNewWorkflowCommand),
    SignalExternalWorkflow(SignalExternalWorkflowCommand),
    RequestCancelExternalWorkflow(RequestCancelExternalWorkflowCommand),
    RecordMarker(RecordMarkerCommand),
}

#[derive(Debug)]
pub struct ScheduleActivityCommand {
    pub activity_id: String,
    pub activity_type: String,
    pub args: Option<Vec<u8>>,
    pub options: ActivityOptions,
}

#[derive(Debug)]
pub struct ScheduleLocalActivityCommand {
    pub activity_id: String,
    pub activity_type: String,
    pub args: Option<Vec<u8>>,
    pub options: LocalActivityOptions,
}

#[derive(Debug)]
pub struct StartChildWorkflowCommand {
    pub workflow_id: String,
    pub workflow_type: String,
    pub args: Option<Vec<u8>>,
    pub options: ChildWorkflowOptions,
}

#[derive(Debug)]
pub struct StartTimerCommand {
    pub timer_id: String,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct CancelTimerCommand {
    pub timer_id: String,
}

#[derive(Debug)]
pub struct CompleteWorkflowCommand {
    pub result: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct FailWorkflowCommand {
    pub reason: String,
    pub details: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct CancelWorkflowCommand {
    pub details: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct ContinueAsNewWorkflowCommand {
    pub workflow_type: String,
    pub input: Option<Vec<u8>>,
    pub options: crate::context::ContinueAsNewOptions,
}

#[derive(Debug)]
pub struct SignalExternalWorkflowCommand {
    pub signal_id: String, // Internal ID for tracking
    pub domain: Option<String>,
    pub workflow_id: String,
    pub run_id: Option<String>,
    pub signal_name: String,
    pub args: Option<Vec<u8>>,
    pub child_workflow_only: bool,
}

#[derive(Debug)]
pub struct RequestCancelExternalWorkflowCommand {
    pub cancellation_id: String, // Internal ID for tracking
    pub domain: Option<String>,
    pub workflow_id: String,
    pub run_id: Option<String>,
    pub child_workflow_only: bool,
}

#[derive(Debug)]
pub struct RecordMarkerCommand {
    pub marker_name: String,
    pub details: Vec<u8>,
    pub header: Option<uber_cadence_proto::shared::Header>,
}
