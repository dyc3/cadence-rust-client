//! Context propagation seam: carry trace context and baggage across the
//! workflow → activity / child-workflow boundary via Cadence headers.
//!
//! This is the Rust analogue of the Go client's `ContextPropagator`. A propagator
//! [`inject`](ContextPropagator::inject)s values from a [`PropagationContext`] into a
//! header carrier when work crosses a boundary, and
//! [`extract`](ContextPropagator::extract)s them back on the receiving side.
//!
//! Two implementations ship in M1:
//! - [`NoopContextPropagator`] — the default; propagates nothing.
//! - [`W3CTraceContextPropagator`] — a scaffold that round-trips the W3C
//!   `traceparent` / `tracestate` / `baggage` header keys.
//!
//! Wiring these to an actual OpenTelemetry tracer (OTLP export, span links) is
//! deferred to M2 (see the parity plan's TD-2); the seam is intentionally decoupled
//! from any tracing backend so that deferral does not block propagation.

use std::collections::HashMap;

/// The carrier of propagated key/value pairs — a Cadence header is a string→bytes map.
pub type PropagationCarrier = HashMap<String, Vec<u8>>;

/// The set of values (trace context + baggage) flowing across a boundary.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PropagationContext {
    entries: HashMap<String, Vec<u8>>,
}

impl PropagationContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a propagated value.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<Vec<u8>>) {
        self.entries.insert(key.into(), value.into());
    }

    /// Get a propagated value.
    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.entries.get(key).map(|v| v.as_slice())
    }

    /// Iterate over all propagated entries.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Vec<u8>)> {
        self.entries.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Injects/extracts propagated values to/from a header carrier across a boundary.
pub trait ContextPropagator: Send + Sync {
    /// Inject values from `context` into `carrier` (called when crossing a boundary
    /// outward — e.g. scheduling an activity or child workflow).
    fn inject(&self, context: &PropagationContext, carrier: &mut PropagationCarrier);

    /// Extract values from `carrier` into a [`PropagationContext`] (called on the
    /// receiving side — e.g. when a worker starts a workflow or activity).
    fn extract(&self, carrier: &PropagationCarrier) -> PropagationContext;
}

/// The default propagator: propagates nothing.
#[derive(Debug, Clone, Default)]
pub struct NoopContextPropagator;

impl ContextPropagator for NoopContextPropagator {
    fn inject(&self, _context: &PropagationContext, _carrier: &mut PropagationCarrier) {}

    fn extract(&self, _carrier: &PropagationCarrier) -> PropagationContext {
        PropagationContext::default()
    }
}

/// W3C Trace Context header keys this propagator carries.
pub const TRACEPARENT_HEADER: &str = "traceparent";
pub const TRACESTATE_HEADER: &str = "tracestate";
pub const BAGGAGE_HEADER: &str = "baggage";

/// A scaffold propagator that round-trips the W3C `traceparent` / `tracestate` /
/// `baggage` header keys between the propagation context and the carrier.
///
/// It moves the raw header values verbatim; binding them to an OpenTelemetry span
/// context (parsing/validating `traceparent`, creating span links, OTLP export) is
/// deferred to M2 (TD-2). This is enough to carry an upstream trace across the
/// workflow/activity boundary unchanged.
#[derive(Debug, Clone, Default)]
pub struct W3CTraceContextPropagator;

impl W3CTraceContextPropagator {
    const KEYS: [&'static str; 3] = [TRACEPARENT_HEADER, TRACESTATE_HEADER, BAGGAGE_HEADER];
}

impl ContextPropagator for W3CTraceContextPropagator {
    fn inject(&self, context: &PropagationContext, carrier: &mut PropagationCarrier) {
        for key in Self::KEYS {
            if let Some(value) = context.get(key) {
                carrier.insert(key.to_string(), value.to_vec());
            }
        }
    }

    fn extract(&self, carrier: &PropagationCarrier) -> PropagationContext {
        let mut context = PropagationContext::default();
        for key in Self::KEYS {
            if let Some(value) = carrier.get(key) {
                context.set(key, value.clone());
            }
        }
        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_propagates_nothing() {
        let mut ctx = PropagationContext::default();
        ctx.set(TRACEPARENT_HEADER, "00-trace-span-01".as_bytes().to_vec());

        let mut carrier = PropagationCarrier::new();
        NoopContextPropagator.inject(&ctx, &mut carrier);
        assert!(carrier.is_empty());

        carrier.insert(TRACEPARENT_HEADER.to_string(), b"x".to_vec());
        assert!(NoopContextPropagator.extract(&carrier).is_empty());
    }

    #[test]
    fn w3c_round_trips_trace_headers() {
        let mut ctx = PropagationContext::default();
        ctx.set(TRACEPARENT_HEADER, b"00-abc-def-01".to_vec());
        ctx.set(BAGGAGE_HEADER, b"k=v".to_vec());
        // A non-W3C key is not propagated by this propagator.
        ctx.set("x-custom", b"ignored".to_vec());

        let propagator = W3CTraceContextPropagator;
        let mut carrier = PropagationCarrier::new();
        propagator.inject(&ctx, &mut carrier);

        assert_eq!(
            carrier.get(TRACEPARENT_HEADER).map(|v| v.as_slice()),
            Some(&b"00-abc-def-01"[..])
        );
        assert_eq!(
            carrier.get(BAGGAGE_HEADER).map(|v| v.as_slice()),
            Some(&b"k=v"[..])
        );
        assert!(!carrier.contains_key("x-custom"));

        let extracted = propagator.extract(&carrier);
        assert_eq!(
            extracted.get(TRACEPARENT_HEADER),
            Some(&b"00-abc-def-01"[..])
        );
        assert_eq!(extracted.get(BAGGAGE_HEADER), Some(&b"k=v"[..]));
        assert_eq!(extracted.get("x-custom"), None);
    }
}
