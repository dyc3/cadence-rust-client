use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uber_cadence_core::CadenceResult;
use uber_cadence_proto::shared::{EventAttributes, EventType, HistoryEvent};
use uber_cadence_workflow::local_activity::{
    decode_local_activity_marker, LocalActivityMarkerData,
};
use uber_cadence_workflow::side_effect_serialization::{
    decode_mutable_side_effect_details, decode_side_effect_details, decode_version_details,
};
use uber_cadence_workflow::state_machine::{DecisionId, DecisionsHelper, StateMachineDecisionType};

use crate::registry::ActivityError;

pub type EventResult = Result<Vec<u8>, ActivityError>;

#[derive(Default)]
pub struct ReplayEngine {
    pub decisions_helper: DecisionsHelper,
    pub event_results: HashMap<i64, EventResult>,
    pub scheduled_event_id_to_decision_id: HashMap<i64, DecisionId>,
    pub signals: HashMap<String, Vec<Vec<u8>>>,
    pub cancel_requested: bool,
    pub last_processed_event_id: i64,
    // Side effect result caches for replay
    pub side_effect_results: HashMap<u64, Vec<u8>>,
    pub mutable_side_effects: HashMap<String, Vec<u8>>,
    // Flag indicating we're in replay mode
    pub is_replay: bool,
    // Deterministic time tracking
    pub workflow_start_time_nanos: Option<i64>,
    pub workflow_task_time_nanos: Option<i64>,
    // Version markers cache for workflow versioning
    pub change_versions: HashMap<String, i32>,
    // Local activity results cache for replay (shared with workflow context)
    pub local_activity_results: Arc<Mutex<HashMap<String, LocalActivityMarkerData>>>,
}

impl ReplayEngine {
    pub fn new() -> Self {
        Self {
            local_activity_results: Arc::new(Mutex::new(HashMap::new())),
            ..Default::default()
        }
    }

    pub fn replay_history(&mut self, events: &[HistoryEvent]) -> CadenceResult<()> {
        for event in events {
            // Only process events we haven't seen before
            if event.event_id > self.last_processed_event_id {
                self.process_event(event)?;
                self.last_processed_event_id = event.event_id;
            }
        }
        Ok(())
    }

    fn process_event(&mut self, event: &HistoryEvent) -> CadenceResult<()> {
        println!(
            "[ReplayEngine] Processing event: ID={}, Type={:?}",
            event.event_id, event.event_type
        );
        match event.event_type {
            EventType::WorkflowExecutionStarted => {
                // Capture workflow start time
                self.workflow_start_time_nanos = Some(event.timestamp);
                println!(
                    "[ReplayEngine] Captured workflow start time: {}",
                    event.timestamp
                );
            }
            EventType::DecisionTaskStarted => {
                // Capture decision task time (workflow "current time")
                self.workflow_task_time_nanos = Some(event.timestamp);
                println!(
                    "[ReplayEngine] Captured decision task time: {}",
                    event.timestamp
                );
            }
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
            EventType::MarkerRecorded => {
                if let Some(EventAttributes::MarkerRecordedEventAttributes(attrs)) =
                    &event.attributes
                {
                    let marker_name = attrs.marker_name.as_str();

                    if let Some(details) = &attrs.details {
                        match marker_name {
                            uber_cadence_workflow::context::SIDE_EFFECT_MARKER_NAME => {
                                // Decode side effect marker details
                                if let Ok((side_effect_id, result)) =
                                    decode_side_effect_details(details)
                                {
                                    println!(
                                        "[ReplayEngine] Processing side effect marker: id={}",
                                        side_effect_id
                                    );
                                    self.side_effect_results.insert(side_effect_id, result);
                                }
                            }
                            uber_cadence_workflow::context::MUTABLE_SIDE_EFFECT_MARKER_NAME => {
                                // Decode mutable side effect marker details
                                if let Ok((id, result)) =
                                    decode_mutable_side_effect_details(details)
                                {
                                    println!(
                                        "[ReplayEngine] Processing mutable side effect marker: id={}",
                                        id
                                    );
                                    self.mutable_side_effects.insert(id, result);
                                }
                            }
                            uber_cadence_workflow::context::VERSION_MARKER_NAME => {
                                // Decode version marker details
                                if let Ok((change_id, version)) = decode_version_details(details) {
                                    println!(
                                        "[ReplayEngine] Processing version marker: changeID='{}', version={}",
                                        change_id, version
                                    );
                                    self.change_versions.insert(change_id, version);
                                } else {
                                    println!(
                                        "[ReplayEngine] Failed to decode version marker details"
                                    );
                                }
                            }
                            uber_cadence_workflow::local_activity::LOCAL_ACTIVITY_MARKER_NAME => {
                                // Decode local activity marker details
                                if let Ok(marker_data) = decode_local_activity_marker(details) {
                                    println!(
                                        "[ReplayEngine] Processing local activity marker: id={}, type={}",
                                        marker_data.activity_id, marker_data.activity_type
                                    );
                                    self.local_activity_results
                                        .lock()
                                        .unwrap()
                                        .insert(marker_data.activity_id.clone(), marker_data);
                                } else {
                                    println!(
                                        "[ReplayEngine] Failed to decode local activity marker details"
                                    );
                                }
                            }
                            _ => {
                                // Unknown marker type, ignore
                                println!("[ReplayEngine] Unknown marker type: {}", marker_name);
                            }
                        }
                    }
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

    /// Get local activity result from replay cache
    ///
    /// Local activities are recorded as markers and their results are cached
    /// for deterministic replay.
    pub fn get_local_activity_result(&self, activity_id: &str) -> Option<LocalActivityMarkerData> {
        println!(
            "[ReplayEngine] Looking up local activity result for: {}",
            activity_id
        );
        let result = self
            .local_activity_results
            .lock()
            .unwrap()
            .get(activity_id)
            .cloned();
        println!(
            "[ReplayEngine] Local activity result for {}: {:?}",
            activity_id,
            result.is_some()
        );
        result
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

    /// Get a side effect result by ID
    pub fn get_side_effect_result(&self, side_effect_id: u64) -> Option<&Vec<u8>> {
        self.side_effect_results.get(&side_effect_id)
    }

    /// Get a mutable side effect result by ID
    pub fn get_mutable_side_effect_result(&self, id: &str) -> Option<&Vec<u8>> {
        self.mutable_side_effects.get(id)
    }

    /// Check if we're in replay mode
    pub fn is_replay(&self) -> bool {
        self.is_replay
    }

    /// Set replay mode
    pub fn set_replay_mode(&mut self, is_replay: bool) {
        self.is_replay = is_replay;
    }

    /// Get workflow start time in nanoseconds
    pub fn get_workflow_start_time_nanos(&self) -> Option<i64> {
        self.workflow_start_time_nanos
    }

    /// Get workflow task time (current time) in nanoseconds
    pub fn get_workflow_task_time_nanos(&self) -> Option<i64> {
        self.workflow_task_time_nanos
    }

    /// Get all change versions (used to populate WorkflowContext)
    pub fn get_change_versions(&self) -> HashMap<String, i32> {
        self.change_versions.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uber_cadence_proto::shared::*;
    use uber_cadence_workflow::side_effect_serialization::{
        encode_mutable_side_effect_details, encode_side_effect_details, encode_version_details,
    };

    fn create_simple_event(event_id: i64, event_type: EventType) -> HistoryEvent {
        HistoryEvent {
            event_id,
            timestamp: 1000000000 + event_id * 1000,
            event_type,
            version: 0,
            task_id: 1,
            attributes: None,
        }
    }

    #[test]
    fn test_replay_empty_history() {
        let mut engine = ReplayEngine::new();
        let result = engine.replay_history(&[]);
        assert!(result.is_ok());
        assert_eq!(engine.last_processed_event_id, 0);
    }

    #[test]
    fn test_workflow_start_captures_time() {
        let mut engine = ReplayEngine::new();
        let event = create_simple_event(1, EventType::WorkflowExecutionStarted);

        engine.replay_history(&[event]).unwrap();

        assert_eq!(engine.workflow_start_time_nanos, Some(1000001000));
        assert_eq!(engine.last_processed_event_id, 1);
    }

    #[test]
    fn test_decision_task_captures_time() {
        let mut engine = ReplayEngine::new();
        let event = create_simple_event(5, EventType::DecisionTaskStarted);

        engine.replay_history(&[event]).unwrap();

        assert_eq!(engine.workflow_task_time_nanos, Some(1000005000));
    }

    #[test]
    fn test_signal_accumulation() {
        let mut engine = ReplayEngine::new();

        let signal1 = HistoryEvent {
            event_id: 40,
            timestamp: 1000040000,
            event_type: EventType::WorkflowExecutionSignaled,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::WorkflowExecutionSignaledEventAttributes(
                Box::new(WorkflowExecutionSignaledEventAttributes {
                    signal_name: "test-signal".to_string(),
                    input: Some(vec![1, 2, 3]),
                    identity: "caller-1".to_string(),
                }),
            )),
        };

        let signal2 = HistoryEvent {
            event_id: 41,
            timestamp: 1000041000,
            event_type: EventType::WorkflowExecutionSignaled,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::WorkflowExecutionSignaledEventAttributes(
                Box::new(WorkflowExecutionSignaledEventAttributes {
                    signal_name: "test-signal".to_string(),
                    input: Some(vec![4, 5, 6]),
                    identity: "caller-2".to_string(),
                }),
            )),
        };

        let signal3 = HistoryEvent {
            event_id: 42,
            timestamp: 1000042000,
            event_type: EventType::WorkflowExecutionSignaled,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::WorkflowExecutionSignaledEventAttributes(
                Box::new(WorkflowExecutionSignaledEventAttributes {
                    signal_name: "other-signal".to_string(),
                    input: Some(vec![7, 8, 9]),
                    identity: "caller-3".to_string(),
                }),
            )),
        };

        engine.replay_history(&[signal1, signal2, signal3]).unwrap();

        // Verify signals are accumulated
        assert_eq!(engine.signals.len(), 2);
        assert_eq!(engine.signals.get("test-signal").unwrap().len(), 2);
        assert_eq!(engine.signals.get("other-signal").unwrap().len(), 1);
        assert_eq!(engine.signals.get("test-signal").unwrap()[0], vec![1, 2, 3]);
        assert_eq!(engine.signals.get("test-signal").unwrap()[1], vec![4, 5, 6]);
    }

    #[test]
    fn test_cancel_requested() {
        let mut engine = ReplayEngine::new();
        assert!(!engine.cancel_requested);

        let event = create_simple_event(50, EventType::WorkflowExecutionCancelRequested);
        engine.replay_history(&[event]).unwrap();

        assert!(engine.cancel_requested);
    }

    #[test]
    fn test_side_effect_marker() {
        let mut engine = ReplayEngine::new();

        let details = encode_side_effect_details(123, &[11, 22, 33]);
        let marker_event = HistoryEvent {
            event_id: 60,
            timestamp: 1000060000,
            event_type: EventType::MarkerRecorded,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::MarkerRecordedEventAttributes(Box::new(
                MarkerRecordedEventAttributes {
                    marker_name: uber_cadence_workflow::context::SIDE_EFFECT_MARKER_NAME
                        .to_string(),
                    details: Some(details),
                    decision_task_completed_event_id: 59,
                    header: None,
                },
            ))),
        };

        engine.replay_history(&[marker_event]).unwrap();

        let result = engine.get_side_effect_result(123);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), &vec![11, 22, 33]);
    }

    #[test]
    fn test_mutable_side_effect_marker() {
        let mut engine = ReplayEngine::new();

        let details = encode_mutable_side_effect_details("mut-id-1", &[44, 55, 66]);
        let marker_event = HistoryEvent {
            event_id: 61,
            timestamp: 1000061000,
            event_type: EventType::MarkerRecorded,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::MarkerRecordedEventAttributes(Box::new(
                MarkerRecordedEventAttributes {
                    marker_name: uber_cadence_workflow::context::MUTABLE_SIDE_EFFECT_MARKER_NAME
                        .to_string(),
                    details: Some(details),
                    decision_task_completed_event_id: 59,
                    header: None,
                },
            ))),
        };

        engine.replay_history(&[marker_event]).unwrap();

        let result = engine.get_mutable_side_effect_result("mut-id-1");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), &vec![44, 55, 66]);
    }

    #[test]
    fn test_version_marker() {
        let mut engine = ReplayEngine::new();

        let details = encode_version_details("feature-flag-1", 3);
        let marker_event = HistoryEvent {
            event_id: 62,
            timestamp: 1000062000,
            event_type: EventType::MarkerRecorded,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::MarkerRecordedEventAttributes(Box::new(
                MarkerRecordedEventAttributes {
                    marker_name: uber_cadence_workflow::context::VERSION_MARKER_NAME.to_string(),
                    details: Some(details),
                    decision_task_completed_event_id: 59,
                    header: None,
                },
            ))),
        };

        engine.replay_history(&[marker_event]).unwrap();

        let versions = engine.get_change_versions();
        assert_eq!(versions.get("feature-flag-1"), Some(&3));
    }

    #[test]
    fn test_replay_idempotency() {
        let mut engine = ReplayEngine::new();

        let event1 = create_simple_event(1, EventType::WorkflowExecutionStarted);
        let event2 = create_simple_event(2, EventType::DecisionTaskStarted);

        // First replay
        engine
            .replay_history(&[event1.clone(), event2.clone()])
            .unwrap();
        assert_eq!(engine.last_processed_event_id, 2);

        // Replay again - should skip already processed events
        engine.replay_history(&[event1, event2]).unwrap();
        assert_eq!(engine.last_processed_event_id, 2);
    }

    #[test]
    fn test_activity_mapping_and_lookup() {
        let mut engine = ReplayEngine::new();

        let event = HistoryEvent {
            event_id: 10,
            timestamp: 1000010000,
            event_type: EventType::ActivityTaskScheduled,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::ActivityTaskScheduledEventAttributes(
                Box::new(ActivityTaskScheduledEventAttributes {
                    activity_id: "activity-1".to_string(),
                    activity_type: Some(ActivityType {
                        name: "TestActivity".to_string(),
                    }),
                    task_list: Some(TaskList {
                        name: "test-tasks".to_string(),
                        kind: TaskListKind::Normal,
                    }),
                    input: None,
                    schedule_to_close_timeout_seconds: Some(60),
                    schedule_to_start_timeout_seconds: Some(30),
                    start_to_close_timeout_seconds: Some(30),
                    heartbeat_timeout_seconds: Some(10),
                    retry_policy: None,
                    decision_task_completed_event_id: 9,
                }),
            )),
        };

        assert!(!engine.is_activity_scheduled("activity-1"));
        engine.replay_history(&[event]).unwrap();
        assert!(engine.is_activity_scheduled("activity-1"));
        assert!(!engine.is_activity_scheduled("other-activity"));
    }

    #[test]
    fn test_timer_mapping_and_lookup() {
        let mut engine = ReplayEngine::new();

        let event = HistoryEvent {
            event_id: 20,
            timestamp: 1000020000,
            event_type: EventType::TimerStarted,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::TimerStartedEventAttributes(Box::new(
                TimerStartedEventAttributes {
                    timer_id: "timer-1".to_string(),
                    start_to_fire_timeout_seconds: 10,
                    decision_task_completed_event_id: 19,
                },
            ))),
        };

        assert!(!engine.is_timer_scheduled("timer-1"));
        engine.replay_history(&[event]).unwrap();
        assert!(engine.is_timer_scheduled("timer-1"));
        assert!(!engine.is_timer_scheduled("other-timer"));
    }

    #[test]
    fn test_activity_completed_stores_result() {
        let mut engine = ReplayEngine::new();

        // Schedule activity first
        let scheduled = HistoryEvent {
            event_id: 10,
            timestamp: 1000010000,
            event_type: EventType::ActivityTaskScheduled,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::ActivityTaskScheduledEventAttributes(
                Box::new(ActivityTaskScheduledEventAttributes {
                    activity_id: "activity-1".to_string(),
                    activity_type: Some(ActivityType {
                        name: "TestActivity".to_string(),
                    }),
                    task_list: Some(TaskList {
                        name: "test-tasks".to_string(),
                        kind: TaskListKind::Normal,
                    }),
                    input: None,
                    schedule_to_close_timeout_seconds: Some(60),
                    schedule_to_start_timeout_seconds: Some(30),
                    start_to_close_timeout_seconds: Some(30),
                    heartbeat_timeout_seconds: Some(10),
                    retry_policy: None,
                    decision_task_completed_event_id: 9,
                }),
            )),
        };

        // Complete activity
        let completed = HistoryEvent {
            event_id: 12,
            timestamp: 1000012000,
            event_type: EventType::ActivityTaskCompleted,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::ActivityTaskCompletedEventAttributes(
                Box::new(ActivityTaskCompletedEventAttributes {
                    result: Some(vec![1, 2, 3, 4]),
                    scheduled_event_id: 10,
                    started_event_id: 11,
                    identity: "worker-1".to_string(),
                }),
            )),
        };

        engine.replay_history(&[scheduled, completed]).unwrap();

        let result = engine.get_activity_result("activity-1");
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref().unwrap(), &vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_activity_failed_stores_error() {
        let mut engine = ReplayEngine::new();

        let scheduled = HistoryEvent {
            event_id: 10,
            timestamp: 1000010000,
            event_type: EventType::ActivityTaskScheduled,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::ActivityTaskScheduledEventAttributes(
                Box::new(ActivityTaskScheduledEventAttributes {
                    activity_id: "activity-fail".to_string(),
                    activity_type: Some(ActivityType {
                        name: "TestActivity".to_string(),
                    }),
                    task_list: Some(TaskList {
                        name: "test-tasks".to_string(),
                        kind: TaskListKind::Normal,
                    }),
                    input: None,
                    schedule_to_close_timeout_seconds: Some(60),
                    schedule_to_start_timeout_seconds: Some(30),
                    start_to_close_timeout_seconds: Some(30),
                    heartbeat_timeout_seconds: Some(10),
                    retry_policy: None,
                    decision_task_completed_event_id: 9,
                }),
            )),
        };

        let failed = HistoryEvent {
            event_id: 12,
            timestamp: 1000012000,
            event_type: EventType::ActivityTaskFailed,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::ActivityTaskFailedEventAttributes(
                Box::new(ActivityTaskFailedEventAttributes {
                    reason: Some("Test failure".to_string()),
                    details: Some(vec![5, 6, 7]),
                    scheduled_event_id: 10,
                    started_event_id: 11,
                    identity: "worker-1".to_string(),
                }),
            )),
        };

        engine.replay_history(&[scheduled, failed]).unwrap();

        let result = engine.get_activity_result("activity-fail");
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_timer_fired_stores_result() {
        let mut engine = ReplayEngine::new();

        let started = HistoryEvent {
            event_id: 20,
            timestamp: 1000020000,
            event_type: EventType::TimerStarted,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::TimerStartedEventAttributes(Box::new(
                TimerStartedEventAttributes {
                    timer_id: "timer-1".to_string(),
                    start_to_fire_timeout_seconds: 10,
                    decision_task_completed_event_id: 19,
                },
            ))),
        };

        let fired = HistoryEvent {
            event_id: 21,
            timestamp: 1000021000,
            event_type: EventType::TimerFired,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::TimerFiredEventAttributes(Box::new(
                TimerFiredEventAttributes {
                    timer_id: "timer-1".to_string(),
                    started_event_id: 20,
                },
            ))),
        };

        engine.replay_history(&[started, fired]).unwrap();

        let result = engine.get_timer_result("timer-1");
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());
    }

    #[test]
    fn test_timer_canceled_stores_error() {
        let mut engine = ReplayEngine::new();

        let started = HistoryEvent {
            event_id: 20,
            timestamp: 1000020000,
            event_type: EventType::TimerStarted,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::TimerStartedEventAttributes(Box::new(
                TimerStartedEventAttributes {
                    timer_id: "timer-cancel".to_string(),
                    start_to_fire_timeout_seconds: 10,
                    decision_task_completed_event_id: 19,
                },
            ))),
        };

        let canceled = HistoryEvent {
            event_id: 22,
            timestamp: 1000022000,
            event_type: EventType::TimerCanceled,
            version: 0,
            task_id: 1,
            attributes: Some(EventAttributes::TimerCanceledEventAttributes(Box::new(
                TimerCanceledEventAttributes {
                    timer_id: "timer-cancel".to_string(),
                    started_event_id: 20,
                    decision_task_completed_event_id: 21,
                    identity: "".to_string(),
                },
            ))),
        };

        engine.replay_history(&[started, canceled]).unwrap();

        let result = engine.get_timer_result("timer-cancel");
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_replay_mode_flag() {
        let mut engine = ReplayEngine::new();
        assert!(!engine.is_replay());

        engine.set_replay_mode(true);
        assert!(engine.is_replay());

        engine.set_replay_mode(false);
        assert!(!engine.is_replay());
    }

    #[test]
    fn test_multiple_signals_same_name() {
        let mut engine = ReplayEngine::new();

        for i in 0..5 {
            let signal = HistoryEvent {
                event_id: 40 + i,
                timestamp: 1000040000 + i * 1000,
                event_type: EventType::WorkflowExecutionSignaled,
                version: 0,
                task_id: 1,
                attributes: Some(EventAttributes::WorkflowExecutionSignaledEventAttributes(
                    Box::new(WorkflowExecutionSignaledEventAttributes {
                        signal_name: "batch-signal".to_string(),
                        input: Some(vec![i as u8]),
                        identity: format!("caller-{}", i),
                    }),
                )),
            };
            engine.replay_history(&[signal]).unwrap();
        }

        assert_eq!(engine.signals.get("batch-signal").unwrap().len(), 5);
        for i in 0..5 {
            assert_eq!(
                engine.signals.get("batch-signal").unwrap()[i as usize],
                vec![i as u8]
            );
        }
    }
}
