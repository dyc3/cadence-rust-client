//! Example 01: Hello Workflow
//!
//! This example demonstrates basic workflow and activity definitions.
//!
//! ## Features Demonstrated
//!
//! - Defining a simple workflow
//! - Defining an activity with heartbeats
//! - Input/output serialization with serde
//! - Signal handling in workflows
//!
//! ## Concepts
//!
//! - **Workflow**: A durable, fault-tolerant function orchestration
//! - **Activity**: A business logic unit that can fail and be retried
//! - **Heartbeat**: Progress reporting from long-running activities
//! - **Signal**: External events that workflows can receive

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
