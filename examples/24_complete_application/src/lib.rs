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
pub mod models;
pub mod workflows;

pub use activities::*;
pub use models::*;
pub use workflows::*;
