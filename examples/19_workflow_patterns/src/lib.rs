//! Example 19: Advanced Workflow Patterns
//!
//! This example demonstrates advanced workflow patterns including:
//! - Saga pattern for distributed transactions with compensation
//! - Fan-out/Fan-in pattern for parallel execution
//! - Retry and circuit breaker patterns
//!
//! ## Features Demonstrated
//!
//! - **Saga Pattern**: Compensating transactions for distributed operations
//! - **Fan-out/Fan-in**: Parallel activity execution with result aggregation
//! - **Compensation**: Automatic rollback on failure
//! - **Error Handling**: Graceful degradation and recovery

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
