//! Example 05: Workflow External
//!
//! This example demonstrates external workflow signaling and cancellation.
//!
//! ## Features Demonstrated
//!
//! - **External Workflow Signaling**: Sending signals to other workflows
//! - **External Cancellation**: Requesting cancellation of other workflows
//! - **Workflow Orchestration**: Coordinating multiple workflows
//! - **Cross-Workflow Communication**: Communication between workflows
//!
//! ## Concepts
//!
//! - **External Signals**: Signaling workflows by ID
//! - **Cancellation Requests**: Gracefully stopping other workflows
//! - **Parent-Child Coordination**: Managing related workflows
//! - **Workflow Discovery**: Finding and interacting with workflows

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
