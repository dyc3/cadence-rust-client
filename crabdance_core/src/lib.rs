//! Core types and utilities for the Cadence client.
//!
//! This crate provides the foundational types, error handling, and
//! serialization framework used throughout the Cadence client.

pub mod encoded;
pub mod error;
pub mod types;

pub use encoded::*;
pub use error::*;
pub use types::*;
