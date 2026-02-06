//! Example 16: Domain Management
//!
//! This example demonstrates domain lifecycle management:
//! - Registering new domains
//! - Describing domain configuration
//! - Updating domain properties
//! - Domain failover operations
//!
//! ## Features Demonstrated
//!
//! - **RegisterDomainRequest**: Creating new domains with configuration
//! - **DomainDescription**: Inspecting domain properties
//! - **UpdateDomainRequest**: Modifying domain settings
//! - **FailoverDomainRequest**: Multi-cluster failover operations
//! - **Archival Configuration**: History and visibility archival settings

pub mod domain_ops;

pub use domain_ops::*;
