//! Testing utilities for Cadence workflows and activities.
//!
//! This crate provides a test framework for unit testing workflows and
//! activities without requiring a running Cadence server.

pub mod env;
pub mod replay;
pub mod suite;

pub use env::{TestRunResult, WorkflowTestEnv};
pub use replay::{RecordedEvent, RecordedHistory, ReplayError};
pub use suite::*;

// Re-export WorkflowError for convenience
pub use crabdance_workflow::context::WorkflowError;
