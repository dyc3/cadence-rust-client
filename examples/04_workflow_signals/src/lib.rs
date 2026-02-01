//! Example 04: Workflow Signals
//!
//! This example demonstrates workflow signal handling patterns.
//!
//! ## Features Demonstrated
//!
//! - **Signal Reception**: Receiving and processing signals
//! - **Multiple Signals**: Handling different signal types
//! - **Signal Data**: Parsing signal payloads
//! - **Signal Channels**: Using channels for async signal handling
//!
//! ## Concepts
//!
//! - **Signals**: External events that can be sent to running workflows
//! - **Signal Channels**: Async receivers for workflow signals
//! - **Signal Routing**: Different handling based on signal type
//! - **State Updates**: Using signals to update workflow state

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
