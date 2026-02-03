//! Shared utilities and helpers for Cadence examples.
//!
//! This crate provides common types, test helpers, and mock implementations
//! that are reused across multiple examples to reduce duplication.

pub mod activities;
pub mod assertions;
pub mod test_helpers;
pub mod tracing_setup;
pub mod types;

pub use assertions::*;
pub use test_helpers::*;
pub use tracing_setup::*;
