//! Example 25: Best Practices
//!
//! This example demonstrates idiomatic patterns and best practices for building
//! production-grade Cadence workflows in Rust.
//!
//! ## Features Demonstrated
//!
//! - Type-safe workflow inputs/outputs
//! - Error handling patterns
//! - Idempotency keys and deduplication
//! - Activity context propagation
//! - Structured logging and tracing
//! - Configuration management
//! - Graceful shutdown patterns
//! - Testing best practices
//! - Documentation standards
//! - Versioning strategies

pub mod activities;
pub mod workflows;
pub mod types;
pub mod utils;
pub mod config;

pub use activities::*;
pub use workflows::*;
pub use types::*;
pub use utils::*;
pub use config::*;
