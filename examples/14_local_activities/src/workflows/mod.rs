//! Workflow implementations for local activities example.
//!
//! These workflows demonstrate local activity execution patterns.

use crate::activities::*;
use cadence_workflow::context::WorkflowError;
use cadence_workflow::{LocalActivityOptions, WorkflowContext};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};

/// Input for data processing pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInput {
    pub raw_data: serde_json::Value,
    pub validation_schema: String,
    pub enrichment_fields: Vec<String>,
    pub transform_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutput {
    pub original_data: serde_json::Value,
    pub validated: bool,
    pub enriched_record: serde_json::Value,
    pub transformed_record: serde_json::Value,
    pub processing_decision: String,
}

/// A workflow that demonstrates local activity patterns
///
/// Local activities are executed synchronously in the workflow thread,
/// making them ideal for short, fast operations that don't need the
/// full activity infrastructure (task queues, heartbeats, etc.).
pub async fn local_activity_pipeline_workflow(
    ctx: &mut WorkflowContext,
    input: PipelineInput,
) -> Result<PipelineOutput, WorkflowError> {
    info!("Starting local activity pipeline workflow");

    // Step 1: Validate data using local activity (fast operation)
    let validate_input = ValidateInput {
        data: input.raw_data.clone(),
        schema: input.validation_schema.clone(),
    };

    let local_activity_options = LocalActivityOptions {
        schedule_to_close_timeout: Duration::from_secs(5),
        retry_policy: Some(cadence_core::RetryPolicy {
            initial_interval: Duration::from_millis(100),
            backoff_coefficient: 1.5,
            maximum_interval: Duration::from_secs(1),
            maximum_attempts: 3,
            non_retryable_error_types: vec![],
            expiration_interval: Duration::from_secs(10),
        }),
    };

    let validation_result = ctx
        .execute_local_activity(
            "validate_data",
            Some(serde_json::to_vec(&validate_input).unwrap()),
            local_activity_options.clone(),
        )
        .await?;

    let validation: ValidateOutput = serde_json::from_slice(&validation_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation: {}", e)))?;

    if !validation.valid {
        error!("Data validation failed: {:?}", validation.errors);
        return Err(WorkflowError::Generic(format!(
            "Validation failed: {:?}",
            validation.errors
        )));
    }

    info!("Data validation passed");

    // Step 2: Enrich data using local activity
    let enrich_input = EnrichInput {
        record: input.raw_data.clone(),
        enrichment_fields: input.enrichment_fields.clone(),
    };

    let enrichment_result = ctx
        .execute_local_activity(
            "enrich_data",
            Some(serde_json::to_vec(&enrich_input).unwrap()),
            local_activity_options.clone(),
        )
        .await?;

    let enrichment: EnrichOutput = serde_json::from_slice(&enrichment_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse enrichment: {}", e)))?;

    info!(
        "Data enriched with {} fields",
        enrichment.fields_added.len()
    );

    // Step 3: Make processing decision using local activity
    let decision_input = DecisionInput {
        context: enrichment.enriched_record.clone(),
        decision_type: "classify".to_string(),
    };

    let decision_result = ctx
        .execute_local_activity(
            "make_decision",
            Some(serde_json::to_vec(&decision_input).unwrap()),
            local_activity_options.clone(),
        )
        .await?;

    let decision: DecisionOutput = serde_json::from_slice(&decision_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse decision: {}", e)))?;

    info!(
        "Processing decision: {} (confidence: {})",
        decision.decision, decision.confidence
    );

    // Step 4: Transform data using local activity
    let transform_input = TransformInput {
        data: enrichment.enriched_record.clone(),
        transform_type: input.transform_type.clone(),
    };

    let transform_result = ctx
        .execute_local_activity(
            "transform_data",
            Some(serde_json::to_vec(&transform_input).unwrap()),
            local_activity_options,
        )
        .await?;

    let transform: TransformOutput = serde_json::from_slice(&transform_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse transform: {}", e)))?;

    info!(
        "Data transformation complete: {}",
        transform.transform_applied
    );

    info!("Local activity pipeline workflow completed successfully");

    Ok(PipelineOutput {
        original_data: input.raw_data,
        validated: validation.valid,
        enriched_record: enrichment.enriched_record,
        transformed_record: transform.transformed_data,
        processing_decision: decision.decision,
    })
}

/// Input for mixed workflow (local + regular activities)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedWorkflowInput {
    pub data: serde_json::Value,
    pub use_local_validation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedWorkflowOutput {
    pub local_validation_result: Option<bool>,
    pub regular_activity_result: Option<String>,
}

/// A workflow that demonstrates mixing local and regular activities
///
/// Use local activities for fast operations that complete quickly,
/// and regular activities for operations that may take longer or
/// need the full activity infrastructure.
pub async fn mixed_local_and_regular_workflow(
    ctx: &mut WorkflowContext,
    input: MixedWorkflowInput,
) -> Result<MixedWorkflowOutput, WorkflowError> {
    info!("Starting mixed local/regular workflow");

    let mut output = MixedWorkflowOutput {
        local_validation_result: None,
        regular_activity_result: None,
    };

    // Use local activity for fast validation
    if input.use_local_validation {
        let validate_input = ValidateInput {
            data: input.data.clone(),
            schema: "quick-check".to_string(),
        };

        let local_options = LocalActivityOptions {
            schedule_to_close_timeout: Duration::from_secs(5),
            retry_policy: None,
        };

        let result = ctx
            .execute_local_activity(
                "validate_data",
                Some(serde_json::to_vec(&validate_input).unwrap()),
                local_options,
            )
            .await?;

        let validation: ValidateOutput = serde_json::from_slice(&result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse: {}", e)))?;

        output.local_validation_result = Some(validation.valid);
        info!("Local validation result: {}", validation.valid);
    }

    // For longer operations, you might use regular activities
    // This is just a placeholder to show the pattern
    info!("Mixed workflow completed");

    Ok(output)
}

// Guidelines for choosing between Local Activities and Regular Activities
//
// Local Activities:
// - Short execution time (< 5 seconds)
// - No need for heartbeats
// - Low failure rate
// - Don't need to be distributed across worker pool
// - Fast data validation, transformation, enrichment
// - Quick business logic decisions
//
// Regular Activities:
// - Long-running operations
// - Need heartbeats for progress
// - May fail and need retries
// - Should be distributed across worker pool
// - External API calls
// - Database operations
// - File processing
// - Network I/O
pub fn local_vs_regular_activities() {}
