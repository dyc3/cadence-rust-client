use crate::registry::ActivityError;
use cadence_core::CadenceError;
use cadence_proto::shared::{EventAttributes, EventType, HistoryEvent};
use cadence_workflow::state_machine::{DecisionId, DecisionsHelper, StateMachineDecisionType};
use std::collections::HashMap;

pub type EventResult = Result<Vec<u8>, ActivityError>;

#[derive(Default)]
pub struct ReplayEngine {
    pub decisions_helper: DecisionsHelper,
    pub event_results: HashMap<i64, EventResult>,
    pub scheduled_event_id_to_decision_id: HashMap<i64, DecisionId>,
    pub signals: HashMap<String, Vec<Vec<u8>>>,
    pub cancel_requested: bool,
    pub last_processed_event_id: i64,
}

impl ReplayEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replay_history(&mut self, events: &[HistoryEvent]) -> Result<(), CadenceError> {
        for event in events {
            // Only process events we haven't seen before
            if event.event_id > self.last_processed_event_id {
                self.process_event(event)?;
                self.last_processed_event_id = event.event_id;
            }
        }
        Ok(())
    }

    fn process_event(&mut self, event: &HistoryEvent) -> Result<(), CadenceError> {
        println!(
            "[ReplayEngine] Processing event: ID={}, Type={:?}",
            event.event_id, event.event_type
        );
        match event.event_type {
            EventType::ActivityTaskScheduled => {
                if let Some(EventAttributes::ActivityTaskScheduledEventAttributes(attrs)) =
                    &event.attributes
                {
                    let activity_id = &attrs.activity_id;
                    println!(
                        "[ReplayEngine] Mapping ActivityScheduled: ID={} ActivityID={}",
                        event.event_id, activity_id
                    );

                    // Pre-create state machine in Initiated state (for replay)
                    self.decisions_helper
                        .add_initiated_activity(activity_id.clone());

                    // Also record in scheduled_event_id mapping
                    self.scheduled_event_id_to_decision_id.insert(
                        event.event_id,
                        DecisionId::new(StateMachineDecisionType::Activity, activity_id),
                    );
                } else {
                    println!("[ReplayEngine] WARN: ActivityTaskScheduled event {} missing attributes or mismatch. Attributes: {:?}", event.event_id, event.attributes);
                }
            }
            EventType::ActivityTaskStarted => {
                if let Some(EventAttributes::ActivityTaskStartedEventAttributes(attrs)) =
                    &event.attributes
                {
                    let scheduled_id = attrs.scheduled_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&scheduled_id)
                    {
                        self.decisions_helper
                            .handle_activity_started(&decision_id.id);
                    }
                }
            }
            EventType::ActivityTaskCompleted => {
                if let Some(EventAttributes::ActivityTaskCompletedEventAttributes(attrs)) =
                    &event.attributes
                {
                    let scheduled_id = attrs.scheduled_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&scheduled_id)
                    {
                        self.decisions_helper
                            .handle_activity_closed(&decision_id.id);
                    }

                    let result = attrs.result.clone().unwrap_or_default();
                    self.event_results.insert(scheduled_id, Ok(result));
                }
            }
            EventType::ActivityTaskFailed => {
                if let Some(EventAttributes::ActivityTaskFailedEventAttributes(attrs)) =
                    &event.attributes
                {
                    let scheduled_id = attrs.scheduled_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&scheduled_id)
                    {
                        self.decisions_helper
                            .handle_activity_closed(&decision_id.id);
                    }

                    let reason = attrs.reason.clone().unwrap_or_default();
                    let details = attrs.details.clone().unwrap_or_default();

                    // Construct ActivityError
                    let error =
                        ActivityError::ExecutionFailed(format!("{}: {:?}", reason, details));
                    self.event_results.insert(scheduled_id, Err(error));
                }
            }
            EventType::TimerStarted => {
                if let Some(EventAttributes::TimerStartedEventAttributes(attrs)) = &event.attributes
                {
                    let timer_id = &attrs.timer_id;

                    // Pre-create state machine in Initiated state (for replay)
                    self.decisions_helper.add_initiated_timer(timer_id.clone());

                    self.scheduled_event_id_to_decision_id.insert(
                        event.event_id,
                        DecisionId::new(StateMachineDecisionType::Timer, timer_id),
                    );
                }
            }
            EventType::TimerFired => {
                if let Some(EventAttributes::TimerFiredEventAttributes(attrs)) = &event.attributes {
                    let started_id = attrs.started_event_id; // For timer, scheduled_event_id is essentially started_event_id

                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&started_id)
                    {
                        self.decisions_helper.handle_timer_fired(&decision_id.id);
                    }

                    // Timers usually result in a "void" success or just waking up.
                    // We treat it as empty result.
                    self.event_results.insert(started_id, Ok(Vec::new()));
                }
            }
            EventType::TimerCanceled => {
                if let Some(EventAttributes::TimerCanceledEventAttributes(attrs)) =
                    &event.attributes
                {
                    let started_id = attrs.started_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&started_id)
                    {
                        self.decisions_helper.handle_timer_canceled(&decision_id.id);
                    }
                    // Timer cancellation is successful if we see this event
                    self.event_results
                        .insert(started_id, Err(ActivityError::Cancelled));
                }
            }
            EventType::StartChildWorkflowExecutionInitiated => {
                if let Some(EventAttributes::StartChildWorkflowExecutionInitiatedEventAttributes(
                    attrs,
                )) = &event.attributes
                {
                    let workflow_id = &attrs.workflow_id;

                    // Pre-create state machine in Initiated state (for replay)
                    self.decisions_helper
                        .add_initiated_child_workflow(workflow_id.clone());

                    self.scheduled_event_id_to_decision_id.insert(
                        event.event_id,
                        DecisionId::new(StateMachineDecisionType::ChildWorkflow, workflow_id),
                    );
                }
            }
            EventType::ChildWorkflowExecutionStarted => {
                if let Some(EventAttributes::ChildWorkflowExecutionStartedEventAttributes(attrs)) =
                    &event.attributes
                {
                    let initiated_id = attrs.initiated_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&initiated_id)
                    {
                        self.decisions_helper
                            .handle_child_workflow_started(&decision_id.id);
                    }
                }
            }
            EventType::ChildWorkflowExecutionCompleted => {
                if let Some(EventAttributes::ChildWorkflowExecutionCompletedEventAttributes(
                    attrs,
                )) = &event.attributes
                {
                    let initiated_id = attrs.initiated_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&initiated_id)
                    {
                        self.decisions_helper
                            .handle_child_workflow_closed(&decision_id.id);
                    }
                    let result = attrs.result.clone().unwrap_or_default();
                    self.event_results.insert(initiated_id, Ok(result));
                }
            }
            EventType::ChildWorkflowExecutionFailed => {
                if let Some(EventAttributes::ChildWorkflowExecutionFailedEventAttributes(attrs)) =
                    &event.attributes
                {
                    let initiated_id = attrs.initiated_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&initiated_id)
                    {
                        self.decisions_helper
                            .handle_child_workflow_closed(&decision_id.id);
                    }
                    let reason = attrs.reason.clone().unwrap_or_default();
                    let details = attrs.details.clone().unwrap_or_default();
                    let error = ActivityError::ExecutionFailed(format!(
                        "Child Workflow Failed: {}: {:?}",
                        reason, details
                    ));
                    self.event_results.insert(initiated_id, Err(error));
                }
            }
            EventType::ChildWorkflowExecutionTimedOut => {
                if let Some(EventAttributes::ChildWorkflowExecutionTimedOutEventAttributes(attrs)) =
                    &event.attributes
                {
                    let initiated_id = attrs.initiated_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&initiated_id)
                    {
                        self.decisions_helper
                            .handle_child_workflow_closed(&decision_id.id);
                    }
                    let error = ActivityError::Timeout(attrs.timeout_type);
                    self.event_results.insert(initiated_id, Err(error));
                }
            }
            EventType::ChildWorkflowExecutionCanceled => {
                if let Some(EventAttributes::ChildWorkflowExecutionCanceledEventAttributes(attrs)) =
                    &event.attributes
                {
                    let initiated_id = attrs.initiated_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&initiated_id)
                    {
                        self.decisions_helper
                            .handle_child_workflow_closed(&decision_id.id);
                    }
                    self.event_results
                        .insert(initiated_id, Err(ActivityError::Cancelled));
                }
            }
            EventType::ChildWorkflowExecutionTerminated => {
                if let Some(EventAttributes::ChildWorkflowExecutionTerminatedEventAttributes(
                    attrs,
                )) = &event.attributes
                {
                    let initiated_id = attrs.initiated_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&initiated_id)
                    {
                        self.decisions_helper
                            .handle_child_workflow_closed(&decision_id.id);
                    }
                    let error =
                        ActivityError::ExecutionFailed("Child Workflow Terminated".to_string());
                    self.event_results.insert(initiated_id, Err(error));
                }
            }
            EventType::WorkflowExecutionSignaled => {
                if let Some(EventAttributes::WorkflowExecutionSignaledEventAttributes(attrs)) =
                    &event.attributes
                {
                    let signal_name = attrs.signal_name.clone();
                    let input = attrs.input.clone().unwrap_or_default();
                    self.signals.entry(signal_name).or_default().push(input);
                }
            }
            EventType::WorkflowExecutionCancelRequested => {
                self.cancel_requested = true;
            }
            EventType::SignalExternalWorkflowExecutionInitiated => {
                if let Some(
                    EventAttributes::SignalExternalWorkflowExecutionInitiatedEventAttributes(attrs),
                ) = &event.attributes
                {
                    // Control is used as ID (set by CommandSink)
                    let signal_id = attrs.control.clone().unwrap_or_default();
                    self.decisions_helper
                        .handle_signal_external_workflow_initiated(&signal_id);
                    self.scheduled_event_id_to_decision_id.insert(
                        event.event_id,
                        DecisionId::new(
                            StateMachineDecisionType::ExternalWorkflowSignal,
                            signal_id,
                        ),
                    );
                }
            }
            EventType::ExternalWorkflowExecutionSignaled => {
                if let Some(EventAttributes::ExternalWorkflowExecutionSignaledEventAttributes(
                    attrs,
                )) = &event.attributes
                {
                    let initiated_id = attrs.initiated_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&initiated_id)
                    {
                        self.decisions_helper
                            .handle_signal_external_workflow_completed(&decision_id.id);
                    }
                    self.event_results.insert(initiated_id, Ok(Vec::new()));
                }
            }
            EventType::SignalExternalWorkflowExecutionFailed => {
                if let Some(
                    EventAttributes::SignalExternalWorkflowExecutionFailedEventAttributes(attrs),
                ) = &event.attributes
                {
                    let initiated_id = attrs.initiated_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&initiated_id)
                    {
                        self.decisions_helper
                            .handle_signal_external_workflow_failed(&decision_id.id);
                    }
                    let error = ActivityError::ExecutionFailed(format!(
                        "Signal External Workflow Failed: {:?}",
                        attrs.cause
                    ));
                    self.event_results.insert(initiated_id, Err(error));
                }
            }
            EventType::RequestCancelExternalWorkflowExecutionInitiated => {
                if let Some(
                    EventAttributes::RequestCancelExternalWorkflowExecutionInitiatedEventAttributes(
                        attrs,
                    ),
                ) = &event.attributes
                {
                    // Control is used as ID
                    let cancel_id = attrs.control.clone().unwrap_or_default();
                    self.decisions_helper
                        .handle_request_cancel_external_workflow_initiated(&cancel_id);
                    self.scheduled_event_id_to_decision_id.insert(
                        event.event_id,
                        DecisionId::new(
                            StateMachineDecisionType::ExternalWorkflowCancellation,
                            cancel_id,
                        ),
                    );
                }
            }
            EventType::ExternalWorkflowExecutionCancelRequested => {
                if let Some(
                    EventAttributes::ExternalWorkflowExecutionCancelRequestedEventAttributes(attrs),
                ) = &event.attributes
                {
                    let initiated_id = attrs.initiated_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&initiated_id)
                    {
                        self.decisions_helper
                            .handle_request_cancel_external_workflow_completed(&decision_id.id);
                    }
                    self.event_results.insert(initiated_id, Ok(Vec::new()));
                }
            }
            EventType::RequestCancelExternalWorkflowExecutionFailed => {
                if let Some(
                    EventAttributes::RequestCancelExternalWorkflowExecutionFailedEventAttributes(
                        attrs,
                    ),
                ) = &event.attributes
                {
                    let initiated_id = attrs.initiated_event_id;
                    if let Some(decision_id) =
                        self.scheduled_event_id_to_decision_id.get(&initiated_id)
                    {
                        self.decisions_helper
                            .handle_request_cancel_external_workflow_failed(&decision_id.id);
                    }
                    let error = ActivityError::ExecutionFailed(format!(
                        "Request Cancel External Workflow Failed: {:?}",
                        attrs.cause
                    ));
                    self.event_results.insert(initiated_id, Err(error));
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn get_activity_result(&self, activity_id: &str) -> Option<&EventResult> {
        println!(
            "[ReplayEngine] Looking up activity result for: {}",
            activity_id
        );
        for (sched_id, decision_id) in &self.scheduled_event_id_to_decision_id {
            if decision_id.decision_type == StateMachineDecisionType::Activity
                && decision_id.id == activity_id
            {
                let result = self.event_results.get(sched_id);
                println!(
                    "[ReplayEngine] Found activity result for {}: {:?}",
                    activity_id,
                    result.is_some()
                );
                return result;
            }
        }
        println!(
            "[ReplayEngine] Activity result for {} NOT FOUND in {} mapped events",
            activity_id,
            self.scheduled_event_id_to_decision_id.len()
        );
        None
    }

    pub fn get_timer_result(&self, timer_id: &str) -> Option<&EventResult> {
        for (sched_id, decision_id) in &self.scheduled_event_id_to_decision_id {
            if decision_id.decision_type == StateMachineDecisionType::Timer
                && decision_id.id == timer_id
            {
                return self.event_results.get(sched_id);
            }
        }
        None
    }

    pub fn get_child_workflow_result(&self, workflow_id: &str) -> Option<&EventResult> {
        for (sched_id, decision_id) in &self.scheduled_event_id_to_decision_id {
            if decision_id.decision_type == StateMachineDecisionType::ChildWorkflow
                && decision_id.id == workflow_id
            {
                return self.event_results.get(sched_id);
            }
        }
        None
    }

    pub fn get_signal_external_workflow_result(&self, signal_id: &str) -> Option<&EventResult> {
        for (sched_id, decision_id) in &self.scheduled_event_id_to_decision_id {
            if decision_id.decision_type == StateMachineDecisionType::ExternalWorkflowSignal
                && decision_id.id == signal_id
            {
                return self.event_results.get(sched_id);
            }
        }
        None
    }

    pub fn get_request_cancel_external_workflow_result(
        &self,
        cancel_id: &str,
    ) -> Option<&EventResult> {
        for (sched_id, decision_id) in &self.scheduled_event_id_to_decision_id {
            if decision_id.decision_type == StateMachineDecisionType::ExternalWorkflowCancellation
                && decision_id.id == cancel_id
            {
                return self.event_results.get(sched_id);
            }
        }
        None
    }

    /// Check if an activity has already been scheduled (from history replay).
    /// This prevents duplicate schedule decisions during replay when an
    /// activity is in-progress but not yet completed.
    pub fn is_activity_scheduled(&self, activity_id: &str) -> bool {
        for decision_id in self.scheduled_event_id_to_decision_id.values() {
            if decision_id.decision_type == StateMachineDecisionType::Activity
                && decision_id.id == activity_id
            {
                return true;
            }
        }
        false
    }

    /// Check if a timer has already been started (from history replay).
    pub fn is_timer_scheduled(&self, timer_id: &str) -> bool {
        for decision_id in self.scheduled_event_id_to_decision_id.values() {
            if decision_id.decision_type == StateMachineDecisionType::Timer
                && decision_id.id == timer_id
            {
                return true;
            }
        }
        false
    }

    /// Check if a child workflow has already been initiated (from history replay).
    pub fn is_child_workflow_initiated(&self, workflow_id: &str) -> bool {
        for decision_id in self.scheduled_event_id_to_decision_id.values() {
            if decision_id.decision_type == StateMachineDecisionType::ChildWorkflow
                && decision_id.id == workflow_id
            {
                return true;
            }
        }
        false
    }
}
