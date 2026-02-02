//! Client implementation for Cadence workflow orchestration service.
//!
//! This module provides the main client interface for starting workflows,
//! querying workflow state, sending signals, and managing workflow executions.

pub mod client;
pub mod domain;
pub mod grpc;

pub use client::*;
pub use domain::*;
pub use grpc::*;