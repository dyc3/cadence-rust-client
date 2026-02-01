//! Example 02: Activity Basics
//!
//! This example demonstrates activity chaining and composition patterns.
//!
//! ## Features Demonstrated
//!
//! - Multiple activities in a single workflow
//! - Activity chaining (output of one activity as input to another)
//! - Activity error propagation
//! - Activity result handling
//!
//! ## Concepts
//!
//! - **Activity Composition**: Building complex workflows by composing simple activities
//! - **Data Flow**: Passing data between activities
//! - **Error Handling**: How activity errors propagate to workflows

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
