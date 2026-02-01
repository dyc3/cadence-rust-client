//! Workflow implementations for child workflows example.
//!
//! This example demonstrates parent-child workflow relationships
//! and parallel execution patterns.

use crate::activities::*;
use cadence_core::{ActivityOptions, ChildWorkflowOptions};
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, error};

/// Input for the parent workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentWorkflowInput {
    pub job_id: String,
    pub data_chunks: Vec<Vec<String>>,
    pub parallel_limit: usize,
}

/// Result from the parent workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentWorkflowResult {
    pub job_id: String,
    pub total_processed: usize,
    pub failed_chunks: Vec<usize>,
    pub child_results: Vec<ChildResult>,
}

/// Result from a child workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildResult {
    pub child_id: String,
    pub chunk_id: usize,
    pub processed_count: usize,
    pub success: bool,
}

/// Parent workflow that orchestrates child workflows
///
/// This workflow demonstrates:
/// - Starting child workflows
/// - Running children in parallel
/// - Collecting results from children
/// - Handling child failures
pub async fn parent_workflow(
    ctx: &mut WorkflowContext,
    input: ParentWorkflowInput,
) -> Result<ParentWorkflowResult, WorkflowError> {
    info!(
        "Parent workflow starting job {} with {} chunks",
        input.job_id,
        input.data_chunks.len()
    );

    let mut failed_chunks = vec![];
    let mut child_results = vec![];

    // Start child workflows with a concurrency limit
    for (idx, chunk) in input.data_chunks.iter().enumerate() {
        if idx >= input.parallel_limit {
            // Wait for some children to complete before starting more
            // (In real implementation, would use proper concurrency control)
            ctx.sleep(Duration::from_millis(10)).await;
        }

        let child_input = ChildWorkflowInput {
            child_id: format!("child_{}", idx),
            chunk_id: idx,
            data: chunk.clone(),
        };

        let child_options = ChildWorkflowOptions {
            workflow_id: format!("{}_child_{}", input.job_id, idx),
            execution_start_to_close_timeout: Duration::from_secs(60),
            task_start_to_close_timeout: Duration::from_secs(10),
            ..Default::default()
        };

        info!("Starting child workflow for chunk {}", idx);

        // Start child workflow
        let child_result = ctx
            .execute_child_workflow(
                "child_processor",
                Some(serde_json::to_vec(&child_input).unwrap()),
                child_options,
            )
            .await;

        match child_result {
            Ok(result_bytes) => {
                let result: ChildWorkflowResult = serde_json::from_slice(&result_bytes)
                    .map_err(|e| WorkflowError::Generic(format!("Failed to parse child result: {}", e)))?;

                child_results.push(ChildResult {
                    child_id: result.child_id,
                    chunk_id: result.chunk_id,
                    processed_count: result.processed_count,
                    success: result.success,
                });
            }
            Err(e) => {
                error!("Child workflow {} failed: {}", idx, e);
                failed_chunks.push(idx);
            }
        }
    }

    let total_processed: usize = child_results.iter().map(|r| r.processed_count).sum();

    info!(
        "Parent workflow completed: {} items processed, {} chunks failed",
        total_processed,
        failed_chunks.len()
    );

    Ok(ParentWorkflowResult {
        job_id: input.job_id,
        total_processed,
        failed_chunks,
        child_results,
    })
}

/// Input for child workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildWorkflowInput {
    pub child_id: String,
    pub chunk_id: usize,
    pub data: Vec<String>,
}

/// Result from child workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildWorkflowResult {
    pub child_id: String,
    pub chunk_id: usize,
    pub processed_count: usize,
    pub success: bool,
}

/// Child workflow that processes a chunk of data
///
/// This workflow demonstrates:
/// - Executing activities
/// - Returning results to parent
/// - Handling cancellation from parent
pub async fn child_processor_workflow(
    ctx: &mut WorkflowContext,
    input: ChildWorkflowInput,
) -> Result<ChildWorkflowResult, WorkflowError> {
    info!(
        "Child workflow {} processing chunk {} with {} items",
        input.child_id,
        input.chunk_id,
        input.data.len()
    );

    // Check for cancellation before starting
    if ctx.is_cancelled() {
        return Err(WorkflowError::Cancelled);
    }

    // Process the chunk using an activity
    let activity_input = ProcessChunkInput {
        chunk_id: input.chunk_id,
        data: input.data.clone(),
        processing_options: ProcessingOptions {
            skip_validation: false,
            enable_logging: true,
        },
    };

    let result = ctx
        .execute_activity(
            "process_chunk",
            Some(serde_json::to_vec(&activity_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let chunk_result: ProcessChunkResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse chunk result: {}", e)))?;

    let success = chunk_result.failed_count == 0;

    info!(
        "Child workflow {} completed: {} processed, success: {}",
        input.child_id,
        chunk_result.processed_count,
        success
    );

    Ok(ChildWorkflowResult {
        child_id: input.child_id,
        chunk_id: input.chunk_id,
        processed_count: chunk_result.processed_count,
        success,
    })
}

/// Input for fan-out workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanOutInput {
    pub parent_id: String,
    pub items: Vec<String>,
    pub child_workflow_type: String,
}

/// Result from fan-out workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanOutResult {
    pub parent_id: String,
    pub completed_children: usize,
    pub failed_children: usize,
    pub all_successful: bool,
}

/// Workflow that fans out to multiple child workflows
///
/// This demonstrates the fan-out pattern where a parent starts
/// many child workflows in parallel.
pub async fn fan_out_workflow(
    ctx: &mut WorkflowContext,
    input: FanOutInput,
) -> Result<FanOutResult, WorkflowError> {
    info!(
        "Fan-out workflow {} starting {} children",
        input.parent_id,
        input.items.len()
    );

    let mut child_handles = vec![];
    let mut completed = 0;
    let mut failed = 0;

    // Start all children in parallel
    for (idx, item) in input.items.iter().enumerate() {
        let child_input = SingleItemInput {
            item_id: format!("item_{}", idx),
            item: item.clone(),
        };

        let child_options = ChildWorkflowOptions {
            workflow_id: format!("{}_child_{}", input.parent_id, idx),
            execution_start_to_close_timeout: Duration::from_secs(30),
            task_start_to_close_timeout: Duration::from_secs(5),
            ..Default::default()
        };

        // Start child workflow
        let handle = ctx.execute_child_workflow(
            &input.child_workflow_type,
            Some(serde_json::to_vec(&child_input).unwrap()),
            child_options,
        );

        child_handles.push(handle);
    }

    // Wait for all children to complete
    for handle in child_handles {
        match handle.await {
            Ok(_) => {
                completed += 1;
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let all_successful = failed == 0;

    info!(
        "Fan-out workflow {} completed: {} succeeded, {} failed",
        input.parent_id,
        completed,
        failed
    );

    Ok(FanOutResult {
        parent_id: input.parent_id,
        completed_children: completed,
        failed_children: failed,
        all_successful,
    })
}

/// Input for single item child workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleItemInput {
    pub item_id: String,
    pub item: String,
}

/// Result from single item child workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleItemResult {
    pub item_id: String,
    pub processed: bool,
    pub result: String,
}

/// Child workflow for processing a single item
pub async fn process_single_item_workflow(
    ctx: &mut WorkflowContext,
    input: SingleItemInput,
) -> Result<SingleItemResult, WorkflowError> {
    info!("Processing single item: {}", input.item_id);

    // Validate the item
    let validation_input = ValidateInput {
        data_id: input.item_id.clone(),
        data: serde_json::json!(input.item),
        validation_rules: vec!["required".to_string()],
    };

    let validation_result = ctx
        .execute_activity(
            "validate_data",
            Some(serde_json::to_vec(&validation_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let validation: ValidationResult = serde_json::from_slice(&validation_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation: {}", e)))?;

    let processed = validation.is_valid;
    let result = if processed {
        format!("processed_{}", input.item)
    } else {
        format!("failed_{}", input.item)
    };

    Ok(SingleItemResult {
        item_id: input.item_id,
        processed,
        result,
    })
}

/// Input for fan-in workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanInInput {
    pub aggregation_id: String,
    pub child_outputs: Vec<Vec<String>>,
}

/// Result from fan-in workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanInResult {
    pub aggregation_id: String,
    pub total_items: usize,
    pub aggregated_data: Vec<String>,
}

/// Workflow that aggregates results from children (fan-in pattern)
///
/// This demonstrates collecting and aggregating results from multiple
/// child workflows.
pub async fn fan_in_workflow(
    ctx: &mut WorkflowContext,
    input: FanInInput,
) -> Result<FanInResult, WorkflowError> {
    info!(
        "Fan-in workflow {} aggregating {} child outputs",
        input.aggregation_id,
        input.child_outputs.len()
    );

    let mut partial_results = vec![];

    // Convert child outputs to partial results
    for (idx, output) in input.child_outputs.iter().enumerate() {
        partial_results.push(PartialResult {
            child_id: format!("child_{}", idx),
            data: output.clone(),
            count: output.len(),
        });
    }

    // Aggregate using an activity
    let aggregate_input = AggregateInput {
        results: partial_results,
    };

    let aggregate_result = ctx
        .execute_activity(
            "aggregate_results",
            Some(serde_json::to_vec(&aggregate_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let aggregation: AggregationResult = serde_json::from_slice(&aggregate_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse aggregation: {}", e)))?;

    info!(
        "Fan-in workflow {} completed: {} items from {} children",
        input.aggregation_id,
        aggregation.total_count,
        aggregation.child_count
    );

    Ok(FanInResult {
        aggregation_id: input.aggregation_id,
        total_items: aggregation.total_count,
        aggregated_data: aggregation.all_data,
    })
}
