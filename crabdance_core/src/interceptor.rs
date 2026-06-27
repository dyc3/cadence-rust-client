//! Around-execution interceptor seam.
//!
//! A lean hook for wrapping the execution of activities, workflows, and child
//! workflows on the worker — the Rust analogue of the Go client's
//! `WorkflowInterceptor`, deliberately reduced to the one thing native
//! observability does *not* already cover: a generic "wrap an operation" point
//! for **timing, fault injection, and policy gates**. Logging and metrics are
//! handled by the native `tracing` / `metrics` facades, so they are not part of
//! this seam.
//!
//! An interceptor sees each operation through two hooks:
//!
//! * [`before`](Interceptor::before) runs immediately before the operation.
//!   Returning `Err` **vetoes** the operation — it is not executed and fails with
//!   that error. This is the fault-injection / policy-gate point.
//! * [`after`](Interceptor::after) runs once the operation finishes, with its
//!   wall-clock [`Outcome`]. This is the timing / accounting point.
//!
//! Interceptors compose: `before` hooks fire in registration order, `after`
//! hooks in reverse (innermost last), so a chain nests like middleware.
//!
//! # Determinism
//!
//! The hooks are **synchronous** and run on the worker, not inside replayed
//! workflow code, so wall-clock work (timing, reading a feature flag) is allowed.
//! A [`Operation::Workflow`] interceptor wraps a *decision-task execution*; it
//! must not mutate workflow state or emit commands, and a veto there fails the
//! decision task rather than the workflow logic. Keep `before`/`after` cheap and
//! side-effect-free with respect to workflow state.
//!
//! Interceptors compose with the [`ContextPropagator`](crate::ContextPropagator)
//! seam: each [`InterceptorContext`] carries the operation's
//! [`PropagationContext`], so an interceptor can read the propagated trace
//! context / baggage for the boundary it is wrapping.

use std::sync::Arc;
use std::time::Duration;

use crate::error::CadenceError;
use crate::propagation::PropagationContext;

/// Which execution boundary an interceptor is wrapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// An activity execution on the activity worker.
    Activity,
    /// A workflow decision-task execution on the decision worker.
    Workflow,
    /// A child workflow execution (a child's own decision tasks are wrapped as
    /// [`Operation::Workflow`] when it runs; this variant marks the parent-side
    /// boundary).
    ChildWorkflow,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Operation::Activity => "activity",
            Operation::Workflow => "workflow",
            Operation::ChildWorkflow => "child_workflow",
        };
        f.write_str(s)
    }
}

/// Context describing the operation being intercepted.
#[derive(Debug, Clone)]
pub struct InterceptorContext {
    /// The boundary being wrapped.
    pub operation: Operation,
    /// The activity type or workflow type name.
    pub name: String,
    /// The workflow id this operation belongs to (best-effort; may be empty).
    pub workflow_id: String,
    /// The workflow run id (best-effort; may be empty).
    pub run_id: String,
    /// Whether the workflow is replaying (meaningful for [`Operation::Workflow`]).
    pub is_replaying: bool,
    /// The propagated trace context / baggage for this boundary.
    pub propagation: PropagationContext,
}

impl InterceptorContext {
    /// Convenience constructor with empty ids and propagation.
    pub fn new(operation: Operation, name: impl Into<String>) -> Self {
        Self {
            operation,
            name: name.into(),
            workflow_id: String::new(),
            run_id: String::new(),
            is_replaying: false,
            propagation: PropagationContext::new(),
        }
    }
}

/// The result of an intercepted operation, passed to [`Interceptor::after`].
#[derive(Debug, Clone, Copy)]
pub struct Outcome {
    /// Wall-clock duration of the operation.
    pub duration: Duration,
    /// Whether the operation completed successfully.
    pub success: bool,
}

/// An around-execution interceptor. Both hooks default to pass-through, so an
/// implementation overrides only what it needs.
pub trait Interceptor: Send + Sync {
    /// Runs before the operation. Return `Err` to veto it.
    fn before(&self, _ctx: &InterceptorContext) -> Result<(), CadenceError> {
        Ok(())
    }

    /// Runs after the operation completes, with its wall-clock outcome.
    fn after(&self, _ctx: &InterceptorContext, _outcome: &Outcome) {}
}

/// An ordered chain of interceptors applied around a single operation.
#[derive(Clone, Default)]
pub struct InterceptorChain {
    interceptors: Vec<Arc<dyn Interceptor>>,
}

impl InterceptorChain {
    pub fn new(interceptors: Vec<Arc<dyn Interceptor>>) -> Self {
        Self { interceptors }
    }

    pub fn is_empty(&self) -> bool {
        self.interceptors.is_empty()
    }

    /// Fire every `before` hook in order. Stops and returns the first veto.
    pub fn before(&self, ctx: &InterceptorContext) -> Result<(), CadenceError> {
        for interceptor in &self.interceptors {
            interceptor.before(ctx)?;
        }
        Ok(())
    }

    /// Fire every `after` hook in reverse order (middleware nesting).
    pub fn after(&self, ctx: &InterceptorContext, outcome: &Outcome) {
        for interceptor in self.interceptors.iter().rev() {
            interceptor.after(ctx, outcome);
        }
    }
}

impl std::fmt::Debug for InterceptorChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InterceptorChain")
            .field("len", &self.interceptors.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[derive(Default)]
    struct RecordingInterceptor {
        before_calls: AtomicUsize,
        after_calls: AtomicUsize,
        last_duration_micros: AtomicUsize,
    }

    impl Interceptor for RecordingInterceptor {
        fn before(&self, _ctx: &InterceptorContext) -> Result<(), CadenceError> {
            self.before_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn after(&self, _ctx: &InterceptorContext, outcome: &Outcome) {
            self.after_calls.fetch_add(1, Ordering::SeqCst);
            self.last_duration_micros
                .store(outcome.duration.as_micros() as usize, Ordering::SeqCst);
        }
    }

    struct VetoInterceptor;
    impl Interceptor for VetoInterceptor {
        fn before(&self, ctx: &InterceptorContext) -> Result<(), CadenceError> {
            Err(CadenceError::Other(format!("vetoed {}", ctx.operation)))
        }
    }

    #[test]
    fn test_chain_fires_before_and_after() {
        let recorder = Arc::new(RecordingInterceptor::default());
        let chain = InterceptorChain::new(vec![recorder.clone()]);
        let ctx = InterceptorContext::new(Operation::Activity, "DoThing");

        assert!(chain.before(&ctx).is_ok());
        chain.after(
            &ctx,
            &Outcome {
                duration: Duration::from_millis(7),
                success: true,
            },
        );

        assert_eq!(recorder.before_calls.load(Ordering::SeqCst), 1);
        assert_eq!(recorder.after_calls.load(Ordering::SeqCst), 1);
        assert_eq!(recorder.last_duration_micros.load(Ordering::SeqCst), 7_000);
    }

    #[test]
    fn test_veto_short_circuits_before_chain() {
        let recorder = Arc::new(RecordingInterceptor::default());
        // Veto is registered before the recorder; the recorder's before must not run.
        let chain = InterceptorChain::new(vec![Arc::new(VetoInterceptor), recorder.clone()]);
        let ctx = InterceptorContext::new(Operation::Activity, "DoThing");

        let result = chain.before(&ctx);
        assert!(result.is_err());
        assert_eq!(recorder.before_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_empty_chain_is_passthrough() {
        let chain = InterceptorChain::default();
        let ctx = InterceptorContext::new(Operation::Workflow, "MyWorkflow");
        assert!(chain.is_empty());
        assert!(chain.before(&ctx).is_ok());
        chain.after(
            &ctx,
            &Outcome {
                duration: Duration::ZERO,
                success: true,
            },
        );
    }
}
