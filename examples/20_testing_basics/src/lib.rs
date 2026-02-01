//! Example 20: Testing Basics - TestWorkflowEnvironment Deep Dive
//!
//! This example demonstrates comprehensive testing strategies for Cadence workflows:
//! - Unit testing workflows in isolation
//! - Mocking activities
//! - Testing time progression
//! - Testing signals and queries
//! - Testing error scenarios

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
