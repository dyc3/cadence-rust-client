//! Worker-side interceptor support: a reference [`TimingInterceptor`] and the
//! helper that extracts a [`PropagationContext`] from a Cadence header so
//! interceptors compose with the configured [`ContextPropagator`]s.
//!
//! The interceptor seam itself ([`Interceptor`], [`InterceptorChain`],
//! [`InterceptorContext`], [`Operation`], [`Outcome`]) lives in `crabdance_core`
//! and is re-exported here for convenience.

use std::sync::Arc;

use crabdance_core::{ContextPropagator, PropagationCarrier, PropagationContext};
use crabdance_proto::shared::Header;

// Re-export the whole interceptor seam (defined in crabdance_core) for convenience.
pub use crabdance_core::{Interceptor, InterceptorChain, InterceptorContext, Operation, Outcome};

/// Reference interceptor that records the wall-clock duration of every
/// intercepted operation through `tracing`. This is the canonical "time every
/// activity" example: register it in `WorkerOptions::interceptors` and each
/// activity (and workflow decision task) emits a timed span event on completion.
///
/// ```
/// use std::sync::Arc;
/// use crabdance_worker::{WorkerOptions, TimingInterceptor};
///
/// let options = WorkerOptions {
///     interceptors: vec![Arc::new(TimingInterceptor)],
///     ..Default::default()
/// };
/// assert_eq!(options.interceptors.len(), 1);
/// ```
#[derive(Debug, Default, Clone)]
pub struct TimingInterceptor;

impl Interceptor for TimingInterceptor {
    fn after(&self, ctx: &InterceptorContext, outcome: &Outcome) {
        tracing::info!(
            operation = %ctx.operation,
            name = %ctx.name,
            workflow_id = %ctx.workflow_id,
            duration_ms = outcome.duration.as_secs_f64() * 1000.0,
            success = outcome.success,
            "operation timed by interceptor"
        );
    }
}

/// Extract a [`PropagationContext`] from a Cadence header using the configured
/// propagators. Returns an empty context when no propagators are configured or
/// the header is absent, so the common (no-propagation) path stays cheap.
pub fn extract_propagation(
    header: &Option<Header>,
    propagators: &[Arc<dyn ContextPropagator>],
) -> PropagationContext {
    if propagators.is_empty() {
        return PropagationContext::new();
    }
    let carrier: PropagationCarrier = header
        .as_ref()
        .map(|h| h.fields.clone())
        .unwrap_or_default();

    let mut merged = PropagationContext::new();
    for propagator in propagators {
        let extracted = propagator.extract(&carrier);
        for (key, value) in extracted.iter() {
            merged.set(key.clone(), value.clone());
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crabdance_core::{W3CTraceContextPropagator, TRACEPARENT_HEADER};

    #[test]
    fn test_extract_propagation_empty_without_propagators() {
        let mut fields = std::collections::HashMap::new();
        fields.insert(TRACEPARENT_HEADER.to_string(), b"00-abc-def-01".to_vec());
        let header = Some(Header { fields });

        let ctx = extract_propagation(&header, &[]);
        assert!(ctx.is_empty(), "no propagators → nothing extracted");
    }

    #[test]
    fn test_extract_propagation_reads_w3c_header() {
        let mut fields = std::collections::HashMap::new();
        fields.insert(TRACEPARENT_HEADER.to_string(), b"00-abc-def-01".to_vec());
        let header = Some(Header { fields });

        let propagators: Vec<Arc<dyn ContextPropagator>> =
            vec![Arc::new(W3CTraceContextPropagator)];
        let ctx = extract_propagation(&header, &propagators);
        assert_eq!(ctx.get(TRACEPARENT_HEADER), Some(&b"00-abc-def-01"[..]));
    }
}
