# Example 16: Domain Management

## Overview

This example demonstrates domain lifecycle management including registration, description, updates, and failover operations.

## Features Demonstrated

- **RegisterDomainRequest**: Creating new domains with configuration
- **DomainDescription**: Inspecting domain properties
- **UpdateDomainRequest**: Modifying domain settings
- **FailoverDomainRequest**: Multi-cluster failover operations
- **Archival Configuration**: History and visibility archival settings

## Key Concepts

### Domain Registration

```rust
let request = RegisterDomainRequest {
    name: "my-domain".to_string(),
    description: Some("My application domain".to_string()),
    owner_email: "admin@example.com".to_string(),
    workflow_execution_retention_period: Duration::from_secs(7 * 24 * 60 * 60),
    emit_metric: true,
    is_global_domain: false,
    history_archival_status: ArchivalStatus::Enabled,
    history_archival_uri: Some("s3://my-bucket/history".to_string()),
    ..Default::default()
};

domain_client.register(request).await?;
```

### Describing a Domain

```rust
let description = domain_client.describe("my-domain").await?;

println!("Domain: {}", description.domain_info.name);
println!("Status: {:?}", description.domain_info.status);
println!("Retention: {:?}", description.configuration.workflow_execution_retention_period);
```

### Updating a Domain

```rust
let update = UpdateDomainRequest {
    name: Some("my-domain".to_string()),
    updated_info: Some(UpdateDomainInfo {
        description: "Updated description".to_string(),
        owner_email: "new-admin@example.com".to_string(),
        data: HashMap::new(),
    }),
    configuration: Some(DomainConfiguration {
        workflow_execution_retention_period: Duration::from_secs(30 * 24 * 60 * 60),
        emit_metric: true,
        history_archival_status: ArchivalStatus::Enabled,
        ..Default::default()
    }),
    ..Default::default()
};

domain_client.update(update).await?;
```

### Domain Failover

```rust
let failover = FailoverDomainRequest {
    name: "global-domain".to_string(),
    clusters: vec!["cluster-us-west".to_string()],
};

domain_client.failover(failover).await?;
```

### Global Domains

Global domains span multiple clusters for disaster recovery:

```rust
let global_domain = RegisterDomainRequest {
    name: "global-app".to_string(),
    is_global_domain: true,
    clusters: vec![
        ClusterReplicationConfiguration { cluster_name: "cluster-east".to_string() },
        ClusterReplicationConfiguration { cluster_name: "cluster-west".to_string() },
    ],
    active_cluster_name: "cluster-east".to_string(),
    ..Default::default()
};
```

## Running the Example

```bash
cargo run -p domain_management
```

## Running Tests

```bash
cargo test -p domain_management
```

## Code Structure

- `src/main.rs` - Example entry point and demonstration
- `src/lib.rs` - Library exports
- `src/domain_ops/` - Domain operation implementations

## Related Examples

- Previous: [15_client_operations](../15_client_operations) - Client API patterns
- Next: [17_worker_configuration](../17_worker_configuration) - Worker setup
