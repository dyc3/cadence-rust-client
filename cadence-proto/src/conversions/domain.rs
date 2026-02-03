//! Conversions for domain management operations

use crate::generated as pb;
use crate::workflow_service as api;

use super::helpers::*;

// ============================================================================
// Register Domain
// ============================================================================

impl From<api::RegisterDomainRequest> for pb::RegisterDomainRequest {
    fn from(req: api::RegisterDomainRequest) -> Self {
        // Determine if this is a global domain
        let is_global = req.is_global_domain.unwrap_or(false);

        // For local domains, ensure active_cluster_name is truly empty
        // Cadence's persistence layer has conditional checks that fail when
        // is_global_domain=false but active_cluster_name is set (even if empty string)
        let active_cluster_name = if is_global || !req.clusters.is_empty() {
            req.active_cluster_name
        } else {
            // For local domains without clusters, use empty string
            String::new()
        };

        pb::RegisterDomainRequest {
            security_token: req.security_token.unwrap_or_default(),
            name: req.name,
            description: req.description.unwrap_or_default(),
            owner_email: req.owner_email,
            workflow_execution_retention_period: seconds_to_duration(Some(
                req.workflow_execution_retention_period_in_days * 86400, // days to seconds
            )),
            clusters: req
                .clusters
                .into_iter()
                .map(|c| pb::ClusterReplicationConfiguration {
                    cluster_name: c.cluster_name,
                })
                .collect(),
            active_cluster_name,
            data: req.data,
            is_global_domain: is_global,
            history_archival_status: req.history_archival_status.map(|s| s as i32).unwrap_or(0),
            history_archival_uri: req.history_archival_uri.unwrap_or_default(),
            visibility_archival_status: req
                .visibility_archival_status
                .map(|s| s as i32)
                .unwrap_or(0),
            visibility_archival_uri: req.visibility_archival_uri.unwrap_or_default(),
            active_clusters: None,
            active_clusters_by_region: std::collections::HashMap::new(),
        }
    }
}

// ============================================================================
// Describe Domain
// ============================================================================

impl From<api::DescribeDomainRequest> for pb::DescribeDomainRequest {
    fn from(req: api::DescribeDomainRequest) -> Self {
        let describe_by = if let Some(uuid) = req.uuid {
            Some(pb::describe_domain_request::DescribeBy::Id(uuid))
        } else {
            req.name.map(pb::describe_domain_request::DescribeBy::Name)
        };

        pb::DescribeDomainRequest { describe_by }
    }
}

impl From<pb::DescribeDomainResponse> for api::DescribeDomainResponse {
    fn from(resp: pb::DescribeDomainResponse) -> Self {
        if let Some(d) = resp.domain {
            let domain_info = Some(api::DomainInfo {
                name: d.name.clone(),
                status: Some(match d.status {
                    1 => api::DomainStatus::Deprecated,
                    2 => api::DomainStatus::Deleted,
                    _ => api::DomainStatus::Registered,
                }),
                description: d.description.clone(),
                owner_email: d.owner_email.clone(),
                data: d.data.clone(),
                uuid: d.id.clone(),
            });

            let configuration = Some(api::DomainConfiguration {
                workflow_execution_retention_period_in_days: duration_to_seconds(
                    d.workflow_execution_retention_period,
                )
                .unwrap_or(0)
                    / 86400, // seconds to days
                emit_metric: d.bad_binaries.is_some(),
                history_archival_status: Some(match d.history_archival_status {
                    1 => api::ArchivalStatus::Enabled,
                    _ => api::ArchivalStatus::Disabled,
                }),
                history_archival_uri: d.history_archival_uri.clone(),
                visibility_archival_status: Some(match d.visibility_archival_status {
                    1 => api::ArchivalStatus::Enabled,
                    _ => api::ArchivalStatus::Disabled,
                }),
                visibility_archival_uri: d.visibility_archival_uri.clone(),
            });

            let replication_configuration = Some(api::DomainReplicationConfiguration {
                active_cluster_name: d.active_cluster_name,
                clusters: d
                    .clusters
                    .into_iter()
                    .map(|c| api::ClusterReplicationConfiguration {
                        cluster_name: c.cluster_name,
                    })
                    .collect(),
            });

            api::DescribeDomainResponse {
                domain_info,
                configuration,
                replication_configuration,
            }
        } else {
            api::DescribeDomainResponse {
                domain_info: None,
                configuration: None,
                replication_configuration: None,
            }
        }
    }
}

// ============================================================================
// Update Domain
// ============================================================================

impl From<api::UpdateDomainRequest> for pb::UpdateDomainRequest {
    fn from(req: api::UpdateDomainRequest) -> Self {
        let mut update_req = pb::UpdateDomainRequest {
            security_token: req.security_token.unwrap_or_default(),
            name: req.name.unwrap_or_default(),
            update_mask: None,
            description: String::new(),
            owner_email: String::new(),
            data: std::collections::HashMap::new(),
            workflow_execution_retention_period: None,
            bad_binaries: None,
            history_archival_status: 0,
            history_archival_uri: String::new(),
            visibility_archival_status: 0,
            visibility_archival_uri: String::new(),
            delete_bad_binary: req.delete_bad_binary.unwrap_or_default(),
            active_cluster_name: String::new(),
            clusters: Vec::new(),
            failover_timeout: None,
            active_clusters: None,
        };

        // Apply updated_info fields
        if let Some(info) = req.updated_info {
            update_req.description = info.description;
            update_req.owner_email = info.owner_email;
            update_req.data = info.data;
        }

        // Apply configuration fields
        if let Some(config) = req.configuration {
            update_req.workflow_execution_retention_period = seconds_to_duration(Some(
                config.workflow_execution_retention_period_in_days * 86400,
            ));
            update_req.history_archival_status = config
                .history_archival_status
                .map(|s| s as i32)
                .unwrap_or(0);
            update_req.history_archival_uri = config.history_archival_uri;
            update_req.visibility_archival_status = config
                .visibility_archival_status
                .map(|s| s as i32)
                .unwrap_or(0);
            update_req.visibility_archival_uri = config.visibility_archival_uri;
        }

        // Apply replication configuration
        if let Some(repl) = req.replication_configuration {
            update_req.active_cluster_name = repl.active_cluster_name;
            update_req.clusters = repl
                .clusters
                .into_iter()
                .map(|c| pb::ClusterReplicationConfiguration {
                    cluster_name: c.cluster_name,
                })
                .collect();
        }

        update_req
    }
}

impl From<pb::UpdateDomainResponse> for api::UpdateDomainResponse {
    fn from(resp: pb::UpdateDomainResponse) -> Self {
        if let Some(d) = resp.domain {
            let domain_info = Some(api::DomainInfo {
                name: d.name.clone(),
                status: Some(match d.status {
                    1 => api::DomainStatus::Deprecated,
                    2 => api::DomainStatus::Deleted,
                    _ => api::DomainStatus::Registered,
                }),
                description: d.description.clone(),
                owner_email: d.owner_email.clone(),
                data: d.data.clone(),
                uuid: d.id.clone(),
            });

            let configuration = Some(api::DomainConfiguration {
                workflow_execution_retention_period_in_days: duration_to_seconds(
                    d.workflow_execution_retention_period,
                )
                .unwrap_or(0)
                    / 86400,
                emit_metric: d.bad_binaries.is_some(),
                history_archival_status: Some(match d.history_archival_status {
                    1 => api::ArchivalStatus::Enabled,
                    _ => api::ArchivalStatus::Disabled,
                }),
                history_archival_uri: d.history_archival_uri.clone(),
                visibility_archival_status: Some(match d.visibility_archival_status {
                    1 => api::ArchivalStatus::Enabled,
                    _ => api::ArchivalStatus::Disabled,
                }),
                visibility_archival_uri: d.visibility_archival_uri.clone(),
            });

            let replication_configuration = Some(api::DomainReplicationConfiguration {
                active_cluster_name: d.active_cluster_name,
                clusters: d
                    .clusters
                    .into_iter()
                    .map(|c| api::ClusterReplicationConfiguration {
                        cluster_name: c.cluster_name,
                    })
                    .collect(),
            });

            api::UpdateDomainResponse {
                domain_info,
                configuration,
                replication_configuration,
            }
        } else {
            api::UpdateDomainResponse {
                domain_info: None,
                configuration: None,
                replication_configuration: None,
            }
        }
    }
}

// ============================================================================
// Failover Domain
// ============================================================================

impl From<api::FailoverDomainRequest> for pb::FailoverDomainRequest {
    fn from(req: api::FailoverDomainRequest) -> Self {
        // Use the first cluster from the list as the active cluster name
        let active_cluster = req.clusters.first().cloned().unwrap_or_default();

        pb::FailoverDomainRequest {
            domain_name: req.name,
            domain_active_cluster_name: active_cluster,
            active_clusters: None,
            reason: String::new(),
        }
    }
}
