//! Workflow implementations for worker configuration example.
//!
//! These workflows demonstrate different patterns that can be registered
//! with workers.

use crate::activities::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;
use uber_cadence_core::ActivityOptions;
use uber_cadence_workflow::context::WorkflowError;
use uber_cadence_workflow::WorkflowContext;

/// Data processing workflow input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProcessingWorkflowInput {
    pub dataset_id: String,
    pub compute_operations: Vec<String>,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProcessingWorkflowOutput {
    pub dataset_id: String,
    pub compute_results: Vec<f64>,
    pub processing_summary: ProcessDataOutput,
}

/// A workflow that orchestrates data processing activities
pub async fn data_processing_workflow(
    ctx: &mut WorkflowContext,
    input: DataProcessingWorkflowInput,
) -> Result<DataProcessingWorkflowOutput, WorkflowError> {
    info!(
        "Starting data_processing_workflow for dataset: {}",
        input.dataset_id
    );

    let mut compute_results = Vec::new();

    // Execute compute activities for each operation
    for operation in &input.compute_operations {
        let compute_input = ComputeInput {
            values: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            operation: operation.clone(),
        };

        let activity_options = ActivityOptions {
            task_list: ctx.workflow_info().task_list.clone(),
            start_to_close_timeout: Duration::from_secs(60),
            retry_policy: Some(uber_cadence_core::RetryPolicy::default()),
            ..Default::default()
        };

        let result = ctx
            .execute_activity(
                "compute",
                Some(serde_json::to_vec(&compute_input).unwrap()),
                activity_options,
            )
            .await?;

        let compute_output: ComputeOutput = serde_json::from_slice(&result).map_err(|e| {
            WorkflowError::Generic(format!("Failed to parse compute result: {}", e))
        })?;

        compute_results.push(compute_output.result);
        info!(
            "Compute operation '{}' result: {}",
            operation, compute_output.result
        );
    }

    // Execute data processing activity
    let process_input = ProcessDataInput {
        records: vec![], // Empty for example
        batch_size: input.batch_size,
    };

    let process_result = ctx
        .execute_activity(
            "process_data",
            Some(serde_json::to_vec(&process_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let processing_summary: ProcessDataOutput = serde_json::from_slice(&process_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse processing result: {}", e)))?;

    info!(
        "Data processing workflow completed for dataset: {}",
        input.dataset_id
    );

    Ok(DataProcessingWorkflowOutput {
        dataset_id: input.dataset_id,
        compute_results,
        processing_summary,
    })
}

/// A simple computation workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleComputeInput {
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleComputeOutput {
    pub sum: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
}

pub async fn simple_compute_workflow(
    ctx: &mut WorkflowContext,
    input: SimpleComputeInput,
) -> Result<SimpleComputeOutput, WorkflowError> {
    info!(
        "Starting simple_compute_workflow with {} values",
        input.values.len()
    );

    let operations = vec!["sum", "avg", "min", "max"];
    let mut results = Vec::new();

    for operation in &operations {
        let compute_input = ComputeInput {
            values: input.values.clone(),
            operation: operation.to_string(),
        };

        let result = ctx
            .execute_activity(
                "compute",
                Some(serde_json::to_vec(&compute_input).unwrap()),
                ActivityOptions::default(),
            )
            .await?;

        let output: ComputeOutput = serde_json::from_slice(&result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

        results.push(output.result);
    }

    Ok(SimpleComputeOutput {
        sum: results[0],
        average: results[1],
        min: results[2],
        max: results[3],
    })
}
