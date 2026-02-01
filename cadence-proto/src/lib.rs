//! Protocol definitions and generated code for the Cadence client.
//!
//! This crate contains the Thrift and Protobuf definitions for communicating
//! with the Cadence server.

// This will be populated with generated code once we integrate the IDL files
// For now, we define the core protocol types manually

pub mod shared;
pub mod workflow_service;

pub use shared::*;
pub use workflow_service::*;

// Re-export thrift crate for use by other crates
pub use thrift;