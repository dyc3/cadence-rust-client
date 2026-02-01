//! Example 21: Workflow Replay
//!
//! This example demonstrates workflow replay and debugging patterns.
//!
//! ## Features Demonstrated
//!
//! - Workflow history replay
//! - Non-determinism detection
//! - Debugging workflow failures
//!
//! ## Concepts
//!
//! - **Workflow Replay**: Replaying workflow history to reproduce issues
//! - **Non-determinism**: Detecting code changes that break workflow determinism

pub mod workflows;

pub use workflows::*;
