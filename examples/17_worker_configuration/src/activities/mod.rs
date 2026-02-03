//! Activity implementations for worker configuration example.
//!
//! These activities demonstrate various patterns that workers can execute.

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Simple computation activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeInput {
    pub values: Vec<f64>,
    pub operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeOutput {
    pub result: f64,
    pub operation: String,
}

pub async fn compute_activity(
    ctx: &ActivityContext,
    input: ComputeInput,
) -> Result<ComputeOutput, ActivityError> {
    let info = ctx.get_info();
    info!(
        "Compute activity started - ID: {}, Attempt: {}",
        info.activity_id, info.attempt
    );

    // Check remaining time
    if let Some(remaining) = ctx.get_remaining_time() {
        info!("Remaining time: {:?}", remaining);
    }

    let result = match input.operation.as_str() {
        "sum" => input.values.iter().sum(),
        "avg" => {
            if input.values.is_empty() {
                0.0
            } else {
                input.values.iter().sum::<f64>() / input.values.len() as f64
            }
        }
        "max" => input.values.iter().cloned().fold(f64::MIN, f64::max),
        "min" => input.values.iter().cloned().fold(f64::MAX, f64::min),
        _ => {
            return Err(ActivityError::ExecutionFailed(format!(
                "Unknown operation: {}",
                input.operation
            )))
        }
    };

    // Simulate some processing time
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Record heartbeat
    ctx.record_heartbeat(None);

    info!("Compute activity completed with result: {}", result);

    Ok(ComputeOutput {
        result,
        operation: input.operation,
    })
}

/// Data processing activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDataInput {
    pub records: Vec<serde_json::Value>,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDataOutput {
    pub processed_count: usize,
    pub failed_count: usize,
    pub batch_results: Vec<BatchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub batch_id: usize,
    pub success: bool,
    pub record_count: usize,
}

pub async fn process_data_activity(
    ctx: &ActivityContext,
    input: ProcessDataInput,
) -> Result<ProcessDataOutput, ActivityError> {
    info!(
        "Processing {} records in batches of {}",
        input.records.len(),
        input.batch_size
    );

    let mut processed_count = 0usize;
    let mut failed_count = 0usize;
    let mut batch_results = Vec::new();

    for (batch_idx, batch) in input.records.chunks(input.batch_size).enumerate() {
        // Check for cancellation
        if ctx.is_cancelled() {
            info!("Activity cancelled at batch {}", batch_idx);
            break;
        }

        // Process batch
        let record_count = batch.len();
        let success = true; // Simulate processing

        if success {
            processed_count += record_count;
        } else {
            failed_count += record_count;
        }

        batch_results.push(BatchResult {
            batch_id: batch_idx,
            success,
            record_count,
        });

        // Record heartbeat every 10 batches
        if batch_idx % 10 == 0 {
            ctx.record_heartbeat(Some(&serde_json::to_vec(&batch_results).unwrap()));
        }

        // Simulate batch processing time
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    info!(
        "Data processing complete: {} processed, {} failed",
        processed_count, failed_count
    );

    Ok(ProcessDataOutput {
        processed_count,
        failed_count,
        batch_results,
    })
}
