//! # Crabdance - Rust Client for Uber Cadence
//!
//! This is a meta-crate that re-exports all Crabdance crates for convenient access.
//!
//! ## Usage
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! crabdance = "0.1.0"
//! ```
//!
//! Then use the crates like this:
//!
//! ```rust,no_run
//! use crabdance::client::WorkflowClient;
//! use crabdance::core::{CadenceError, CadenceResult};
//! use crabdance::workflow::WorkflowContext;
//! use crabdance::activity::ActivityContext;
//! ```
//!
//! ## Re-exported Crates
//!
//! - [`proto`] - Protocol definitions and generated types
//! - [`core`] - Core types, errors, and serialization
//! - [`macros`] - Procedural macros for workflows and activities
//! - [`client`] - Client for workflow operations
//! - [`worker`] - Worker for hosting workflows and activities
//! - [`workflow`] - Workflow authoring SDK
//! - [`activity`] - Activity authoring SDK
//! - [`testsuite`] - Testing utilities

pub use crabdance_activity as activity;
pub use crabdance_client as client;
pub use crabdance_core as core;
pub use crabdance_macros as macros;
pub use crabdance_proto as proto;
pub use crabdance_testsuite as testsuite;
pub use crabdance_worker as worker;
pub use crabdance_workflow as workflow;
pub use serde_json;
