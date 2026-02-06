//! Example 17: Worker Configuration
//!
//! This example demonstrates worker setup and tuning:
//! - Worker options and configuration
//! - Activity and workflow registration
//! - Rate limiting and concurrency controls
//! - Sticky execution and session workers
//!
//! ## Features Demonstrated
//!
//! - **WorkerOptions**: Configuring worker behavior
//! - **Registry**: Registering workflows and activities
//! - **Rate Limiting**: Activities per second controls
//! - **Concurrency**: Max concurrent execution settings
//! - **Sticky Execution**: Workflow caching for performance

pub mod activities;
pub mod worker_setup;
pub mod workflows;

pub use activities::*;
pub use worker_setup::*;
pub use workflows::*;
