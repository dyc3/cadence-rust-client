//! Example 06: Child Workflows
//!
//! This example demonstrates child workflow patterns.
//!
//! ## Features Demonstrated
//!
//! - **Child Workflow Execution**: Starting child workflows from parent
//! - **Fan-Out Pattern**: Running multiple child workflows in parallel
//! - **Fan-In Pattern**: Collecting results from child workflows
//! - **Error Handling**: Handling child workflow failures
//! - **Cancellation**: Cancelling child workflows
//!
//! ## Concepts
//!
//! - **Child Workflows**: Workflows started by other workflows
//! - **Parent-Child Relationship**: Parent manages child lifecycle
//! - **Parallel Execution**: Running multiple children concurrently
//! - **Result Aggregation**: Combining child results

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
