//! Workflow implementations for query operations example.

use crate::activities::{
    BatchProcessingInput, BatchProcessingResult, DataProcessingTask, ProcessingResult,
};
use cadence_core::ActivityOptions;
use cadence_workflow::context::WorkflowError;
use cadence_workflow::WorkflowContext;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Current state of the workflow (queryable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub workflow_id: String,
    pub current_stage: String,
    pub total_records: usize,
    pub processed_records: usize,
    pub start_time: i64,
    pub status: String,
}

/// Query response for current progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressQueryResult {
    pub percentage: f64,
    pub processed: usize,
    pub total: usize,
    pub current_stage: String,
    pub estimated_completion: String,
}

/// Workflow that supports progress queries
pub async fn queryable_data_processing_workflow(
    ctx: &mut WorkflowContext,
    task: DataProcessingTask,
) -> Result<ProcessingResult, WorkflowError> {
    info!(
        "Starting queryable data processing workflow for task {}",
        task.task_id
    );

    let workflow_id = ctx.workflow_info().workflow_execution.workflow_id.clone();
    let start_time = chrono::Utc::now().timestamp();

    // Initialize workflow state for queries
    let mut state = WorkflowState {
        workflow_id: workflow_id.clone(),
        current_stage: "initialization".to_string(),
        total_records: task.record_count,
        processed_records: 0,
        start_time,
        status: "running".to_string(),
    };

    // Stage 1: Data validation
    state.current_stage = "validation".to_string();
    let validation = ctx
        .execute_activity(
            "validate_data",
            Some(serde_json::to_vec(&task.data_source).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let is_valid: bool = serde_json::from_slice(&validation)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation: {}", e)))?;

    if !is_valid {
        state.status = "failed_validation".to_string();
        return Err(WorkflowError::Generic("Data validation failed".to_string()));
    }

    // Stage 2: Process data
    state.current_stage = "processing".to_string();
    let result = ctx
        .execute_activity(
            "process_data",
            Some(serde_json::to_vec(&task).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let processing_result: ProcessingResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

    state.processed_records = processing_result.records_processed;
    state.current_stage = "completed".to_string();
    state.status = "completed".to_string();

    info!(
        "Data processing workflow completed: {} records processed",
        processing_result.records_processed
    );

    Ok(processing_result)
}

/// Query response for workflow details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDetailsQueryResult {
    pub workflow_id: String,
    pub active_batches: usize,
    pub completed_batches: usize,
    pub failed_items: Vec<String>,
    pub status: String,
}

/// Workflow with multiple batch processing stages
pub async fn batch_processing_with_queries_workflow(
    ctx: &mut WorkflowContext,
    batches: Vec<BatchProcessingInput>,
) -> Result<Vec<BatchProcessingResult>, WorkflowError> {
    let _workflow_id = ctx.workflow_info().workflow_execution.workflow_id.clone();

    info!(
        "Starting batch processing workflow with {} batches",
        batches.len()
    );

    let mut completed_batches = 0usize;
    let mut all_failed_items: Vec<String> = Vec::new();
    let mut results = Vec::new();

    // Process each batch
    for (idx, batch) in batches.iter().enumerate() {
        info!("Processing batch {}/{}", idx + 1, batches.len());

        let result = ctx
            .execute_activity(
                "process_batch",
                Some(serde_json::to_vec(batch).unwrap()),
                ActivityOptions::default(),
            )
            .await?;

        let batch_result: BatchProcessingResult = serde_json::from_slice(&result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse batch result: {}", e)))?;

        all_failed_items.extend(batch_result.failed_items.clone());
        completed_batches += 1;
        results.push(batch_result);

        // Small delay between batches
        ctx.sleep(Duration::from_millis(10)).await;
    }

    info!(
        "Batch processing workflow completed: {} batches, {} failed items",
        completed_batches,
        all_failed_items.len()
    );

    Ok(results)
}

/// Simple query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusQueryResult {
    pub status: String,
    pub timestamp: i64,
}

/// Workflow demonstrating simple status queries
pub async fn simple_status_query_workflow(
    ctx: &mut WorkflowContext,
    task: DataProcessingTask,
) -> Result<ProcessingResult, WorkflowError> {
    info!("Starting simple status query workflow");

    let mut _current_status = "starting".to_string();

    // Update status and process
    _current_status = "validating".to_string();

    ctx.sleep(Duration::from_millis(100)).await;

    _current_status = "processing".to_string();

    let result = ctx
        .execute_activity(
            "process_data",
            Some(serde_json::to_vec(&task).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let processing_result: ProcessingResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

    _current_status = "completed".to_string();

    info!("Simple status query workflow completed");

    Ok(processing_result)
}
