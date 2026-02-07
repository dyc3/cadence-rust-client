//! Workflow implementations for continue-as-new example.
//!
//! This example demonstrates the continue-as-new pattern for handling
//! long-running workflows and large datasets.

use crate::activities::*;
use tracing::info;
use uber_cadence_core::ActivityOptions;
use uber_cadence_workflow::context::WorkflowError;
use uber_cadence_workflow::WorkflowContext;

/// Maximum number of iterations before continuing as new
const MAX_ITERATIONS: u32 = 5;
/// Batch size for data processing
const BATCH_SIZE: usize = 10;

/// A workflow that demonstrates continue-as-new pattern
///
/// This workflow processes data in batches and periodically continues as new
/// to prevent the workflow history from growing too large.
pub async fn batched_processor_workflow(
    ctx: &mut WorkflowContext,
    checkpoint: Option<CheckpointData>,
) -> Result<BatchSummary, WorkflowError> {
    // Extract state from checkpoint or initialize
    let (mut iteration, mut total_processed, mut cursor, metadata) = match checkpoint {
        Some(cp) => {
            info!(
                "Resuming from checkpoint: iteration={}, total={}",
                cp.iteration, cp.total_processed
            );
            (
                cp.iteration,
                cp.total_processed,
                cp.last_cursor,
                cp.metadata,
            )
        }
        None => {
            info!("Starting new batched processor workflow");
            (
                0u32,
                0u64,
                None,
                serde_json::json!({ "start_time": chrono::Utc::now() }),
            )
        }
    };

    let mut batch_results = Vec::new();
    let mut _should_continue = true;

    while _should_continue && iteration < MAX_ITERATIONS {
        iteration += 1;
        info!("Starting iteration {}", iteration);

        // Fetch data batch
        let fetch_input = DataFetchInput {
            cursor: cursor.clone(),
            batch_size: BATCH_SIZE,
        };

        let fetch_result = ctx
            .execute_activity(
                "fetch_data",
                Some(serde_json::to_vec(&fetch_input).unwrap()),
                ActivityOptions::default(),
            )
            .await?;

        let data_result: DataFetchResult = serde_json::from_slice(&fetch_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse fetch result: {}", e)))?;

        // If no data, we're done
        if data_result.items.is_empty() {
            info!("No more data to process");
            _should_continue = false;
            break;
        }

        // Process the batch
        let batch_input = BatchProcessInput {
            batch_id: format!("batch_{}_{}", iteration, chrono::Utc::now().timestamp()),
            items: data_result.items,
            iteration,
        };

        let process_result = ctx
            .execute_activity(
                "process_batch",
                Some(serde_json::to_vec(&batch_input).unwrap()),
                ActivityOptions::default(),
            )
            .await?;

        let batch_result: BatchProcessResult = serde_json::from_slice(&process_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse batch result: {}", e)))?;

        batch_results.push(batch_result.clone());
        total_processed += batch_result.processed_count as u64;
        cursor = data_result.next_cursor;

        info!(
            "Iteration {} completed: {} processed, total: {}",
            iteration, batch_result.processed_count, total_processed
        );

        // Check if we should continue as new or there's no more data
        if !data_result.has_more {
            info!("All data processed");
            _should_continue = false;
        }
    }

    // Save checkpoint before potentially continuing as new
    let checkpoint_data = CheckpointData {
        iteration,
        total_processed,
        last_cursor: cursor.clone(),
        metadata: metadata.clone(),
    };

    let _ = ctx
        .execute_activity(
            "save_checkpoint",
            Some(serde_json::to_vec(&checkpoint_data).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    // If we've reached max iterations but there's still more data, continue as new
    if iteration >= MAX_ITERATIONS && cursor.is_some() {
        info!("Continuing as new at iteration {}", iteration);

        // In real Cadence, this would call continue_as_new
        // For this example, we return a special result indicating continuation
        return Ok(BatchSummary {
            total_processed,
            iterations_completed: iteration,
            continued_as_new: true,
            final_cursor: cursor,
            batches: batch_results,
        });
    }

    // Otherwise, return final summary
    Ok(BatchSummary {
        total_processed,
        iterations_completed: iteration,
        continued_as_new: false,
        final_cursor: None,
        batches: batch_results,
    })
}

/// Summary of batch processing
#[derive(Debug, Clone)]
pub struct BatchSummary {
    pub total_processed: u64,
    pub iterations_completed: u32,
    pub continued_as_new: bool,
    pub final_cursor: Option<String>,
    pub batches: Vec<BatchProcessResult>,
}

/// A workflow that processes paginated data with size limits
pub async fn paginated_processor_workflow(
    ctx: &mut WorkflowContext,
    initial_cursor: Option<String>,
) -> Result<PaginationSummary, WorkflowError> {
    info!("Starting paginated processor workflow");

    let mut cursor = initial_cursor;
    let mut all_items = Vec::new();
    let mut pages_processed = 0u32;
    let max_pages = 3; // Limit pages per workflow run

    while pages_processed < max_pages {
        pages_processed += 1;

        let fetch_input = DataFetchInput {
            cursor: cursor.clone(),
            batch_size: 5, // Smaller batch for pagination demo
        };

        let fetch_result = ctx
            .execute_activity(
                "fetch_data",
                Some(serde_json::to_vec(&fetch_input).unwrap()),
                ActivityOptions::default(),
            )
            .await?;

        let data_result: DataFetchResult = serde_json::from_slice(&fetch_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse fetch result: {}", e)))?;

        if data_result.items.is_empty() {
            break;
        }

        all_items.extend(data_result.items.clone());
        cursor = data_result.next_cursor;

        info!(
            "Page {}: fetched {} items, total: {}",
            pages_processed,
            data_result.items.len(),
            all_items.len()
        );

        if !data_result.has_more {
            info!("No more pages");
            break;
        }
    }

    // Process all collected items
    if !all_items.is_empty() {
        let batch_input = BatchProcessInput {
            batch_id: format!("page_batch_{}", chrono::Utc::now().timestamp()),
            items: all_items.clone(),
            iteration: pages_processed,
        };

        let process_result = ctx
            .execute_activity(
                "process_batch",
                Some(serde_json::to_vec(&batch_input).unwrap()),
                ActivityOptions::default(),
            )
            .await?;

        let batch_result: BatchProcessResult = serde_json::from_slice(&process_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse batch result: {}", e)))?;

        Ok(PaginationSummary {
            total_items: all_items.len() as u64,
            pages_processed,
            processed_count: batch_result.processed_count as u64,
            has_more: cursor.is_some(),
            next_cursor: cursor,
        })
    } else {
        Ok(PaginationSummary {
            total_items: 0,
            pages_processed,
            processed_count: 0,
            has_more: false,
            next_cursor: None,
        })
    }
}

/// Summary of pagination processing
#[derive(Debug, Clone)]
pub struct PaginationSummary {
    pub total_items: u64,
    pub pages_processed: u32,
    pub processed_count: u64,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}
