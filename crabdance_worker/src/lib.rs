//! Cadence worker implementation.
//!
//! This crate provides the worker for hosting workflow and activity
//! implementations, polling tasks from the Cadence server, and executing
//! them.

pub mod autoscaler;
pub mod executor;
pub mod handlers;
pub mod heartbeat;
pub mod interceptor;
pub mod local_activity_queue;
pub mod metrics;
#[cfg(feature = "otel")]
pub mod otel;
pub mod pollers;
pub mod registry;
pub mod replay_verifier;
#[cfg(test)]
mod resource_tests;
pub mod worker;

pub use interceptor::TimingInterceptor;
pub use local_activity_queue::*;
pub use registry::*;
pub use replay_verifier::*;
pub use worker::*;
