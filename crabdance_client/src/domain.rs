//! Domain client for managing Cadence domains.
//!
//! This module provides the interface for domain lifecycle management
//! including registration, updates, and failover operations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crabdance_core::CadenceResult;
use crabdance_proto::workflow_service::{self as ws, WorkflowService};

use crate::grpc::GrpcWorkflowServiceClient;

/// Number of seconds in a day, used to convert between the idiomatic
/// [`Duration`] retention period and Cadence's wire format (whole days).
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// Domain client trait for managing domains
#[async_trait]
pub trait DomainClient: Send + Sync {
    /// Register a new domain
    async fn register(&self, request: RegisterDomainRequest) -> CadenceResult<()>;

    /// Describe a domain
    async fn describe(&self, name: &str) -> CadenceResult<DomainDescription>;

    /// Update a domain configuration
    async fn update(&self, request: UpdateDomainRequest) -> CadenceResult<UpdateDomainResponse>;

    /// Failover a domain to another cluster
    async fn failover(&self, request: FailoverDomainRequest) -> CadenceResult<()>;
}

/// Request to register a domain
#[derive(Debug, Clone)]
pub struct RegisterDomainRequest {
    pub name: String,
    pub description: Option<String>,
    pub owner_email: String,
    pub workflow_execution_retention_period: Duration,
    pub emit_metric: bool,
    pub clusters: Vec<ClusterReplicationConfiguration>,
    pub active_cluster_name: String,
    pub data: HashMap<String, String>,
    pub security_token: Option<String>,
    pub is_global_domain: bool,
    pub history_archival_status: ArchivalStatus,
    pub history_archival_uri: Option<String>,
    pub visibility_archival_status: ArchivalStatus,
    pub visibility_archival_uri: Option<String>,
}

impl Default for RegisterDomainRequest {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            owner_email: String::new(),
            workflow_execution_retention_period: Duration::from_secs(2 * 24 * 60 * 60), // 2 days
            emit_metric: true,
            clusters: vec![],
            active_cluster_name: String::new(),
            data: HashMap::new(),
            security_token: None,
            is_global_domain: false,
            history_archival_status: ArchivalStatus::Disabled,
            history_archival_uri: None,
            visibility_archival_status: ArchivalStatus::Disabled,
            visibility_archival_uri: None,
        }
    }
}

/// Cluster replication configuration
#[derive(Debug, Clone)]
pub struct ClusterReplicationConfiguration {
    pub cluster_name: String,
}

/// Archival status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchivalStatus {
    #[default]
    Disabled,
    Enabled,
}

/// Domain description returned by describe
#[derive(Debug, Clone, Default)]
pub struct DomainDescription {
    pub domain_info: DomainInfo,
    pub configuration: DomainConfiguration,
    pub replication_configuration: DomainReplicationConfiguration,
}

/// Domain information
#[derive(Debug, Clone, Default)]
pub struct DomainInfo {
    pub name: String,
    pub status: DomainStatus,
    pub description: String,
    pub owner_email: String,
    pub data: HashMap<String, String>,
    pub uuid: String,
}

/// Domain status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DomainStatus {
    #[default]
    Registered,
    Deprecated,
    Deleted,
}

/// Domain configuration
#[derive(Debug, Clone, Default)]
pub struct DomainConfiguration {
    pub workflow_execution_retention_period: Duration,
    pub emit_metric: bool,
    pub history_archival_status: ArchivalStatus,
    pub history_archival_uri: String,
    pub visibility_archival_status: ArchivalStatus,
    pub visibility_archival_uri: String,
}

/// Domain replication configuration
#[derive(Debug, Clone, Default)]
pub struct DomainReplicationConfiguration {
    pub active_cluster_name: String,
    pub clusters: Vec<ClusterReplicationConfiguration>,
}

/// Request to update a domain
#[derive(Debug, Clone)]
pub struct UpdateDomainRequest {
    pub name: Option<String>,
    pub uuid: Option<String>,
    pub updated_info: Option<UpdateDomainInfo>,
    pub configuration: Option<DomainConfiguration>,
    pub replication_configuration: Option<DomainReplicationConfiguration>,
    pub security_token: Option<String>,
    pub delete_bad_binary: Option<String>,
}

/// Updated domain info
#[derive(Debug, Clone)]
pub struct UpdateDomainInfo {
    pub description: String,
    pub owner_email: String,
    pub data: HashMap<String, String>,
}

/// Response from update domain
#[derive(Debug, Clone)]
pub struct UpdateDomainResponse {
    pub domain_info: DomainInfo,
    pub configuration: DomainConfiguration,
    pub replication_configuration: DomainReplicationConfiguration,
}

/// Request to failover a domain
#[derive(Debug, Clone)]
pub struct FailoverDomainRequest {
    pub name: String,
    pub clusters: Vec<String>,
}

/// Domain client implementation backed by the shared gRPC service client.
///
/// This wraps the same [`GrpcWorkflowServiceClient`] used by the main
/// [`crate::client::WorkflowClient`], translating the idiomatic domain API
/// types in this module to and from the protocol-buffer-facing API types in
/// [`crabdance_proto::workflow_service`].
pub struct DomainClientImpl {
    service: Arc<GrpcWorkflowServiceClient>,
}

impl DomainClientImpl {
    /// Create a new domain client from a connected gRPC service client.
    pub fn new(service: Arc<GrpcWorkflowServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl DomainClient for DomainClientImpl {
    async fn register(&self, request: RegisterDomainRequest) -> CadenceResult<()> {
        self.service
            .register_domain(register_request_to_pb(request))
            .await?;
        Ok(())
    }

    async fn describe(&self, name: &str) -> CadenceResult<DomainDescription> {
        // Mirrors Go's `domainClient.Describe`, which builds a DescribeDomainRequest
        // with only the name set and returns the full domain description.
        let request = ws::DescribeDomainRequest {
            name: Some(name.to_string()),
            uuid: None,
        };
        let response = self.service.describe_domain(request).await?;
        Ok(describe_response_to_domain_description(response))
    }

    async fn update(&self, request: UpdateDomainRequest) -> CadenceResult<UpdateDomainResponse> {
        let response = self
            .service
            .update_domain(update_request_to_pb(request))
            .await?;
        Ok(update_response_from_pb(response))
    }

    async fn failover(&self, request: FailoverDomainRequest) -> CadenceResult<()> {
        // Cadence exposes a dedicated FailoverDomain RPC (matching Go's
        // `domainClient.Failover`), so we mirror that wire behavior rather than
        // issuing an UpdateDomain with an active-cluster change.
        let request = ws::FailoverDomainRequest {
            name: request.name,
            clusters: request.clusters,
        };
        self.service.failover_domain(request).await?;
        Ok(())
    }
}

// ============================================================================
// Conversions between idiomatic domain API types and proto-facing API types
// ============================================================================

/// Convert a retention [`Duration`] to Cadence's wire unit of whole days.
fn duration_to_days(period: Duration) -> i32 {
    (period.as_secs() / SECONDS_PER_DAY) as i32
}

/// Convert Cadence's wire unit of whole days back to a [`Duration`].
fn days_to_duration(days: i32) -> Duration {
    Duration::from_secs(days.max(0) as u64 * SECONDS_PER_DAY)
}

fn archival_status_to_pb(status: ArchivalStatus) -> ws::ArchivalStatus {
    match status {
        ArchivalStatus::Disabled => ws::ArchivalStatus::Disabled,
        ArchivalStatus::Enabled => ws::ArchivalStatus::Enabled,
    }
}

fn archival_status_from_pb(status: Option<ws::ArchivalStatus>) -> ArchivalStatus {
    match status {
        Some(ws::ArchivalStatus::Enabled) => ArchivalStatus::Enabled,
        _ => ArchivalStatus::Disabled,
    }
}

fn domain_status_from_pb(status: Option<ws::DomainStatus>) -> DomainStatus {
    match status {
        Some(ws::DomainStatus::Deprecated) => DomainStatus::Deprecated,
        Some(ws::DomainStatus::Deleted) => DomainStatus::Deleted,
        _ => DomainStatus::Registered,
    }
}

fn cluster_to_pb(cluster: ClusterReplicationConfiguration) -> ws::ClusterReplicationConfiguration {
    ws::ClusterReplicationConfiguration {
        cluster_name: cluster.cluster_name,
    }
}

fn cluster_from_pb(
    cluster: ws::ClusterReplicationConfiguration,
) -> ClusterReplicationConfiguration {
    ClusterReplicationConfiguration {
        cluster_name: cluster.cluster_name,
    }
}

fn configuration_to_pb(config: DomainConfiguration) -> ws::DomainConfiguration {
    ws::DomainConfiguration {
        workflow_execution_retention_period_in_days: duration_to_days(
            config.workflow_execution_retention_period,
        ),
        emit_metric: config.emit_metric,
        history_archival_status: Some(archival_status_to_pb(config.history_archival_status)),
        history_archival_uri: config.history_archival_uri,
        visibility_archival_status: Some(archival_status_to_pb(config.visibility_archival_status)),
        visibility_archival_uri: config.visibility_archival_uri,
    }
}

fn configuration_from_pb(config: ws::DomainConfiguration) -> DomainConfiguration {
    DomainConfiguration {
        workflow_execution_retention_period: days_to_duration(
            config.workflow_execution_retention_period_in_days,
        ),
        emit_metric: config.emit_metric,
        history_archival_status: archival_status_from_pb(config.history_archival_status),
        history_archival_uri: config.history_archival_uri,
        visibility_archival_status: archival_status_from_pb(config.visibility_archival_status),
        visibility_archival_uri: config.visibility_archival_uri,
    }
}

fn replication_to_pb(repl: DomainReplicationConfiguration) -> ws::DomainReplicationConfiguration {
    ws::DomainReplicationConfiguration {
        active_cluster_name: repl.active_cluster_name,
        clusters: repl.clusters.into_iter().map(cluster_to_pb).collect(),
    }
}

fn info_from_pb(info: ws::DomainInfo) -> DomainInfo {
    DomainInfo {
        name: info.name,
        status: domain_status_from_pb(info.status),
        description: info.description,
        owner_email: info.owner_email,
        data: info.data,
        uuid: info.uuid,
    }
}

fn replication_from_pb(repl: ws::DomainReplicationConfiguration) -> DomainReplicationConfiguration {
    DomainReplicationConfiguration {
        active_cluster_name: repl.active_cluster_name,
        clusters: repl.clusters.into_iter().map(cluster_from_pb).collect(),
    }
}

fn register_request_to_pb(req: RegisterDomainRequest) -> ws::RegisterDomainRequest {
    ws::RegisterDomainRequest {
        name: req.name,
        description: req.description,
        owner_email: req.owner_email,
        workflow_execution_retention_period_in_days: duration_to_days(
            req.workflow_execution_retention_period,
        ),
        emit_metric: Some(req.emit_metric),
        clusters: req.clusters.into_iter().map(cluster_to_pb).collect(),
        active_cluster_name: req.active_cluster_name,
        data: req.data,
        security_token: req.security_token,
        is_global_domain: Some(req.is_global_domain),
        history_archival_status: Some(archival_status_to_pb(req.history_archival_status)),
        history_archival_uri: req.history_archival_uri,
        visibility_archival_status: Some(archival_status_to_pb(req.visibility_archival_status)),
        visibility_archival_uri: req.visibility_archival_uri,
    }
}

fn update_request_to_pb(req: UpdateDomainRequest) -> ws::UpdateDomainRequest {
    ws::UpdateDomainRequest {
        name: req.name,
        uuid: req.uuid,
        updated_info: req.updated_info.map(|info| ws::UpdateDomainInfo {
            description: info.description,
            owner_email: info.owner_email,
            data: info.data,
        }),
        configuration: req.configuration.map(configuration_to_pb),
        replication_configuration: req.replication_configuration.map(replication_to_pb),
        security_token: req.security_token,
        delete_bad_binary: req.delete_bad_binary,
    }
}

fn describe_response_to_domain_description(resp: ws::DescribeDomainResponse) -> DomainDescription {
    DomainDescription {
        domain_info: resp.domain_info.map(info_from_pb).unwrap_or_default(),
        configuration: resp
            .configuration
            .map(configuration_from_pb)
            .unwrap_or_default(),
        replication_configuration: resp
            .replication_configuration
            .map(replication_from_pb)
            .unwrap_or_default(),
    }
}

fn update_response_from_pb(resp: ws::UpdateDomainResponse) -> UpdateDomainResponse {
    UpdateDomainResponse {
        domain_info: resp.domain_info.map(info_from_pb).unwrap_or_default(),
        configuration: resp
            .configuration
            .map(configuration_from_pb)
            .unwrap_or_default(),
        replication_configuration: resp
            .replication_configuration
            .map(replication_from_pb)
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Unit tests for the idiomatic <-> proto-facing conversions.
    // ------------------------------------------------------------------

    #[test]
    fn test_duration_days_round_trip_preserves_whole_days() {
        assert_eq!(
            duration_to_days(Duration::from_secs(7 * SECONDS_PER_DAY)),
            7
        );
        assert_eq!(
            days_to_duration(7),
            Duration::from_secs(7 * SECONDS_PER_DAY)
        );
        // Sub-day remainders are truncated to whole days, matching the wire format.
        assert_eq!(
            duration_to_days(Duration::from_secs(SECONDS_PER_DAY + 5)),
            1
        );
    }

    #[test]
    fn test_archival_status_maps_both_directions() {
        assert_eq!(
            archival_status_to_pb(ArchivalStatus::Enabled),
            ws::ArchivalStatus::Enabled
        );
        assert_eq!(
            archival_status_to_pb(ArchivalStatus::Disabled),
            ws::ArchivalStatus::Disabled
        );
        assert_eq!(
            archival_status_from_pb(Some(ws::ArchivalStatus::Enabled)),
            ArchivalStatus::Enabled
        );
        // Absent status defaults to Disabled.
        assert_eq!(archival_status_from_pb(None), ArchivalStatus::Disabled);
    }

    #[test]
    fn test_domain_status_from_pb_defaults_to_registered() {
        assert_eq!(
            domain_status_from_pb(Some(ws::DomainStatus::Deprecated)),
            DomainStatus::Deprecated
        );
        assert_eq!(
            domain_status_from_pb(Some(ws::DomainStatus::Deleted)),
            DomainStatus::Deleted
        );
        assert_eq!(domain_status_from_pb(None), DomainStatus::Registered);
        assert_eq!(
            domain_status_from_pb(Some(ws::DomainStatus::Registered)),
            DomainStatus::Registered
        );
    }

    #[test]
    fn test_register_request_to_pb_covers_all_fields() {
        let mut data = HashMap::new();
        data.insert("team".to_string(), "core".to_string());

        let req = RegisterDomainRequest {
            name: "my-domain".to_string(),
            description: Some("desc".to_string()),
            owner_email: "owner@example.com".to_string(),
            workflow_execution_retention_period: Duration::from_secs(3 * SECONDS_PER_DAY),
            emit_metric: true,
            clusters: vec![ClusterReplicationConfiguration {
                cluster_name: "primary".to_string(),
            }],
            active_cluster_name: "primary".to_string(),
            data: data.clone(),
            security_token: Some("token".to_string()),
            is_global_domain: true,
            history_archival_status: ArchivalStatus::Enabled,
            history_archival_uri: Some("s3://history".to_string()),
            visibility_archival_status: ArchivalStatus::Disabled,
            visibility_archival_uri: None,
        };

        let pb = register_request_to_pb(req);

        assert_eq!(pb.name, "my-domain");
        assert_eq!(pb.description.as_deref(), Some("desc"));
        assert_eq!(pb.owner_email, "owner@example.com");
        assert_eq!(pb.workflow_execution_retention_period_in_days, 3);
        assert_eq!(pb.emit_metric, Some(true));
        assert_eq!(pb.clusters.len(), 1);
        assert_eq!(pb.clusters[0].cluster_name, "primary");
        assert_eq!(pb.active_cluster_name, "primary");
        assert_eq!(pb.data, data);
        assert_eq!(pb.security_token.as_deref(), Some("token"));
        assert_eq!(pb.is_global_domain, Some(true));
        assert_eq!(
            pb.history_archival_status,
            Some(ws::ArchivalStatus::Enabled)
        );
        assert_eq!(pb.history_archival_uri.as_deref(), Some("s3://history"));
        assert_eq!(
            pb.visibility_archival_status,
            Some(ws::ArchivalStatus::Disabled)
        );
        assert_eq!(pb.visibility_archival_uri, None);
    }

    #[test]
    fn test_describe_response_to_domain_description_maps_all_sections() {
        let resp = ws::DescribeDomainResponse {
            domain_info: Some(ws::DomainInfo {
                name: "d".to_string(),
                status: Some(ws::DomainStatus::Deprecated),
                description: "desc".to_string(),
                owner_email: "o@e.com".to_string(),
                data: HashMap::new(),
                uuid: "uuid-1".to_string(),
            }),
            configuration: Some(ws::DomainConfiguration {
                workflow_execution_retention_period_in_days: 5,
                emit_metric: true,
                history_archival_status: Some(ws::ArchivalStatus::Enabled),
                history_archival_uri: "s3://h".to_string(),
                visibility_archival_status: None,
                visibility_archival_uri: String::new(),
            }),
            replication_configuration: Some(ws::DomainReplicationConfiguration {
                active_cluster_name: "primary".to_string(),
                clusters: vec![ws::ClusterReplicationConfiguration {
                    cluster_name: "primary".to_string(),
                }],
            }),
        };

        let desc = describe_response_to_domain_description(resp);

        assert_eq!(desc.domain_info.name, "d");
        assert_eq!(desc.domain_info.status, DomainStatus::Deprecated);
        assert_eq!(desc.domain_info.uuid, "uuid-1");
        assert_eq!(
            desc.configuration.workflow_execution_retention_period,
            Duration::from_secs(5 * SECONDS_PER_DAY)
        );
        assert_eq!(
            desc.configuration.history_archival_status,
            ArchivalStatus::Enabled
        );
        assert_eq!(
            desc.configuration.visibility_archival_status,
            ArchivalStatus::Disabled
        );
        assert_eq!(
            desc.replication_configuration.active_cluster_name,
            "primary"
        );
        assert_eq!(desc.replication_configuration.clusters.len(), 1);
    }

    #[test]
    fn test_describe_response_with_empty_sections_uses_defaults() {
        let resp = ws::DescribeDomainResponse {
            domain_info: None,
            configuration: None,
            replication_configuration: None,
        };

        let desc = describe_response_to_domain_description(resp);

        assert_eq!(desc.domain_info.name, "");
        assert_eq!(desc.domain_info.status, DomainStatus::Registered);
        assert_eq!(
            desc.configuration.history_archival_status,
            ArchivalStatus::Disabled
        );
        assert!(desc.replication_configuration.clusters.is_empty());
    }

    #[test]
    fn test_update_request_to_pb_maps_optional_sections() {
        let req = UpdateDomainRequest {
            name: Some("d".to_string()),
            uuid: None,
            updated_info: Some(UpdateDomainInfo {
                description: "new desc".to_string(),
                owner_email: "new@e.com".to_string(),
                data: HashMap::new(),
            }),
            configuration: Some(DomainConfiguration {
                workflow_execution_retention_period: Duration::from_secs(10 * SECONDS_PER_DAY),
                emit_metric: false,
                history_archival_status: ArchivalStatus::Enabled,
                history_archival_uri: "s3://h".to_string(),
                visibility_archival_status: ArchivalStatus::Enabled,
                visibility_archival_uri: "s3://v".to_string(),
            }),
            replication_configuration: Some(DomainReplicationConfiguration {
                active_cluster_name: "secondary".to_string(),
                clusters: vec![ClusterReplicationConfiguration {
                    cluster_name: "secondary".to_string(),
                }],
            }),
            security_token: Some("tok".to_string()),
            delete_bad_binary: Some("checksum".to_string()),
        };

        let pb = update_request_to_pb(req);

        assert_eq!(pb.name.as_deref(), Some("d"));
        let info = pb.updated_info.expect("updated_info present");
        assert_eq!(info.description, "new desc");
        assert_eq!(info.owner_email, "new@e.com");
        let config = pb.configuration.expect("configuration present");
        assert_eq!(config.workflow_execution_retention_period_in_days, 10);
        assert_eq!(
            config.visibility_archival_status,
            Some(ws::ArchivalStatus::Enabled)
        );
        let repl = pb
            .replication_configuration
            .expect("replication configuration present");
        assert_eq!(repl.active_cluster_name, "secondary");
        assert_eq!(repl.clusters[0].cluster_name, "secondary");
        assert_eq!(pb.security_token.as_deref(), Some("tok"));
        assert_eq!(pb.delete_bad_binary.as_deref(), Some("checksum"));
    }

    #[test]
    fn test_update_request_to_pb_omits_absent_sections() {
        let req = UpdateDomainRequest {
            name: Some("d".to_string()),
            uuid: None,
            updated_info: None,
            configuration: None,
            replication_configuration: None,
            security_token: None,
            delete_bad_binary: None,
        };

        let pb = update_request_to_pb(req);

        assert!(pb.updated_info.is_none());
        assert!(pb.configuration.is_none());
        assert!(pb.replication_configuration.is_none());
    }

    // ------------------------------------------------------------------
    // Integration tests against a live Cadence server (cargo test -- --ignored).
    // ------------------------------------------------------------------

    const CADENCE_GRPC_ENDPOINT: &str = "http://localhost:7833";

    fn unique_domain_name() -> String {
        format!("domain-client-it-{}", uuid::Uuid::new_v4())
    }

    async fn connect_domain_client(domain: &str) -> DomainClientImpl {
        let service = Arc::new(
            GrpcWorkflowServiceClient::connect(CADENCE_GRPC_ENDPOINT, domain, None)
                .await
                .expect("Failed to connect to Cadence gRPC server on port 7833"),
        );
        DomainClientImpl::new(service)
    }

    #[tokio::test]
    #[ignore]
    async fn test_register_and_describe_domain_via_domain_client() {
        let name = unique_domain_name();
        let client = connect_domain_client(&name).await;

        let request = RegisterDomainRequest {
            name: name.clone(),
            description: Some("DomainClient integration test".to_string()),
            owner_email: "test@example.com".to_string(),
            workflow_execution_retention_period: Duration::from_secs(SECONDS_PER_DAY),
            ..Default::default()
        };
        client.register(request).await.expect("register failed");

        // Allow the domain to propagate through the server cache.
        tokio::time::sleep(Duration::from_millis(1500)).await;

        let description = client.describe(&name).await.expect("describe failed");
        assert_eq!(description.domain_info.name, name);
        assert_eq!(
            description.domain_info.description,
            "DomainClient integration test"
        );
        assert_eq!(description.domain_info.status, DomainStatus::Registered);
        assert_eq!(
            description
                .configuration
                .workflow_execution_retention_period,
            Duration::from_secs(SECONDS_PER_DAY)
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_update_domain_via_domain_client() {
        let name = unique_domain_name();
        let client = connect_domain_client(&name).await;

        let request = RegisterDomainRequest {
            name: name.clone(),
            owner_email: "test@example.com".to_string(),
            workflow_execution_retention_period: Duration::from_secs(SECONDS_PER_DAY),
            ..Default::default()
        };
        client.register(request).await.expect("register failed");
        tokio::time::sleep(Duration::from_millis(1500)).await;

        let update = UpdateDomainRequest {
            name: Some(name.clone()),
            uuid: None,
            updated_info: Some(UpdateDomainInfo {
                description: "updated description".to_string(),
                owner_email: "updated@example.com".to_string(),
                data: HashMap::new(),
            }),
            configuration: None,
            replication_configuration: None,
            security_token: None,
            delete_bad_binary: None,
        };

        let response = client.update(update).await.expect("update failed");
        assert_eq!(response.domain_info.name, name);
        assert_eq!(response.domain_info.description, "updated description");
        assert_eq!(response.domain_info.owner_email, "updated@example.com");
    }
}
