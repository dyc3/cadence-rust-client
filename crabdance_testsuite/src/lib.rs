//! Testing utilities for Cadence workflows and activities.
//!
//! This crate provides a test framework for unit testing workflows and
//! activities without requiring a running Cadence server.

pub mod suite;

pub use suite::*;

// Re-export WorkflowError for convenience
pub use crabdance_workflow::context::WorkflowError;
