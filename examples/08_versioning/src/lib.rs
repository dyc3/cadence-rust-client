//! Example 08: Versioning
//!
//! This example demonstrates workflow versioning and side effects.
//!
//! ## Features Demonstrated
//!
//! - Workflow versioning for backwards compatibility
//! - Side effects and their handling
//! - Mutable side effects
//! - Version-specific behavior
//!
//! ## Concepts
//!
//! - **Versioning**: Managing changes to workflow logic over time
//! - **Side Effect**: Non-deterministic operations that need special handling
//! - **Mutable Side Effect**: Side effects that can change between runs

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
