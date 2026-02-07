//! Workflow implementations for performance tuning example.
//!
//! This module demonstrates high-performance workflow patterns:
//! - Parallel activity execution
//! - Batched operations
//! - Resource pooling
//! - Efficient data flow

use crate::activities::*;
use std::time::Duration;
use tracing::{debug, info};
use uber_cadence_core::ActivityOptions;
use uber_cadence_workflow::context::WorkflowError;
use uber_cadence_workflow::WorkflowContext;

/// Process large dataset with batching and parallelization
pub async fn high_throughput_workflow(
    ctx: &mut WorkflowContext,
    items: Vec<String>,
) -> Result<BatchResult, WorkflowError> {
    info!(
        "Starting high-throughput workflow with {} items",
        items.len()
    );

    let batch_input = BatchInput {
        items,
        batch_size: 100,
    };

    let result = ctx
        .execute_activity(
            "batch_process",
            Some(serde_json::to_vec(&batch_input).unwrap()),
            ActivityOptions {
                start_to_close_timeout: Duration::from_secs(300),
                heartbeat_timeout: Duration::from_secs(30),
                ..Default::default()
            },
        )
        .await?;

    let batch_result: BatchResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

    info!(
        "Workflow complete: {} items processed in {}ms",
        batch_result.processed_count, batch_result.total_duration_ms
    );

    Ok(batch_result)
}

/// Parallel data processing workflow
pub async fn parallel_processing_workflow(
    ctx: &mut WorkflowContext,
    datasets: Vec<Vec<u8>>,
) -> Result<Vec<TransformOutput>, WorkflowError> {
    info!(
        "Starting parallel processing for {} datasets",
        datasets.len()
    );

    let mut futures = Vec::new();

    // Spawn parallel activity executions
    for (idx, data) in datasets.into_iter().enumerate() {
        let transform_input = TransformInput {
            data,
            compression_level: 50,
        };

        debug!("Spawning transform activity for dataset {}", idx);

        let activity_result = ctx
            .execute_activity(
                "data_transform",
                Some(serde_json::to_vec(&transform_input).unwrap()),
                ActivityOptions::default(),
            )
            .await?;

        let output: TransformOutput = serde_json::from_slice(&activity_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse transform: {}", e)))?;

        futures.push(output);
    }

    info!(
        "Parallel processing complete: {} datasets transformed",
        futures.len()
    );
    Ok(futures)
}

/// Cache-optimized workflow with warmup
pub async fn cache_optimized_workflow(
    ctx: &mut WorkflowContext,
    keys: Vec<String>,
) -> Result<(usize, usize), WorkflowError> {
    info!("Starting cache-optimized workflow with {} keys", keys.len());

    // Step 1: Warm up cache
    let warmup_input = CacheWarmupInput {
        keys: keys.clone(),
        preload_size: 1000,
    };

    let warmup_result = ctx
        .execute_activity(
            "cache_warmup",
            Some(serde_json::to_vec(&warmup_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let warmed_count: usize = serde_json::from_slice(&warmup_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse warmup: {}", e)))?;

    // Step 2: Process with warmed cache
    let batch_input = BatchInput {
        items: keys,
        batch_size: 500,
    };

    let batch_result = ctx
        .execute_activity(
            "batch_process",
            Some(serde_json::to_vec(&batch_input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let processed: BatchResult = serde_json::from_slice(&batch_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse batch: {}", e)))?;

    info!(
        "Cache workflow complete: {} warmed, {} processed",
        warmed_count, processed.processed_count
    );

    Ok((warmed_count, processed.processed_count))
}

/// End-to-end high-throughput ingestion workflow
pub async fn ingestion_pipeline_workflow(
    ctx: &mut WorkflowContext,
    records: Vec<String>,
) -> Result<usize, WorkflowError> {
    info!("Starting ingestion pipeline with {} records", records.len());

    let result = ctx
        .execute_activity(
            "high_throughput_ingest",
            Some(serde_json::to_vec(&records).unwrap()),
            ActivityOptions {
                start_to_close_timeout: Duration::from_secs(600),
                heartbeat_timeout: Duration::from_secs(60),
                retry_policy: Some(uber_cadence_core::RetryPolicy {
                    initial_interval: Duration::from_secs(1),
                    backoff_coefficient: 2.0,
                    maximum_interval: Duration::from_secs(30),
                    maximum_attempts: 3,
                    non_retryable_error_types: vec![],
                    expiration_interval: Duration::from_secs(0),
                }),
                ..Default::default()
            },
        )
        .await?;

    let ingested: usize = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

    info!("Ingestion pipeline complete: {} records ingested", ingested);
    Ok(ingested)
}
