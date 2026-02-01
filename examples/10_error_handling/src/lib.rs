//! Example 10: Error Handling
//!
//! This example demonstrates error types and handling in Cadence workflows.
//!
//! ## Features Demonstrated
//!
//! - Activity error types and handling
//! - Workflow error propagation
//! - Retry policies and error classification
//! - Custom error types
//!
//! ## Concepts
//!
//! - **ActivityError**: Errors from activity execution
//! - **WorkflowError**: Errors from workflow execution
//! - **Retry Policy**: Configurable retry behavior for failures
//! - **Error Classification**: Distinguishing retryable vs non-retryable errors

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
