//! # Example 16: Domain Management
//!
//! This example demonstrates domain lifecycle management operations.
//!
//! ## Features Demonstrated
//!
//! - Registering new domains with configuration
//! - Describing domain properties
//! - Updating domain settings
//! - Domain failover operations
//! - Archival configuration
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p domain_management
//! ```

use domain_management::*;
use examples_common::tracing_setup::init_tracing;
use std::collections::HashMap;
use std::time::Duration;
use uber_cadence_client::domain::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Domain Management Example ===\n");
    println!("This example demonstrates:");
    println!("1. Domain registration with full configuration");
    println!("2. Domain description and inspection");
    println!("3. Domain updates and modifications");
    println!("4. Domain failover for multi-cluster setups");
    println!("5. Archival configuration");
    println!();

    // Demonstrate domain registration
    demonstrate_domain_registration();

    // Demonstrate domain updates
    demonstrate_domain_updates();

    // Demonstrate failover
    demonstrate_failover();

    // Demonstrate archival configuration
    demonstrate_archival();

    println!("\nExample completed successfully!");
    println!("\nTo run tests:");
    println!("  cargo test -p domain_management");

    Ok(())
}

fn demonstrate_domain_registration() {
    println!("\n--- Domain Registration ---\n");

    // Create a basic domain registration
    let basic_domain = create_domain_registration_request("my-app-domain", "admin@mycompany.com");

    println!("Domain Registration Request:");
    println!("  Name: {}", basic_domain.name);
    println!("  Description: {:?}", basic_domain.description);
    println!("  Owner: {}", basic_domain.owner_email);
    println!(
        "  Retention Period: {:?}",
        basic_domain.workflow_execution_retention_period
    );
    println!("  Emit Metrics: {}", basic_domain.emit_metric);
    println!("  Is Global: {}", basic_domain.is_global_domain);

    // Create a global domain for multi-cluster
    let global_domain = create_global_domain_request(
        "my-global-domain",
        "admin@mycompany.com",
        vec!["cluster-us-east", "cluster-us-west", "cluster-eu-west"],
    );

    println!("\nGlobal Domain Registration:");
    println!("  Name: {}", global_domain.name);
    println!(
        "  Clusters: {:?}",
        global_domain
            .clusters
            .iter()
            .map(|c| &c.cluster_name)
            .collect::<Vec<_>>()
    );
    println!("  Active Cluster: {}", global_domain.active_cluster_name);
    println!(
        "  Retention Period: {:?}",
        global_domain.workflow_execution_retention_period
    );

    println!("\nKey Configuration Options:");
    println!("  - workflow_execution_retention_period: How long to keep workflow history");
    println!("  - emit_metric: Enable/disable domain metrics");
    println!("  - is_global_domain: Enable multi-cluster replication");
    println!("  - clusters: List of clusters for global domains");
    println!("  - active_cluster_name: Current primary cluster");
}

fn demonstrate_domain_updates() {
    println!("\n--- Domain Updates ---\n");

    let update_request = create_domain_update_request(
        "my-app-domain",
        "Updated description for production domain",
        "new-admin@mycompany.com",
    );

    println!("Domain Update Request:");
    println!("  Target: {:?}", update_request.name);

    if let Some(info) = &update_request.updated_info {
        println!("  New Description: {}", info.description);
        println!("  New Owner: {}", info.owner_email);
        println!("  Data Fields: {:?}", info.data);
    }

    if let Some(config) = &update_request.configuration {
        println!("\nConfiguration Updates:");
        println!(
            "  Retention Period: {:?}",
            config.workflow_execution_retention_period
        );
        println!("  Emit Metrics: {}", config.emit_metric);
    }

    println!("\nUpdateable Fields:");
    println!("  - Description");
    println!("  - Owner Email");
    println!("  - Data (custom key-value pairs)");
    println!("  - Retention Period");
    println!("  - Archival Settings");
    println!("  - Replication Configuration");
}

fn demonstrate_failover() {
    println!("\n--- Domain Failover ---\n");

    let failover_request = create_failover_request("my-global-domain", "cluster-us-west");

    println!("Failover Request:");
    println!("  Domain: {}", failover_request.name);
    println!("  Target Clusters: {:?}", failover_request.clusters);

    println!("\nFailover Scenarios:");
    println!("  1. Planned Failover - Switch to different cluster for maintenance");
    println!("  2. Disaster Recovery - Failover due to cluster outage");
    println!("  3. Load Balancing - Distribute load across clusters");

    println!("\nFailover Process:");
    println!("  1. Stop new workflow starts on current cluster");
    println!("  2. Wait for in-flight workflows to complete");
    println!("  3. Update active cluster in domain config");
    println!("  4. Resume operations on new cluster");
}

fn demonstrate_archival() {
    println!("\n--- Archival Configuration ---\n");

    println!("Archival Types:");
    println!("  1. History Archival - Workflow execution history");
    println!("  2. Visibility Archival - Workflow visibility records");

    println!("\nArchival Status Options:");
    let statuses = vec![
        (
            ArchivalStatus::Disabled,
            "No archival, data deleted after retention",
        ),
        (ArchivalStatus::Enabled, "Data archived to specified URI"),
    ];

    for (status, desc) in statuses {
        println!("  {:?}: {}", status, desc);
    }

    println!("\nArchival URIs:");
    println!("  S3: s3://bucket-name/prefix");
    println!("  GCS: gcs://bucket-name/prefix");
    println!("  File: file:///path/to/archive");

    println!("\nExample Configuration:");
    let archival_config = RegisterDomainRequest {
        name: "archived-domain".to_string(),
        description: Some("Domain with archival enabled".to_string()),
        owner_email: "admin@example.com".to_string(),
        workflow_execution_retention_period: Duration::from_secs(2 * 24 * 60 * 60),
        emit_metric: true,
        clusters: vec![],
        active_cluster_name: "cluster-1".to_string(),
        data: HashMap::new(),
        security_token: None,
        is_global_domain: false,
        history_archival_status: ArchivalStatus::Enabled,
        history_archival_uri: Some("s3://my-cadence-archive/history".to_string()),
        visibility_archival_status: ArchivalStatus::Enabled,
        visibility_archival_uri: Some("s3://my-cadence-archive/visibility".to_string()),
    };

    println!(
        "  History Archival: {:?}",
        archival_config.history_archival_status
    );
    println!("  History URI: {:?}", archival_config.history_archival_uri);
    println!(
        "  Visibility Archival: {:?}",
        archival_config.visibility_archival_status
    );
    println!(
        "  Visibility URI: {:?}",
        archival_config.visibility_archival_uri
    );
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_domain_registration_request() {
//         let request = create_domain_registration_request(
//             "test-domain",
//             "test@example.com"
//         );
//
//         assert_eq!(request.name, "test-domain");
//         assert_eq!(request.owner_email, "test@example.com");
//         assert!(!request.is_global_domain);
//         assert_eq!(request.clusters.len(), 1);
//     }
//
//     #[test]
//     fn test_global_domain_request() {
//         let request = create_global_domain_request(
//             "global-test",
//             "test@example.com",
//             vec!["cluster-1", "cluster-2"]
//         );
//
//         assert_eq!(request.name, "global-test");
//         assert!(request.is_global_domain);
//         assert_eq!(request.clusters.len(), 2);
//     }
//
//     #[test]
//     fn test_domain_update_request() {
//         let request = create_domain_update_request(
//             "test-domain",
//             "Updated desc",
//             "new-owner@example.com"
//         );
//
//         assert_eq!(request.name, Some("test-domain".to_string()));
//
//         let info = request.updated_info.unwrap();
//         assert_eq!(info.description, "Updated desc");
//         assert_eq!(info.owner_email, "new-owner@example.com");
//     }
//
//     #[test]
//     fn test_failover_request() {
//         let request = create_failover_request("my-domain", "cluster-west");
//
//         assert_eq!(request.name, "my-domain");
//         assert_eq!(request.clusters, vec!["cluster-west"]);
//     }
//
//     #[test]
//     fn test_archival_status() {
//         assert_ne!(
//             std::mem::discriminant(&ArchivalStatus::Enabled),
//             std::mem::discriminant(&ArchivalStatus::Disabled)
//         );
//     }
//
//     #[tokio::test]
//     async fn test_domain_lifecycle() {
//         let result = demonstrate_domain_lifecycle().await;
//         assert!(result.is_ok());
//     }
// }
