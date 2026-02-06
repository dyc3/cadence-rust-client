//! Example 15: Client Operations
//!
//! This example demonstrates the Client API for workflow management:
//! - Starting workflows (sync and async)
//! - Signaling workflows
//! - Querying workflow state
//! - Cancelling and terminating workflows
//! - Listing and scanning workflows
//!
//! ## Features Demonstrated
//!
//! - **StartWorkflowOptions**: Configuring workflow execution parameters
//! - **Signal Patterns**: Sending signals to running workflows
//! - **Query Patterns**: Querying workflow state
//! - **Lifecycle Management**: Cancel, terminate, and reset workflows
//! - **Workflow Discovery**: List, scan, and count workflows

pub mod workflows;

pub use workflows::*;
