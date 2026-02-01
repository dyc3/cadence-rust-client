//! Activity implementations for child workflows example.
//!
//! Activities support the child workflow patterns by handling
//! individual task processing.

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Input for processing a chunk of data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessChunkInput {
    pub chunk_id: usize,
    pub data: Vec<String>,
    pub processing_options: ProcessingOptions,
}

/// Processing options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOptions {
    pub skip_validation: bool,
    pub enable_logging: bool,
}

/// Result of chunk processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessChunkResult {
    pub chunk_id: usize,
    pub processed_count: usize,
    pub failed_count: usize,
    pub results: Vec<String>,
}

/// Process a single chunk of data
pub async fn process_chunk_activity(
    _ctx: &ActivityContext,
    input: ProcessChunkInput,
) -> Result<ProcessChunkResult, ActivityError> {
    info!(
        "Processing chunk {} with {} items",
        input.chunk_id,
        input.data.len()
    );

    let mut results = vec![];
    let mut failed_count = 0;

    for item in &input.data {
        // Simulate processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        if item.is_empty() {
            failed_count += 1;
        } else {
            let processed = format!("processed_{}", item);
            results.push(processed);
        }
    }

    info!(
        "Chunk {} completed: {} processed, {} failed",
        input.chunk_id,
        results.len(),
        failed_count
    );

    Ok(ProcessChunkResult {
        chunk_id: input.chunk_id,
        processed_count: results.len(),
        failed_count,
        results,
    })
}

/// Input for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateInput {
    pub data_id: String,
    pub data: serde_json::Value,
    pub validation_rules: Vec<String>,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub data_id: String,
    pub is_valid: bool,
    pub errors: Vec<String>,
}

/// Validate data
pub async fn validate_data_activity(
    _ctx: &ActivityContext,
    input: ValidateInput,
) -> Result<ValidationResult, ActivityError> {
    info!("Validating data {}", input.data_id);

    let mut errors = vec![];

    // Simulate validation
    for rule in &input.validation_rules {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

        if rule == "required" && input.data.is_null() {
            errors.push("Data is required".to_string());
        }
    }

    let is_valid = errors.is_empty();

    info!("Validation of {}: {}", input.data_id, if is_valid { "passed" } else { "failed" });

    Ok(ValidationResult {
        data_id: input.data_id,
        is_valid,
        errors,
    })
}

/// Input for aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateInput {
    pub results: Vec<PartialResult>,
}

/// Partial result from child
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialResult {
    pub child_id: String,
    pub data: Vec<String>,
    pub count: usize,
}

/// Final aggregation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    pub total_count: usize,
    pub all_data: Vec<String>,
    pub child_count: usize,
}

/// Aggregate results from multiple children
pub async fn aggregate_results_activity(
    _ctx: &ActivityContext,
    input: AggregateInput,
) -> Result<AggregationResult, ActivityError> {
    info!("Aggregating results from {} children", input.results.len());

    let mut all_data = vec![];
    let mut total_count = 0;

    for partial in &input.results {
        all_data.extend(partial.data.clone());
        total_count += partial.count;
    }

    info!("Aggregation complete: {} items from {} children", total_count, input.results.len());

    Ok(AggregationResult {
        total_count,
        all_data,
        child_count: input.results.len(),
    })
}
