//! Example 14: Search Attributes
//!
//! This example demonstrates searchable workflows using Elasticsearch integration.
//!
//! ## Features Demonstrated
//!
//! - Setting search attributes on workflows
//! - Querying workflows by search attributes
//! - Updating search attributes during execution
//! - Search attribute types (keyword, int, datetime, bool)
//! - Listing workflows with filters
//!
//! ## Concepts
//!
//! - **Search Attributes**: Indexed metadata for workflow discovery
//! - **Elasticsearch**: Backend for workflow search capabilities
//! - **Keyword**: String-based searchable field
//! - **Indexed Fields**: Fields that can be used in queries
//! - **Dynamic Updates**: Modifying search attributes during workflow execution

pub mod activities;
pub mod workflows;

pub use activities::*;
pub use workflows::*;
