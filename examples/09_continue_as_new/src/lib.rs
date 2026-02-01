//! Example 09: Continue As New
//!
//! This example demonstrates the continue-as-new pattern for long-running workflows.
//!
//! ## Features Demonstrated
//!
//! - Continue-as-new workflow pattern
//! - State transfer between workflow runs
//! - Iteration limits and pagination
//! - Long-running workflow management
//!
//! ## Concepts
//!
//! - **Continue As New**: Completing a workflow and immediately starting a new one
//! - **State Transfer**: Passing state from the old workflow to the new one
//! - **Pagination**: Processing large datasets in chunks across workflow runs

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
