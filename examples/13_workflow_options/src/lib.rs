//! Example 13: Workflow Options
//!
//! This example demonstrates workflow lifecycle policies and configuration options.
//!
//! ## Features Demonstrated
//!
//! - Workflow timeouts and limits
//! - Workflow ID reuse policies
//! - Workflow retry policies
//! - Cron workflows
//! - Parent-close policies
//! - Memo and headers
//!
//! ## Concepts
//!
//! - **Workflow Options**: Configuration for workflow execution
//! - **Timeout Policies**: Various timeout configurations (execution, run, task)
//! - **ID Reuse Policy**: How to handle duplicate workflow IDs
//! - **Cron Schedule**: Recurring workflow execution
//! - **Parent-Close Policy**: Behavior when parent workflow completes

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
