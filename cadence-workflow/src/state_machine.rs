//! Decision state machine for workflow execution.
//!
//! This module manages the state of decisions (activities, timers, child workflows, etc.)
//! to ensure deterministic workflow execution and proper event handling.

use cadence_proto::shared::{
    CancelTimerDecisionAttributes, CompleteWorkflowExecutionDecisionAttributes,
    ContinueAsNewWorkflowExecutionDecisionAttributes, Decision as ProtoDecision,
    DecisionAttributes, DecisionType as ProtoDecisionType, FailWorkflowExecutionDecisionAttributes,
    RequestCancelExternalWorkflowExecutionDecisionAttributes,
    ScheduleActivityTaskDecisionAttributes, SignalExternalWorkflowExecutionDecisionAttributes,
    StartChildWorkflowExecutionDecisionAttributes, StartTimerDecisionAttributes,
};
use std::collections::HashMap;

/// Decision states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionState {
    /// Decision created but not yet sent
    Created,
    /// Decision sent to server
    DecisionSent,
    /// Canceled before being initiated
    CanceledBeforeInitiated,
    /// Decision initiated (server accepted)
    Initiated,
    /// Activity started executing
    Started,
    /// Canceled after initiated but before started
    CanceledAfterInitiated,
    /// Canceled after started
    CanceledAfterStarted,
    /// Cancellation decision sent
    CancellationDecisionSent,
    /// Completed after cancellation was sent
    CompletedAfterCancellationDecisionSent,
    /// Completed successfully
    Completed,
}

/// Decision type identifiers for state machine tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateMachineDecisionType {
    Activity,
    Timer,
    ChildWorkflow,
    ExternalWorkflowCancellation,
    ExternalWorkflowSignal,
    Marker,
    UpsertSearchAttributes,
    CompleteWorkflow,
    FailWorkflow,
    ContinueAsNewWorkflow,
}

/// Decision ID for identifying decisions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecisionId {
    pub decision_type: StateMachineDecisionType,
    pub id: String,
}

impl DecisionId {
    pub fn new(decision_type: StateMachineDecisionType, id: impl Into<String>) -> Self {
        Self {
            decision_type,
            id: id.into(),
        }
    }
}

/// Trait for decision state machines
pub trait DecisionStateMachine: Send + Sync {
    /// Get current state
    fn state(&self) -> DecisionState;

    /// Get decision ID
    fn id(&self) -> &DecisionId;

    /// Check if decision is done
    fn is_done(&self) -> bool;

    /// Get the decision to send (if any)
    fn get_decision(&self) -> Option<ProtoDecision>;

    /// Cancel the decision
    fn cancel(&mut self);

    /// Handle events
    fn handle_started_event(&mut self);
    fn handle_cancel_initiated_event(&mut self);
    fn handle_canceled_event(&mut self);
    fn handle_cancel_failed_event(&mut self);
    fn handle_completion_event(&mut self);
    fn handle_initiation_failed_event(&mut self);
    fn handle_initiated_event(&mut self);

    /// Mark decision as sent
    fn handle_decision_sent(&mut self);
}

/// Base state machine implementation
pub struct DecisionStateMachineBase {
    pub id: DecisionId,
    pub state: DecisionState,
    pub history: Vec<String>,
}

impl DecisionStateMachineBase {
    pub fn new(id: DecisionId) -> Self {
        Self {
            id,
            state: DecisionState::Created,
            history: vec!["Created".to_string()],
        }
    }

    pub fn transition_to(&mut self, new_state: DecisionState) {
        self.history.push(format!("{:?}", new_state));
        self.state = new_state;
    }
}

/// Activity decision state machine
pub struct ActivityDecisionStateMachine {
    base: DecisionStateMachineBase,
    attributes: ScheduleActivityTaskDecisionAttributes,
    #[allow(dead_code)]
    scheduled_event_id: Option<i64>,
    #[allow(dead_code)]
    started_event_id: Option<i64>,
}

impl ActivityDecisionStateMachine {
    pub fn new(id: String, attributes: ScheduleActivityTaskDecisionAttributes) -> Self {
        Self {
            base: DecisionStateMachineBase::new(DecisionId::new(
                StateMachineDecisionType::Activity,
                id,
            )),
            attributes,
            scheduled_event_id: None,
            started_event_id: None,
        }
    }
}

impl DecisionStateMachine for ActivityDecisionStateMachine {
    fn state(&self) -> DecisionState {
        self.base.state
    }

    fn id(&self) -> &DecisionId {
        &self.base.id
    }

    fn is_done(&self) -> bool {
        matches!(
            self.base.state,
            DecisionState::Completed
                | DecisionState::CanceledBeforeInitiated
                | DecisionState::CanceledAfterInitiated
        )
    }

    fn get_decision(&self) -> Option<ProtoDecision> {
        match self.base.state {
            DecisionState::Created => Some(ProtoDecision {
                decision_type: ProtoDecisionType::ScheduleActivityTask,
                attributes: Some(DecisionAttributes::ScheduleActivityTaskDecisionAttributes(
                    Box::new(self.attributes.clone()),
                )),
            }),
            _ => None,
        }
    }

    fn cancel(&mut self) {
        match self.base.state {
            DecisionState::Created => {
                self.base
                    .transition_to(DecisionState::CanceledBeforeInitiated);
            }
            DecisionState::Initiated => {
                self.base
                    .transition_to(DecisionState::CanceledAfterInitiated);
            }
            DecisionState::Started => {
                self.base.transition_to(DecisionState::CanceledAfterStarted);
            }
            _ => {}
        }
    }

    fn handle_started_event(&mut self) {
        if self.base.state == DecisionState::Initiated {
            self.base.transition_to(DecisionState::Started);
        }
    }

    fn handle_cancel_initiated_event(&mut self) {
        if matches!(
            self.base.state,
            DecisionState::CanceledAfterInitiated | DecisionState::CanceledAfterStarted
        ) {
            self.base
                .transition_to(DecisionState::CancellationDecisionSent);
        }
    }

    fn handle_canceled_event(&mut self) {
        if self.base.state == DecisionState::CancellationDecisionSent {
            self.base.transition_to(DecisionState::Completed);
        }
    }

    fn handle_cancel_failed_event(&mut self) {
        // Cancellation failed, decision should complete normally
        if self.base.state == DecisionState::CancellationDecisionSent {
            // Stay in current state or transition to completed
        }
    }

    fn handle_completion_event(&mut self) {
        if matches!(
            self.base.state,
            DecisionState::Initiated
                | DecisionState::Started
                | DecisionState::CancellationDecisionSent
        ) {
            self.base.transition_to(DecisionState::Completed);
        }
    }

    fn handle_initiation_failed_event(&mut self) {
        if self.base.state == DecisionState::DecisionSent {
            self.base.transition_to(DecisionState::Completed);
        }
    }

    fn handle_initiated_event(&mut self) {
        if matches!(
            self.base.state,
            DecisionState::DecisionSent | DecisionState::CanceledBeforeInitiated
        ) && self.base.state == DecisionState::DecisionSent
        {
            self.base.transition_to(DecisionState::Initiated);
        }
        // If canceled before initiated, the cancellation is pending
    }

    fn handle_decision_sent(&mut self) {
        if self.base.state == DecisionState::Created {
            self.base.transition_to(DecisionState::DecisionSent);
        }
    }
}

/// Timer decision state machine
pub struct TimerDecisionStateMachine {
    base: DecisionStateMachineBase,
    attributes: StartTimerDecisionAttributes,
    canceled: bool,
}

impl TimerDecisionStateMachine {
    pub fn new(id: String, attributes: StartTimerDecisionAttributes) -> Self {
        Self {
            base: DecisionStateMachineBase::new(DecisionId::new(
                StateMachineDecisionType::Timer,
                id,
            )),
            attributes,
            canceled: false,
        }
    }
}

impl DecisionStateMachine for TimerDecisionStateMachine {
    fn state(&self) -> DecisionState {
        self.base.state
    }

    fn id(&self) -> &DecisionId {
        &self.base.id
    }

    fn is_done(&self) -> bool {
        matches!(
            self.base.state,
            DecisionState::Completed | DecisionState::CanceledBeforeInitiated
        )
    }

    fn get_decision(&self) -> Option<ProtoDecision> {
        match self.base.state {
            DecisionState::Created if !self.canceled => Some(ProtoDecision {
                decision_type: ProtoDecisionType::StartTimer,
                attributes: Some(DecisionAttributes::StartTimerDecisionAttributes(Box::new(
                    self.attributes.clone(),
                ))),
            }),
            DecisionState::Created if self.canceled => Some(ProtoDecision {
                decision_type: ProtoDecisionType::CancelTimer,
                attributes: Some(DecisionAttributes::CancelTimerDecisionAttributes(Box::new(
                    CancelTimerDecisionAttributes {
                        timer_id: self.attributes.timer_id.clone(),
                    },
                ))),
            }),
            _ => None,
        }
    }

    fn cancel(&mut self) {
        self.canceled = true;
        if self.base.state == DecisionState::Created {
            self.base
                .transition_to(DecisionState::CanceledBeforeInitiated);
        }
    }

    fn handle_started_event(&mut self) {
        // Timers don't have started events
    }

    fn handle_cancel_initiated_event(&mut self) {
        if self.base.state == DecisionState::CanceledBeforeInitiated {
            self.base
                .transition_to(DecisionState::CancellationDecisionSent);
        }
    }

    fn handle_canceled_event(&mut self) {
        if self.base.state == DecisionState::CancellationDecisionSent {
            self.base.transition_to(DecisionState::Completed);
        }
    }

    fn handle_cancel_failed_event(&mut self) {}

    fn handle_completion_event(&mut self) {
        if matches!(
            self.base.state,
            DecisionState::Initiated | DecisionState::DecisionSent
        ) {
            self.base.transition_to(DecisionState::Completed);
        }
    }

    fn handle_initiation_failed_event(&mut self) {
        if self.base.state == DecisionState::DecisionSent {
            self.base.transition_to(DecisionState::Completed);
        }
    }

    fn handle_initiated_event(&mut self) {
        if self.base.state == DecisionState::DecisionSent {
            self.base.transition_to(DecisionState::Initiated);
        }
    }

    fn handle_decision_sent(&mut self) {
        if self.base.state == DecisionState::Created {
            self.base.transition_to(DecisionState::DecisionSent);
        }
    }
}

/// Child workflow decision state machine
pub struct ChildWorkflowDecisionStateMachine {
    base: DecisionStateMachineBase,
    attributes: StartChildWorkflowExecutionDecisionAttributes,
}

impl ChildWorkflowDecisionStateMachine {
    pub fn new(id: String, attributes: StartChildWorkflowExecutionDecisionAttributes) -> Self {
        Self {
            base: DecisionStateMachineBase::new(DecisionId::new(
                StateMachineDecisionType::ChildWorkflow,
                id,
            )),
            attributes,
        }
    }
}

impl DecisionStateMachine for ChildWorkflowDecisionStateMachine {
    fn state(&self) -> DecisionState {
        self.base.state
    }

    fn id(&self) -> &DecisionId {
        &self.base.id
    }

    fn is_done(&self) -> bool {
        matches!(
            self.base.state,
            DecisionState::Completed
                | DecisionState::CanceledBeforeInitiated
                | DecisionState::CanceledAfterInitiated
        )
    }

    fn get_decision(&self) -> Option<ProtoDecision> {
        match self.base.state {
            DecisionState::Created => Some(ProtoDecision {
                decision_type: ProtoDecisionType::StartChildWorkflowExecution,
                attributes: Some(
                    DecisionAttributes::StartChildWorkflowExecutionDecisionAttributes(Box::new(
                        self.attributes.clone(),
                    )),
                ),
            }),
            _ => None,
        }
    }

    fn cancel(&mut self) {
        match self.base.state {
            DecisionState::Created => {
                self.base
                    .transition_to(DecisionState::CanceledBeforeInitiated);
            }
            DecisionState::Initiated | DecisionState::Started => {
                self.base
                    .transition_to(DecisionState::CanceledAfterInitiated);
            }
            _ => {}
        }
    }

    fn handle_started_event(&mut self) {
        if self.base.state == DecisionState::Initiated {
            self.base.transition_to(DecisionState::Started);
        }
    }

    fn handle_cancel_initiated_event(&mut self) {
        if matches!(
            self.base.state,
            DecisionState::CanceledAfterInitiated | DecisionState::CanceledAfterStarted
        ) {
            self.base
                .transition_to(DecisionState::CancellationDecisionSent);
        }
    }

    fn handle_canceled_event(&mut self) {
        if self.base.state == DecisionState::CancellationDecisionSent {
            self.base.transition_to(DecisionState::Completed);
        }
    }

    fn handle_cancel_failed_event(&mut self) {}

    fn handle_completion_event(&mut self) {
        if matches!(
            self.base.state,
            DecisionState::Initiated
                | DecisionState::Started
                | DecisionState::CancellationDecisionSent
        ) {
            self.base.transition_to(DecisionState::Completed);
        }
    }

    fn handle_initiation_failed_event(&mut self) {
        if self.base.state == DecisionState::DecisionSent {
            self.base.transition_to(DecisionState::Completed);
        }
    }

    fn handle_initiated_event(&mut self) {
        if self.base.state == DecisionState::DecisionSent {
            self.base.transition_to(DecisionState::Initiated);
        }
    }

    fn handle_decision_sent(&mut self) {
        if self.base.state == DecisionState::Created {
            self.base.transition_to(DecisionState::DecisionSent);
        }
    }
}

/// Complete workflow decision state machine
pub struct CompleteWorkflowDecisionStateMachine {
    base: DecisionStateMachineBase,
    attributes: CompleteWorkflowExecutionDecisionAttributes,
}

impl CompleteWorkflowDecisionStateMachine {
    pub fn new(attributes: CompleteWorkflowExecutionDecisionAttributes) -> Self {
        Self {
            base: DecisionStateMachineBase::new(DecisionId::new(
                StateMachineDecisionType::CompleteWorkflow,
                "complete",
            )),
            attributes,
        }
    }
}

impl DecisionStateMachine for CompleteWorkflowDecisionStateMachine {
    fn state(&self) -> DecisionState {
        self.base.state
    }
    fn id(&self) -> &DecisionId {
        &self.base.id
    }
    fn is_done(&self) -> bool {
        self.base.state == DecisionState::Completed
    }

    fn get_decision(&self) -> Option<ProtoDecision> {
        if self.base.state == DecisionState::Created {
            Some(ProtoDecision {
                decision_type: ProtoDecisionType::CompleteWorkflowExecution,
                attributes: Some(
                    DecisionAttributes::CompleteWorkflowExecutionDecisionAttributes(Box::new(
                        self.attributes.clone(),
                    )),
                ),
            })
        } else {
            None
        }
    }

    fn cancel(&mut self) {}
    fn handle_started_event(&mut self) {}
    fn handle_cancel_initiated_event(&mut self) {}
    fn handle_canceled_event(&mut self) {}
    fn handle_cancel_failed_event(&mut self) {}
    fn handle_completion_event(&mut self) {}
    fn handle_initiation_failed_event(&mut self) {}
    fn handle_initiated_event(&mut self) {}

    fn handle_decision_sent(&mut self) {
        if self.base.state == DecisionState::Created {
            self.base.transition_to(DecisionState::DecisionSent);
        }
    }
}

/// Fail workflow decision state machine
pub struct FailWorkflowDecisionStateMachine {
    base: DecisionStateMachineBase,
    attributes: FailWorkflowExecutionDecisionAttributes,
}

impl FailWorkflowDecisionStateMachine {
    pub fn new(attributes: FailWorkflowExecutionDecisionAttributes) -> Self {
        Self {
            base: DecisionStateMachineBase::new(DecisionId::new(
                StateMachineDecisionType::FailWorkflow,
                "fail",
            )),
            attributes,
        }
    }
}

impl DecisionStateMachine for FailWorkflowDecisionStateMachine {
    fn state(&self) -> DecisionState {
        self.base.state
    }
    fn id(&self) -> &DecisionId {
        &self.base.id
    }
    fn is_done(&self) -> bool {
        self.base.state == DecisionState::Completed
    }

    fn get_decision(&self) -> Option<ProtoDecision> {
        if self.base.state == DecisionState::Created {
            Some(ProtoDecision {
                decision_type: ProtoDecisionType::FailWorkflowExecution,
                attributes: Some(DecisionAttributes::FailWorkflowExecutionDecisionAttributes(
                    Box::new(self.attributes.clone()),
                )),
            })
        } else {
            None
        }
    }

    fn cancel(&mut self) {}
    fn handle_started_event(&mut self) {}
    fn handle_cancel_initiated_event(&mut self) {}
    fn handle_canceled_event(&mut self) {}
    fn handle_cancel_failed_event(&mut self) {}
    fn handle_completion_event(&mut self) {}
    fn handle_initiation_failed_event(&mut self) {}
    fn handle_initiated_event(&mut self) {}

    fn handle_decision_sent(&mut self) {
        if self.base.state == DecisionState::Created {
            self.base.transition_to(DecisionState::DecisionSent);
        }
    }
}

/// Continue as new workflow decision state machine
pub struct ContinueAsNewWorkflowDecisionStateMachine {
    base: DecisionStateMachineBase,
    attributes: ContinueAsNewWorkflowExecutionDecisionAttributes,
}

impl ContinueAsNewWorkflowDecisionStateMachine {
    pub fn new(attributes: ContinueAsNewWorkflowExecutionDecisionAttributes) -> Self {
        Self {
            base: DecisionStateMachineBase::new(DecisionId::new(
                StateMachineDecisionType::ContinueAsNewWorkflow,
                "continue_as_new",
            )),
            attributes,
        }
    }
}

impl DecisionStateMachine for ContinueAsNewWorkflowDecisionStateMachine {
    fn state(&self) -> DecisionState {
        self.base.state
    }
    fn id(&self) -> &DecisionId {
        &self.base.id
    }
    fn is_done(&self) -> bool {
        self.base.state == DecisionState::Completed
    }

    fn get_decision(&self) -> Option<ProtoDecision> {
        if self.base.state == DecisionState::Created {
            Some(ProtoDecision {
                decision_type: ProtoDecisionType::ContinueAsNewWorkflowExecution,
                attributes: Some(
                    DecisionAttributes::ContinueAsNewWorkflowExecutionDecisionAttributes(Box::new(
                        self.attributes.clone(),
                    )),
                ),
            })
        } else {
            None
        }
    }

    fn cancel(&mut self) {}
    fn handle_started_event(&mut self) {}
    fn handle_cancel_initiated_event(&mut self) {}
    fn handle_canceled_event(&mut self) {}
    fn handle_cancel_failed_event(&mut self) {}
    fn handle_completion_event(&mut self) {}
    fn handle_initiation_failed_event(&mut self) {}
    fn handle_initiated_event(&mut self) {}

    fn handle_decision_sent(&mut self) {
        if self.base.state == DecisionState::Created {
            self.base.transition_to(DecisionState::DecisionSent);
        }
    }
}

/// Signal external workflow decision state machine
pub struct SignalExternalWorkflowDecisionStateMachine {
    base: DecisionStateMachineBase,
    attributes: SignalExternalWorkflowExecutionDecisionAttributes,
}

impl SignalExternalWorkflowDecisionStateMachine {
    pub fn new(id: String, attributes: SignalExternalWorkflowExecutionDecisionAttributes) -> Self {
        Self {
            base: DecisionStateMachineBase::new(DecisionId::new(
                StateMachineDecisionType::ExternalWorkflowSignal,
                id,
            )),
            attributes,
        }
    }
}

impl DecisionStateMachine for SignalExternalWorkflowDecisionStateMachine {
    fn state(&self) -> DecisionState {
        self.base.state
    }
    fn id(&self) -> &DecisionId {
        &self.base.id
    }
    fn is_done(&self) -> bool {
        matches!(
            self.base.state,
            DecisionState::Completed | DecisionState::CanceledBeforeInitiated
        )
    }

    fn get_decision(&self) -> Option<ProtoDecision> {
        if self.base.state == DecisionState::Created {
            Some(ProtoDecision {
                decision_type: ProtoDecisionType::SignalExternalWorkflowExecution,
                attributes: Some(
                    DecisionAttributes::SignalExternalWorkflowExecutionDecisionAttributes(
                        Box::new(self.attributes.clone()),
                    ),
                ),
            })
        } else {
            None
        }
    }

    fn cancel(&mut self) {
        if self.base.state == DecisionState::Created {
            self.base
                .transition_to(DecisionState::CanceledBeforeInitiated);
        }
    }
    fn handle_started_event(&mut self) {}
    fn handle_cancel_initiated_event(&mut self) {}
    fn handle_canceled_event(&mut self) {
        // Not applicable for signal
    }
    fn handle_cancel_failed_event(&mut self) {}
    fn handle_completion_event(&mut self) {
        if matches!(
            self.base.state,
            DecisionState::Initiated | DecisionState::DecisionSent
        ) {
            self.base.transition_to(DecisionState::Completed);
        }
    }
    fn handle_initiation_failed_event(&mut self) {
        if self.base.state == DecisionState::DecisionSent {
            self.base.transition_to(DecisionState::Completed);
        }
    }
    fn handle_initiated_event(&mut self) {
        if self.base.state == DecisionState::DecisionSent {
            self.base.transition_to(DecisionState::Initiated);
        }
    }

    fn handle_decision_sent(&mut self) {
        if self.base.state == DecisionState::Created {
            self.base.transition_to(DecisionState::DecisionSent);
        }
    }
}

/// Request cancel external workflow decision state machine
pub struct RequestCancelExternalWorkflowDecisionStateMachine {
    base: DecisionStateMachineBase,
    attributes: RequestCancelExternalWorkflowExecutionDecisionAttributes,
}

impl RequestCancelExternalWorkflowDecisionStateMachine {
    pub fn new(
        id: String,
        attributes: RequestCancelExternalWorkflowExecutionDecisionAttributes,
    ) -> Self {
        Self {
            base: DecisionStateMachineBase::new(DecisionId::new(
                StateMachineDecisionType::ExternalWorkflowCancellation,
                id,
            )),
            attributes,
        }
    }
}

impl DecisionStateMachine for RequestCancelExternalWorkflowDecisionStateMachine {
    fn state(&self) -> DecisionState {
        self.base.state
    }
    fn id(&self) -> &DecisionId {
        &self.base.id
    }
    fn is_done(&self) -> bool {
        matches!(
            self.base.state,
            DecisionState::Completed | DecisionState::CanceledBeforeInitiated
        )
    }

    fn get_decision(&self) -> Option<ProtoDecision> {
        if self.base.state == DecisionState::Created {
            Some(ProtoDecision {
                decision_type: ProtoDecisionType::RequestCancelExternalWorkflowExecution,
                attributes: Some(
                    DecisionAttributes::RequestCancelExternalWorkflowExecutionDecisionAttributes(
                        Box::new(self.attributes.clone()),
                    ),
                ),
            })
        } else {
            None
        }
    }

    fn cancel(&mut self) {
        if self.base.state == DecisionState::Created {
            self.base
                .transition_to(DecisionState::CanceledBeforeInitiated);
        }
    }
    fn handle_started_event(&mut self) {}
    fn handle_cancel_initiated_event(&mut self) {}
    fn handle_canceled_event(&mut self) {}
    fn handle_cancel_failed_event(&mut self) {}
    fn handle_completion_event(&mut self) {
        if matches!(
            self.base.state,
            DecisionState::Initiated | DecisionState::DecisionSent
        ) {
            self.base.transition_to(DecisionState::Completed);
        }
    }
    fn handle_initiation_failed_event(&mut self) {
        if self.base.state == DecisionState::DecisionSent {
            self.base.transition_to(DecisionState::Completed);
        }
    }
    fn handle_initiated_event(&mut self) {
        if self.base.state == DecisionState::DecisionSent {
            self.base.transition_to(DecisionState::Initiated);
        }
    }

    fn handle_decision_sent(&mut self) {
        if self.base.state == DecisionState::Created {
            self.base.transition_to(DecisionState::DecisionSent);
        }
    }
}

/// Decision helper for managing multiple decisions
pub struct DecisionsHelper {
    decisions: HashMap<DecisionId, Box<dyn DecisionStateMachine>>,
    ordered_decisions: Vec<DecisionId>,
}

impl DecisionsHelper {
    pub fn new() -> Self {
        Self {
            decisions: HashMap::new(),
            ordered_decisions: Vec::new(),
        }
    }

    /// Add a decision. Returns true if added, false if duplicate.
    pub fn add_decision(&mut self, decision: Box<dyn DecisionStateMachine>) -> bool {
        let id = decision.id().clone();

        // Prevent duplicate decisions with same ID
        if self.decisions.contains_key(&id) {
            return false;
        }

        self.ordered_decisions.push(id.clone());
        self.decisions.insert(id, decision);
        true
    }

    /// Get all decisions that need to be sent
    pub fn get_pending_decisions(&self) -> Vec<ProtoDecision> {
        self.ordered_decisions
            .iter()
            .filter_map(|id| self.decisions.get(id))
            .filter_map(|d| d.get_decision())
            .collect()
    }

    /// Get a decision by ID
    pub fn get_decision(&mut self, id: &DecisionId) -> Option<&mut Box<dyn DecisionStateMachine>> {
        self.decisions.get_mut(id)
    }

    /// Mark all created decisions as sent
    pub fn mark_decisions_sent(&mut self) {
        for id in &self.ordered_decisions {
            if let Some(decision) = self.decisions.get_mut(id) {
                if decision.state() == DecisionState::Created {
                    decision.handle_decision_sent();
                }
            }
        }
    }

    /// Check if all decisions are done
    pub fn all_done(&self) -> bool {
        self.decisions.values().all(|d| d.is_done())
    }

    /// Handle activity scheduled event
    pub fn handle_activity_scheduled(&mut self, _scheduled_event_id: i64, activity_id: &str) {
        let id = DecisionId::new(StateMachineDecisionType::Activity, activity_id.to_string());
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_initiated_event();
        }
    }

    /// Handle activity started event
    pub fn handle_activity_started(&mut self, activity_id: &str) {
        let id = DecisionId::new(StateMachineDecisionType::Activity, activity_id.to_string());
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_started_event();
        }
    }

    /// Handle activity completed/failed event
    pub fn handle_activity_closed(&mut self, activity_id: &str) {
        let id = DecisionId::new(StateMachineDecisionType::Activity, activity_id.to_string());
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_completion_event();
        }
    }

    /// Handle timer started event
    pub fn handle_timer_started(&mut self, timer_id: &str) {
        let id = DecisionId::new(StateMachineDecisionType::Timer, timer_id.to_string());
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_initiated_event();
        }
    }

    /// Handle timer fired event
    pub fn handle_timer_fired(&mut self, timer_id: &str) {
        let id = DecisionId::new(StateMachineDecisionType::Timer, timer_id.to_string());
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_completion_event();
        }
    }

    /// Handle timer canceled event
    pub fn handle_timer_canceled(&mut self, timer_id: &str) {
        let id = DecisionId::new(StateMachineDecisionType::Timer, timer_id.to_string());
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_canceled_event();
        }
    }

    /// Handle child workflow initiated event
    pub fn handle_child_workflow_initiated(&mut self, workflow_id: &str) {
        let id = DecisionId::new(
            StateMachineDecisionType::ChildWorkflow,
            workflow_id.to_string(),
        );
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_initiated_event();
        }
    }

    /// Handle child workflow started event
    pub fn handle_child_workflow_started(&mut self, workflow_id: &str) {
        let id = DecisionId::new(
            StateMachineDecisionType::ChildWorkflow,
            workflow_id.to_string(),
        );
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_started_event();
        }
    }

    /// Handle child workflow completed/failed/canceled/timed_out event
    pub fn handle_child_workflow_closed(&mut self, workflow_id: &str) {
        let id = DecisionId::new(
            StateMachineDecisionType::ChildWorkflow,
            workflow_id.to_string(),
        );
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_completion_event();
        }
    }

    pub fn complete_workflow_execution(&mut self, result: Vec<u8>) {
        let attrs = CompleteWorkflowExecutionDecisionAttributes {
            result: Some(result),
        };
        let decision = Box::new(CompleteWorkflowDecisionStateMachine::new(attrs));
        self.add_decision(decision);
    }

    pub fn fail_workflow_execution(&mut self, reason: String, details: String) {
        let attrs = FailWorkflowExecutionDecisionAttributes {
            reason: Some(reason),
            details: Some(details.into_bytes()),
        };
        let decision = Box::new(FailWorkflowDecisionStateMachine::new(attrs));
        self.add_decision(decision);
    }

    pub fn continue_as_new_workflow_execution(
        &mut self,
        attributes: ContinueAsNewWorkflowExecutionDecisionAttributes,
    ) {
        let decision = Box::new(ContinueAsNewWorkflowDecisionStateMachine::new(attributes));
        self.add_decision(decision);
    }

    pub fn signal_external_workflow_execution(
        &mut self,
        id: String,
        attributes: SignalExternalWorkflowExecutionDecisionAttributes,
    ) {
        let decision = Box::new(SignalExternalWorkflowDecisionStateMachine::new(
            id, attributes,
        ));
        self.add_decision(decision);
    }

    pub fn request_cancel_external_workflow_execution(
        &mut self,
        id: String,
        attributes: RequestCancelExternalWorkflowExecutionDecisionAttributes,
    ) {
        let decision = Box::new(RequestCancelExternalWorkflowDecisionStateMachine::new(
            id, attributes,
        ));
        self.add_decision(decision);
    }

    pub fn handle_signal_external_workflow_initiated(&mut self, signal_id: &str) {
        let id = DecisionId::new(
            StateMachineDecisionType::ExternalWorkflowSignal,
            signal_id.to_string(),
        );
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_initiated_event();
        }
    }

    pub fn handle_signal_external_workflow_completed(&mut self, signal_id: &str) {
        let id = DecisionId::new(
            StateMachineDecisionType::ExternalWorkflowSignal,
            signal_id.to_string(),
        );
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_completion_event();
        }
    }

    pub fn handle_signal_external_workflow_failed(&mut self, signal_id: &str) {
        let id = DecisionId::new(
            StateMachineDecisionType::ExternalWorkflowSignal,
            signal_id.to_string(),
        );
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_initiation_failed_event();
        }
    }

    pub fn handle_request_cancel_external_workflow_initiated(&mut self, cancel_id: &str) {
        let id = DecisionId::new(
            StateMachineDecisionType::ExternalWorkflowCancellation,
            cancel_id.to_string(),
        );
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_initiated_event();
        }
    }

    pub fn handle_request_cancel_external_workflow_completed(&mut self, cancel_id: &str) {
        let id = DecisionId::new(
            StateMachineDecisionType::ExternalWorkflowCancellation,
            cancel_id.to_string(),
        );
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_completion_event();
        }
    }

    pub fn handle_request_cancel_external_workflow_failed(&mut self, cancel_id: &str) {
        let id = DecisionId::new(
            StateMachineDecisionType::ExternalWorkflowCancellation,
            cancel_id.to_string(),
        );
        if let Some(decision) = self.decisions.get_mut(&id) {
            decision.handle_initiation_failed_event();
        }
    }

    /// Add a decision from history replay (already initiated).
    /// Used during replay to pre-populate state machines before workflow code runs.
    /// This prevents duplicate schedule decisions during replay.
    pub fn add_initiated_activity(&mut self, activity_id: String) {
        let id = DecisionId::new(StateMachineDecisionType::Activity, activity_id.clone());
        if self.decisions.contains_key(&id) {
            return; // Already exists
        }

        // Create with minimal attributes (not needed for replay)
        let attrs = ScheduleActivityTaskDecisionAttributes {
            activity_id: activity_id.clone(),
            activity_type: None,
            task_list: None,
            input: None,
            schedule_to_close_timeout_seconds: None,
            schedule_to_start_timeout_seconds: None,
            start_to_close_timeout_seconds: None,
            heartbeat_timeout_seconds: None,
            retry_policy: None,
            header: None,
        };
        let mut sm = ActivityDecisionStateMachine::new(activity_id, attrs);

        // Transition to Initiated state (matching history)
        sm.base.transition_to(DecisionState::DecisionSent);
        sm.base.transition_to(DecisionState::Initiated);

        self.ordered_decisions.push(id.clone());
        self.decisions.insert(id, Box::new(sm));
    }

    /// Add a timer from history replay (already started).
    pub fn add_initiated_timer(&mut self, timer_id: String) {
        let id = DecisionId::new(StateMachineDecisionType::Timer, timer_id.clone());
        if self.decisions.contains_key(&id) {
            return; // Already exists
        }

        // Create with default attributes (not needed for replay)
        let attrs = StartTimerDecisionAttributes {
            timer_id: timer_id.clone(),
            start_to_fire_timeout_seconds: 0,
        };
        let mut sm = TimerDecisionStateMachine::new(timer_id, attrs);

        // Transition to Initiated state (matching history)
        sm.base.transition_to(DecisionState::DecisionSent);
        sm.base.transition_to(DecisionState::Initiated);

        self.ordered_decisions.push(id.clone());
        self.decisions.insert(id, Box::new(sm));
    }

    /// Add a child workflow from history replay (already initiated).
    pub fn add_initiated_child_workflow(&mut self, workflow_id: String) {
        let id = DecisionId::new(StateMachineDecisionType::ChildWorkflow, workflow_id.clone());
        if self.decisions.contains_key(&id) {
            return; // Already exists
        }

        // Create with minimal attributes (not needed for replay)
        let attrs = StartChildWorkflowExecutionDecisionAttributes {
            domain: String::new(),
            workflow_id: workflow_id.clone(),
            workflow_type: None,
            task_list: None,
            input: None,
            execution_start_to_close_timeout_seconds: None,
            task_start_to_close_timeout_seconds: None,
            retry_policy: None,
            cron_schedule: None,
            header: None,
            memo: None,
            search_attributes: None,
            workflow_id_reuse_policy: None,
            parent_close_policy: None,
            control: None,
            decision_task_completed_event_id: 0,
        };
        let mut sm = ChildWorkflowDecisionStateMachine::new(workflow_id, attrs);

        // Transition to Initiated state (matching history)
        sm.base.transition_to(DecisionState::DecisionSent);
        sm.base.transition_to(DecisionState::Initiated);

        self.ordered_decisions.push(id.clone());
        self.decisions.insert(id, Box::new(sm));
    }
}

impl Default for DecisionsHelper {
    fn default() -> Self {
        Self::new()
    }
}
