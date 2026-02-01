//! Activity implementations for versioning example.
//!
//! This example demonstrates activities that may have different
//! implementations across workflow versions.

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

/// API call input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallInput {
    pub endpoint: String,
    pub method: String,
    pub payload: serde_json::Value,
}

/// API call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallResult {
    pub call_id: String,
    pub status_code: u16,
    pub response: serde_json::Value,
}

/// Database query input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseQueryInput {
    pub query: String,
    pub parameters: Vec<serde_json::Value>,
    pub use_new_schema: bool, // Version-specific flag
}

/// Database query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseQueryResult {
    pub query_id: String,
    pub rows_affected: u64,
    pub data: Vec<serde_json::Value>,
}

/// External service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_name: String,
    pub endpoint: String,
    pub timeout_seconds: u64,
    pub api_version: String, // e.g., "v1", "v2"
}

/// Make an external API call
pub async fn external_api_call_activity(
    ctx: &ActivityContext,
    input: ApiCallInput,
) -> Result<ApiCallResult, ActivityError> {
    info!(
        "Making API call to {} ({}",
        input.endpoint, input.method
    );
    
    // Simulate API call latency
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Record heartbeat with progress
    ctx.record_heartbeat(Some(&serde_json::to_vec(&serde_json::json!({
        "progress": 50,
        "endpoint": &input.endpoint,
    })).unwrap()));
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    let call_id = Uuid::new_v4().to_string();
    
    info!("API call {} completed", call_id);
    
    Ok(ApiCallResult {
        call_id,
        status_code: 200,
        response: serde_json::json!({
            "success": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    })
}

/// Execute a database query with version-specific handling
pub async fn database_query_activity(
    ctx: &ActivityContext,
    input: DatabaseQueryInput,
) -> Result<DatabaseQueryResult, ActivityError> {
    info!(
        "Executing database query: {} (new schema: {})",
        input.query, input.use_new_schema
    );
    
    // Simulate query execution
    tokio::time::sleep(Duration::from_millis(75)).await;
    
    ctx.record_heartbeat(None);
    
    let query_id = Uuid::new_v4().to_string();
    
    // Simulate different behavior based on schema version
    let rows_affected = if input.use_new_schema {
        5 // New schema returns more data
    } else {
        3 // Old schema returns less data
    };
    
    info!("Query {} completed, {} rows affected", query_id, rows_affected);
    
    let mut data = Vec::new();
    for i in 0..rows_affected {
        data.push(serde_json::json!({
            "id": i + 1,
            "value": format!("data_{}", i),
        }));
    }
    
    Ok(DatabaseQueryResult {
        query_id,
        rows_affected,
        data,
    })
}

/// Get service configuration
pub async fn get_service_config_activity(
    _ctx: &ActivityContext,
    service_name: String,
) -> Result<ServiceConfig, ActivityError> {
    info!("Fetching configuration for service: {}", service_name);
    
    // Simulate config fetch
    tokio::time::sleep(Duration::from_millis(25)).await;
    
    // Return versioned config based on service
    let (endpoint, api_version) = match service_name.as_str() {
        "payment_service" => ("https://api.payment.example.com", "v2"),
        "user_service" => ("https://api.user.example.com", "v1"),
        _ => ("https://api.default.example.com", "v1"),
    };
    
    Ok(ServiceConfig {
        service_name,
        endpoint: endpoint.to_string(),
        timeout_seconds: 30,
        api_version: api_version.to_string(),
    })
}
