//! Workflow implementations for versioning example.
//!
//! This example demonstrates workflow versioning patterns and side effects
//! for handling non-deterministic operations.

use crate::activities::*;
use cadence_core::ActivityOptions;
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use tracing::{info, warn};
use std::time::Duration;

/// A workflow demonstrating versioned behavior
pub async fn versioned_workflow_v1(
    ctx: &mut WorkflowContext,
    input: String,
) -> Result<String, WorkflowError> {
    info!("Running versioned workflow v1 with input: {}", input);
    
    // V1: Simple API call
    let api_input = ApiCallInput {
        endpoint: "/api/v1/process".to_string(),
        method: "POST".to_string(),
        payload: serde_json::json!({ "data": input }),
    };
    
    let result = ctx
        .execute_activity(
            "external_api_call",
            Some(serde_json::to_vec(&api_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let api_result: ApiCallResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse API result: {}", e)))?;
    
    Ok(format!("V1 Result: {:?}", api_result.status_code))
}

/// Updated version of the workflow with additional steps
pub async fn versioned_workflow_v2(
    ctx: &mut WorkflowContext,
    input: String,
) -> Result<String, WorkflowError> {
    info!("Running versioned workflow v2 with input: {}", input);
    
    // V2: API call followed by database query
    let api_input = ApiCallInput {
        endpoint: "/api/v2/process".to_string(),
        method: "POST".to_string(),
        payload: serde_json::json!({ 
            "data": input,
            "version": "v2",
        }),
    };
    
    let api_result = ctx
        .execute_activity(
            "external_api_call",
            Some(serde_json::to_vec(&api_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let api_response: ApiCallResult = serde_json::from_slice(&api_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse API result: {}", e)))?;
    
    // V2 addition: Store result in database
    let db_input = DatabaseQueryInput {
        query: "INSERT INTO results (data) VALUES (?)".to_string(),
        parameters: vec![serde_json::json!(api_response.call_id)],
        use_new_schema: true,
    };
    
    let db_result = ctx
        .execute_activity(
            "database_query",
            Some(serde_json::to_vec(&db_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let db_response: DatabaseQueryResult = serde_json::from_slice(&db_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse DB result: {}", e)))?;
    
    Ok(format!(
        "V2 Result: API={}, DB Rows={}",
        api_response.status_code, db_response.rows_affected
    ))
}

/// Workflow demonstrating side effects
pub async fn side_effect_workflow(
    ctx: &mut WorkflowContext,
    operation: String,
) -> Result<String, WorkflowError> {
    info!("Running side effect workflow: {}", operation);
    
    // Get current timestamp as a side effect
    // In real workflows, this would be wrapped in a side effect to ensure
    // the same value is used on replay
    let timestamp = chrono::Utc::now().timestamp();
    
    // Generate a UUID as a side effect
    let uuid = uuid::Uuid::new_v4().to_string();
    
    info!("Side effects - timestamp: {}, uuid: {}", timestamp, uuid);
    
    // Execute business logic using the side effect values
    let api_input = ApiCallInput {
        endpoint: format!("/api/operations/{}", operation),
        method: "POST".to_string(),
        payload: serde_json::json!({
            "timestamp": timestamp,
            "request_id": uuid,
            "operation": operation,
        }),
    };
    
    let result = ctx
        .execute_activity(
            "external_api_call",
            Some(serde_json::to_vec(&api_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let api_result: ApiCallResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    Ok(format!(
        "Operation {} completed with id {} at timestamp {}",
        operation, uuid, timestamp
    ))
}

/// Workflow with conditional version-based logic
pub async fn conditional_version_workflow(
    ctx: &mut WorkflowContext,
    (input, use_v2_api): (String, bool),
) -> Result<String, WorkflowError> {
    info!(
        "Running conditional version workflow (v2 api: {})",
        use_v2_api
    );
    
    // Get service configuration
    let config_result = ctx
        .execute_activity(
            "get_service_config",
            Some(serde_json::to_vec(&"payment_service".to_string()).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let config: ServiceConfig = serde_json::from_slice(&config_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse config: {}", e)))?;
    
    // Choose API version based on flag and config
    let api_version = if use_v2_api && config.api_version == "v2" {
        "v2"
    } else {
        warn!("Falling back to v1 API");
        "v1"
    };
    
    let api_input = ApiCallInput {
        endpoint: format!("{}/api/{}/process", config.endpoint, api_version),
        method: "POST".to_string(),
        payload: serde_json::json!({
            "data": input,
            "requested_version": api_version,
        }),
    };
    
    let result = ctx
        .execute_activity(
            "external_api_call",
            Some(serde_json::to_vec(&api_input).unwrap()),
            ActivityOptions {
                start_to_close_timeout: Duration::from_secs(config.timeout_seconds),
                ..Default::default()
            },
        )
        .await?;
    
    let api_result: ApiCallResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    // If using v2, also query the database
    if api_version == "v2" {
        let db_input = DatabaseQueryInput {
            query: "SELECT * FROM results WHERE api_version = 'v2'".to_string(),
            parameters: vec![],
            use_new_schema: true,
        };
        
        let db_result = ctx
            .execute_activity(
                "database_query",
                Some(serde_json::to_vec(&db_input).unwrap()),
                ActivityOptions::default(),
            )
            .await?;
        
        let db_response: DatabaseQueryResult = serde_json::from_slice(&db_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse DB result: {}", e)))?;
        
        Ok(format!(
            "V2 API call completed with {} DB rows retrieved",
            db_response.rows_affected
        ))
    } else {
        Ok(format!("V1 API call completed with status {}", api_result.status_code))
    }
}
