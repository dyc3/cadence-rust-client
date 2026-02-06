//! Example 18: Local Activities
//!
//! This example demonstrates local activity execution patterns:
//! - Executing activities synchronously in workflow thread
//! - Local activity options and retry policies
//! - When to use local activities vs regular activities
//! - Performance considerations
//!
//! ## Features Demonstrated
//!
//! - **LocalActivityOptions**: Configuring local activity execution
//! - **execute_local_activity**: Running activities synchronously
//! - **Retry Policies**: Configuring retry behavior for local activities
//! - **Use Cases**: Short, fast activities that don't need task queue

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
