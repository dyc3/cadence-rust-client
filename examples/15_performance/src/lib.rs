//! Example 23: Performance Tuning
//!
//! This example demonstrates performance optimization techniques for Cadence workflows.
//!
//! ## Features Demonstrated
//!
//! - Batched activity execution
//! - Concurrent workflow operations
//! - Memory-efficient data structures
//! - Connection pooling patterns
//! - Worker configuration for high throughput
//!
//! ## Concepts
//!
//! - **Batching**: Grouping multiple operations to reduce overhead
//! - **Concurrency**: Parallel execution of independent activities
//! - **Resource Management**: Efficient memory and connection usage
//! - **Throughput Optimization**: Configuring workers for maximum performance

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
