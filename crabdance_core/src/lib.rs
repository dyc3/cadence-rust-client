//! Core types and utilities for the Cadence client.
//!
//! This crate provides the foundational types, error handling, and
//! serialization framework used throughout the Cadence client.

pub mod encoded;
pub mod error;
pub mod propagation;
pub mod resources;
pub mod types;

pub use encoded::*;
pub use error::*;
pub use propagation::{
    ContextPropagator, NoopContextPropagator, PropagationCarrier, PropagationContext,
    W3CTraceContextPropagator, BAGGAGE_HEADER, TRACEPARENT_HEADER, TRACESTATE_HEADER,
};
pub use resources::*;
pub use types::*;
