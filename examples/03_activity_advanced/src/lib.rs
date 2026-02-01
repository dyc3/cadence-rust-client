//! Example 03: Activity Advanced
//!
//! This example demonstrates activity heartbeats, cancellation handling, and deadlines.
//!
//! ## Features Demonstrated
//!
//! - **Activity Heartbeats**: Long-running activities reporting progress
//! - **Cancellation Handling**: Gracefully handling activity cancellation
//! - **Deadline Management**: Working with activity time limits
//! - **Heartbeat Details**: Resuming from previous attempts with state
//!
//! ## Concepts
//!
//! - **Heartbeats**: Activities report progress to prevent timeouts
//! - **Cancellation**: Activities can be cancelled and should cleanup
//! - **Deadlines**: Maximum time limits for activity execution
//! - **Retry State**: Using heartbeat details to resume interrupted work

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
