//! Activity implementations for continue-as-new example.
//!
//! This example demonstrates activities for processing data in batches
//! and managing long-running operations.

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Batch processing input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessInput {
    pub batch_id: String,
    pub items: Vec<ProcessItem>,
    pub iteration: u32,
}

/// Individual item to process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessItem {
    pub item_id: String,
    pub data: serde_json::Value,
}

/// Batch processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessResult {
    pub batch_id: String,
    pub processed_count: usize,
    pub failed_count: usize,
    pub iteration: u32,
}

/// Data fetch input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFetchInput {
    pub cursor: Option<String>,
    pub batch_size: usize,
}

/// Data fetch result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFetchResult {
    pub items: Vec<ProcessItem>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub total_processed: u64,
}

/// Checkpoint data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointData {
    pub iteration: u32,
    pub total_processed: u64,
    pub last_cursor: Option<String>,
    pub metadata: serde_json::Value,
}

/// Process a batch of items
pub async fn process_batch_activity(
    ctx: &ActivityContext,
    input: BatchProcessInput,
) -> Result<BatchProcessResult, ActivityError> {
    info!(
        "Processing batch {} (iteration {}): {} items",
        input.batch_id, input.iteration, input.items.len()
    );
    
    let mut processed = 0;
    let mut failed = 0;
    
    for (idx, item) in input.items.iter().enumerate() {
        // Simulate processing
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Record heartbeat periodically
        if idx % 10 == 0 {
            ctx.record_heartbeat(Some(&serde_json::to_vec(&serde_json::json!({
                "progress": idx,
                "total": input.items.len(),
                "iteration": input.iteration,
            })).unwrap()));
        }
        
        // Simulate occasional failures
        if item.item_id.contains("fail") {
            failed += 1;
        } else {
            processed += 1;
        }
    }
    
    info!(
        "Batch {} completed: {} processed, {} failed",
        input.batch_id, processed, failed
    );
    
    Ok(BatchProcessResult {
        batch_id: input.batch_id,
        processed_count: processed,
        failed_count: failed,
        iteration: input.iteration,
    })
}

/// Fetch data from external source
pub async fn fetch_data_activity(
    _ctx: &ActivityContext,
    input: DataFetchInput,
) -> Result<DataFetchResult, ActivityError> {
    info!(
        "Fetching data (cursor: {:?}, batch_size: {})",
        input.cursor, input.batch_size
    );
    
    // Simulate data fetch delay
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Generate mock data
    let base_id = input.cursor.as_ref()
        .and_then(|c| c.parse::<u64>().ok())
        .unwrap_or(0);
    
    let mut items = Vec::new();
    for i in 0..input.batch_size {
        let item_id = format!("item_{}", base_id + i as u64);
        items.push(ProcessItem {
            item_id,
            data: serde_json::json!({
                "index": i,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        });
    }
    
    // Determine if there's more data
    let has_more = items.len() >= input.batch_size;
    let next_cursor = if has_more {
        Some((base_id + input.batch_size as u64).to_string())
    } else {
        None
    };
    
    let total_processed = base_id + items.len() as u64;
    
    info!(
        "Fetched {} items, has_more: {}, total: {}",
        items.len(), has_more, total_processed
    );
    
    Ok(DataFetchResult {
        items,
        next_cursor,
        has_more,
        total_processed,
    })
}

/// Save checkpoint for continue-as-new
pub async fn save_checkpoint_activity(
    _ctx: &ActivityContext,
    checkpoint: CheckpointData,
) -> Result<String, ActivityError> {
    info!(
        "Saving checkpoint at iteration {} (total processed: {})",
        checkpoint.iteration, checkpoint.total_processed
    );
    
    // Simulate checkpoint save
    tokio::time::sleep(Duration::from_millis(25)).await;
    
    let checkpoint_id = format!("checkpoint_{}_{}", 
        checkpoint.iteration, 
        chrono::Utc::now().timestamp()
    );
    
    info!("Checkpoint saved: {}", checkpoint_id);
    
    Ok(checkpoint_id)
}
