//! Core types and utilities for the Cadence client.
//!
//! This crate provides the foundational types, error handling, and
//! serialization framework used throughout the Cadence client.

pub mod error;
pub mod types;
pub mod encoded;

pub use error::*;
pub use types::*;
pub use encoded::*;