//! Domain operations for domain management example.
//!
//! This module demonstrates domain lifecycle operations including
//! registration, description, updates, and failover.

use cadence_client::domain::*;
use std::collections::HashMap;
use std::time::Duration;
use tracing::info;

/// Example: Register a new domain with full configuration
pub fn create_domain_registration_request(name: &str, owner_email: &str) -> RegisterDomainRequest {
    let mut data = HashMap::new();
    data.insert("created_by".to_string(), "example_16".to_string());
    data.insert("environment".to_string(), "development".to_string());

    RegisterDomainRequest {
        name: name.to_string(),
        description: Some(format!("Domain {} created by example 16", name)),
        owner_email: owner_email.to_string(),
        workflow_execution_retention_period: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
        emit_metric: true,
        clusters: vec![ClusterReplicationConfiguration {
            cluster_name: "cluster-1".to_string(),
        }],
        active_cluster_name: "cluster-1".to_string(),
        data,
        security_token: None,
        is_global_domain: false,
        history_archival_status: ArchivalStatus::Enabled,
        history_archival_uri: Some("s3://my-bucket/history".to_string()),
        visibility_archival_status: ArchivalStatus::Enabled,
        visibility_archival_uri: Some("s3://my-bucket/visibility".to_string()),
    }
}

/// Example: Create a global domain request for multi-cluster setup
pub fn create_global_domain_request(
    name: &str,
    owner_email: &str,
    clusters: Vec<&str>,
) -> RegisterDomainRequest {
    let cluster_configs: Vec<ClusterReplicationConfiguration> = clusters
        .into_iter()
        .map(|name| ClusterReplicationConfiguration {
            cluster_name: name.to_string(),
        })
        .collect();

    RegisterDomainRequest {
        name: name.to_string(),
        description: Some("Global domain for disaster recovery".to_string()),
        owner_email: owner_email.to_string(),
        workflow_execution_retention_period: Duration::from_secs(14 * 24 * 60 * 60), // 14 days
        emit_metric: true,
        clusters: cluster_configs.clone(),
        active_cluster_name: cluster_configs[0].cluster_name.clone(),
        data: HashMap::new(),
        security_token: None,
        is_global_domain: true,
        history_archival_status: ArchivalStatus::Enabled,
        history_archival_uri: Some("s3://global-bucket/history".to_string()),
        visibility_archival_status: ArchivalStatus::Enabled,
        visibility_archival_uri: Some("s3://global-bucket/visibility".to_string()),
    }
}

/// Example: Create a domain update request
pub fn create_domain_update_request(
    name: &str,
    new_description: &str,
    new_owner: &str,
) -> UpdateDomainRequest {
    let mut data = HashMap::new();
    data.insert("last_updated".to_string(), chrono::Utc::now().to_rfc3339());
    data.insert("updated_by".to_string(), "example_16".to_string());

    UpdateDomainRequest {
        name: Some(name.to_string()),
        uuid: None,
        updated_info: Some(UpdateDomainInfo {
            description: new_description.to_string(),
            owner_email: new_owner.to_string(),
            data,
        }),
        configuration: Some(DomainConfiguration {
            workflow_execution_retention_period: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            emit_metric: true,
            history_archival_status: ArchivalStatus::Enabled,
            history_archival_uri: "s3://updated-bucket/history".to_string(),
            visibility_archival_status: ArchivalStatus::Enabled,
            visibility_archival_uri: "s3://updated-bucket/visibility".to_string(),
        }),
        replication_configuration: None,
        security_token: None,
        delete_bad_binary: None,
    }
}

/// Example: Create a failover request
pub fn create_failover_request(domain_name: &str, target_cluster: &str) -> FailoverDomainRequest {
    FailoverDomainRequest {
        name: domain_name.to_string(),
        clusters: vec![target_cluster.to_string()],
    }
}

/// Helper to print domain information
pub fn print_domain_info(description: &DomainDescription) {
    info!("Domain Information:");
    info!("  Name: {}", description.domain_info.name);
    info!("  Status: {:?}", description.domain_info.status);
    info!("  Description: {}", description.domain_info.description);
    info!("  Owner: {}", description.domain_info.owner_email);
    info!("  UUID: {}", description.domain_info.uuid);

    info!("Configuration:");
    info!(
        "  Retention Period: {:?}",
        description
            .configuration
            .workflow_execution_retention_period
    );
    info!("  Emit Metrics: {}", description.configuration.emit_metric);
    info!(
        "  History Archival: {:?}",
        description.configuration.history_archival_status
    );
    info!(
        "  Visibility Archival: {:?}",
        description.configuration.visibility_archival_status
    );

    info!("Replication:");
    info!(
        "  Active Cluster: {}",
        description.replication_configuration.active_cluster_name
    );
    info!(
        "  Clusters: {:?}",
        description
            .replication_configuration
            .clusters
            .iter()
            .map(|c| c.cluster_name.clone())
            .collect::<Vec<_>>()
    );
}

/// Example workflow demonstrating domain operations
pub async fn demonstrate_domain_lifecycle() -> Result<(), String> {
    info!("Starting domain lifecycle demonstration");

    // 1. Create registration request
    let register_request =
        create_domain_registration_request("example-domain", "admin@example.com");
    info!(
        "Created registration request for domain: {}",
        register_request.name
    );

    // 2. Create global domain request
    let global_request = create_global_domain_request(
        "global-example-domain",
        "admin@example.com",
        vec!["cluster-1", "cluster-2"],
    );
    info!("Created global domain request: {}", global_request.name);

    // 3. Create update request
    let _update_request = create_domain_update_request(
        "example-domain",
        "Updated description",
        "new-owner@example.com",
    );
    info!("Created update request for domain");

    // 4. Create failover request
    let failover_request = create_failover_request("global-example-domain", "cluster-2");
    info!(
        "Created failover request to cluster: {}",
        failover_request.clusters[0]
    );

    info!("Domain lifecycle demonstration completed");
    Ok(())
}
