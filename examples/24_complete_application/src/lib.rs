//! Example 24: Complete Application
//!
//! This example demonstrates a production-ready complete application using Cadence.
//!
//! ## Features Demonstrated
//!
//! - Complete e-commerce order processing system
//! - User registration and authentication flow
//! - Inventory management with reservation patterns
//! - Payment processing with retry logic
//! - Notification system with multiple channels
//! - Error handling and compensation
//! - Distributed tracing and monitoring
//! - Configuration management
//! - Graceful shutdown handling
//!
//! ## Architecture
//!
//! The application follows a microservices-inspired architecture where:
//! - Workflows orchestrate business processes
//! - Activities implement domain logic
//! - Sagas handle long-running transactions
//! - Compensation handles failures

pub mod activities;
pub mod workflows;
pub mod models;

pub use activities::*;
pub use workflows::*;
pub use models::*;
