//! Conversion functions between hand-written API types and generated protobuf types.
//!
//! This module provides conversions (From/Into implementations) to bridge between
//! the clean hand-written API in `shared.rs` and `workflow_service.rs` with the
//! auto-generated protobuf types in `generated/`.

mod activity;
mod decision;
mod domain;
mod helpers;
mod history;
mod shared;
mod workflow;

// Re-export public items if needed
// (Currently all conversions are implemented via trait impls, so no re-exports needed)
