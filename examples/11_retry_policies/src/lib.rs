//! Example 11: Retry Policies
//!
//! This example demonstrates various retry configuration patterns for activities and workflows.
//!
//! ## Features Demonstrated
//!
//! - Exponential backoff retry policies
//! - Maximum retry attempts configuration
//! - Activity-specific retry policies
//! - Workflow retry policies
//! - Non-retryable error types
//! - Custom retry predicates
//!
//! ## Concepts
//!
//! - **Retry Policy**: Controls how and when failed activities are retried
//! - **Backoff Strategy**: How delay increases between retry attempts
//! - **Maximum Attempts**: Upper limit on retry attempts
//! - **Non-retryable Errors**: Error types that should not trigger retries

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
