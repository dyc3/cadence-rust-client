//! Activity implementations for query operations example.
//!
//! This example demonstrates activities that support queryable workflows:
//! - Long-running data processing
//! - Multi-stage operations
//! - Progress-reporting activities

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Data processing task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProcessingTask {
    pub task_id: String,
    pub data_source: String,
    pub record_count: usize,
}

/// Processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub task_id: String,
    pub records_processed: usize,
    pub success_count: usize,
    pub error_count: usize,
}

/// Batch processing input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessingInput {
    pub batch_id: String,
    pub items: Vec<ProcessingItem>,
}

/// Individual item to process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingItem {
    pub item_id: String,
    pub data: String,
}

/// Batch processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessingResult {
    pub batch_id: String,
    pub processed_items: Vec<String>,
    pub failed_items: Vec<String>,
}

/// Activity that processes data in stages (supports progress queries)
pub async fn process_data_activity(
    ctx: &ActivityContext,
    task: DataProcessingTask,
) -> Result<ProcessingResult, ActivityError> {
    info!(
        "Starting data processing task {} with {} records",
        task.task_id, task.record_count
    );
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    // Process records in batches
    let batch_size = 10;
    let total_batches = (task.record_count + batch_size - 1) / batch_size;
    
    for batch in 0..total_batches {
        // Record heartbeat with progress info
        let progress = format!(
            "{{\"batch\":{},\"total_batches\":{},\"processed\":{}}}",
            batch, total_batches, batch * batch_size
        );
        ctx.record_heartbeat(Some(progress.as_bytes()));
        
        // Simulate processing
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Simulate some failures (5% error rate)
        let records_in_batch = std::cmp::min(batch_size, task.record_count - batch * batch_size);
        let errors_in_batch = if batch % 20 == 19 { 1 } else { 0 };
        success_count += records_in_batch - errors_in_batch;
        error_count += errors_in_batch;
        
        info!(
            "Task {}: Processed batch {}/{} ({} records)",
            task.task_id, batch + 1, total_batches, records_in_batch
        );
    }
    
    info!(
        "Task {} completed: {} succeeded, {} failed",
        task.task_id, success_count, error_count
    );
    
    Ok(ProcessingResult {
        task_id: task.task_id,
        records_processed: success_count + error_count,
        success_count,
        error_count,
    })
}

/// Activity that processes a batch of items
pub async fn process_batch_activity(
    ctx: &ActivityContext,
    batch: BatchProcessingInput,
) -> Result<BatchProcessingResult, ActivityError> {
    info!("Processing batch {} with {} items", batch.batch_id, batch.items.len());
    
    let mut processed = Vec::new();
    let mut failed = Vec::new();
    
    for (idx, item) in batch.items.iter().enumerate() {
        // Record periodic heartbeats
        if idx % 5 == 0 {
            let progress = format!("{{\"progress\":{}/{} }}", idx, batch.items.len());
            ctx.record_heartbeat(Some(progress.as_bytes()));
        }
        
        // Simulate processing
        tokio::time::sleep(Duration::from_millis(20)).await;
        
        // Simulate occasional failures
        if item.data.contains("error") {
            failed.push(item.item_id.clone());
        } else {
            processed.push(item.item_id.clone());
        }
    }
    
    info!(
        "Batch {} completed: {} processed, {} failed",
        batch.batch_id, processed.len(), failed.len()
    );
    
    Ok(BatchProcessingResult {
        batch_id: batch.batch_id,
        processed_items: processed,
        failed_items: failed,
    })
}

/// Activity that validates data
pub async fn validate_data_activity(
    _ctx: &ActivityContext,
    data_source: String,
) -> Result<bool, ActivityError> {
    info!("Validating data source: {}", data_source);
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Simple validation - reject empty or error sources
    if data_source.is_empty() || data_source.contains("invalid") {
        return Ok(false);
    }
    
    Ok(true)
}
