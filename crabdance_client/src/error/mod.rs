//! Error types for the Cadence client.
//!
//! This module provides layer-specific error types that preserve semantic
//! information and enable precise error handling:
//!
//! - [`TransportError`]: gRPC and network-level failures (re-exported from core)
//! - [`ClientError`]: Client-level validation and workflow operation errors
//!
//! # Error Hierarchy
//!
//! ```text
//! TransportError (gRPC/network)
//!       ↓
//! ClientError (validation, workflow operations)
//!       ↓
//! CadenceError (workflow/activity execution)
//! ```
//!
//! # Example
//!
//! ```no_run
//! use crabdance_client::error::{ClientError, TransportError};
//!
//! // Example of matching on error types
//! fn handle_error(error: ClientError) {
//!     match error {
//!         ClientError::InvalidWorkflowId(id) => {
//!             eprintln!("Invalid workflow ID: {}", id);
//!         },
//!         ClientError::WorkflowNotFound { workflow_id, .. } => {
//!             eprintln!("Workflow not found: {}", workflow_id);
//!         },
//!         ClientError::Transport(transport_err) => {
//!             eprintln!("Transport error: {}", transport_err);
//!         },
//!         e => eprintln!("Error: {}", e),
//!     }
//! }
//! ```

pub mod client;

pub use client::ClientError;
pub use crabdance_core::TransportError;
