//! Decision state machine for workflow execution.
//!
//! This module manages the state of decisions (activities, timers, child workflows, etc.)
//! to ensure deterministic workflow execution and proper event handling.

use cadence_proto::shared::{
    CancelTimerDecisionAttributes, Decision as ProtoDecision, DecisionAttributes,
    DecisionType as ProtoDecisionType, ScheduleActivityTaskDecisionAttributes,
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
        ) {
            if self.base.state == DecisionState::DecisionSent {
                self.base.transition_to(DecisionState::Initiated);
            }
            // If canceled before initiated, the cancellation is pending
        }
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
        match self.base.state {
            DecisionState::Created => {
                self.base
                    .transition_to(DecisionState::CanceledBeforeInitiated);
            }
            _ => {}
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

    /// Add a decision
    pub fn add_decision(&mut self, decision: Box<dyn DecisionStateMachine>) {
        let id = decision.id().clone();
        self.ordered_decisions.push(id.clone());
        self.decisions.insert(id, decision);
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
    pub fn handle_activity_started(&mut self, _scheduled_event_id: i64) {
        // Find by scheduled_event_id - would need to track this mapping
    }

    /// Handle activity completed event
    pub fn handle_activity_completed(&mut self, _scheduled_event_id: i64) {
        // Find by scheduled_event_id
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
}

impl Default for DecisionsHelper {
    fn default() -> Self {
        Self::new()
    }
}
