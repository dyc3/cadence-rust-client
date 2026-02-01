//! Example 12: Query Operations
//!
//! This example demonstrates querying workflow state for debugging and monitoring.
//!
//! ## Features Demonstrated
//!
//! - Query handlers in workflows
//! - Query types and responses
//! - Async query execution
//! - Query error handling
//! - Workflow state inspection
//! - Query usage patterns
//!
//! ## Concepts
//!
//! - **Query**: Read-only operation to inspect workflow state
//! - **Query Handler**: Function that responds to queries
//! - **Query Response**: Typed response from workflow queries
//! - **State Inspection**: Viewing workflow progress without modification

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
