//! Protocol definitions and generated code for the Cadence client.
//!
//! This crate contains the Thrift and Protobuf definitions for communicating
//! with the Cadence server.

// Thrift-generated code from IDL files
pub mod generated;

// Manual protocol type definitions (to be gradually replaced by generated types)
pub mod shared;
pub mod workflow_service;

pub use shared::*;
pub use workflow_service::*;

// Re-export thrift crate for use by other crates
pub use thrift;