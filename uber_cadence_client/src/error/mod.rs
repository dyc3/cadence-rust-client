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
//! use uber_cadence_client::{WorkflowClient, error::{ClientError, TransportError}};
//! use tonic::Code;
//!
//! async fn handle_workflow_start(client: &WorkflowClient) {
//!     match client.start_workflow(/* ... */).await {
//!         Ok(execution) => println!("Started: {:?}", execution),
//!         Err(ClientError::InvalidWorkflowId(id)) => {
//!             eprintln!("Invalid workflow ID: {}", id);
//!         },
//!         Err(ClientError::Transport(TransportError::GrpcStatus {
//!             code: Code::AlreadyExists, ..
//!         })) => {
//!             eprintln!("Workflow already exists");
//!         },
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! ```

pub mod client;

pub use client::ClientError;
pub use uber_cadence_core::TransportError;
