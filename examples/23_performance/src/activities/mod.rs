//! Activity implementations for performance tuning example.
//!
//! This module demonstrates high-performance activity patterns:
//! - Batch processing for bulk operations
//! - Efficient data serialization
//! - Connection reuse patterns

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use tracing::{info, debug};
use std::time::Duration;

/// Batch processing input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchInput<T> {
    pub items: Vec<T>,
    pub batch_size: usize,
}

/// Batch processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub processed_count: usize,
    pub failed_count: usize,
    pub total_duration_ms: u64,
}

/// Data transformation input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformInput {
    pub data: Vec<u8>,
    pub compression_level: u8,
}

/// Data transformation output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformOutput {
    pub transformed_data: Vec<u8>,
    pub original_size: usize,
    pub transformed_size: usize,
}

/// Cache warmup input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheWarmupInput {
    pub keys: Vec<String>,
    pub preload_size: usize,
}

/// Process items in batches for improved throughput
pub async fn batch_process_activity<T: Serialize + for<'de> Deserialize<'de> + Send>(
    ctx: &ActivityContext,
    input: BatchInput<T>,
) -> Result<BatchResult, ActivityError> {
    let start_time = std::time::Instant::now();
    let total_items = input.items.len();
    
    info!("Starting batch processing of {} items", total_items);
    
    let mut processed = 0;
    let mut failed = 0;
    
    // Process in batches
    for (batch_idx, batch) in input.items.chunks(input.batch_size).enumerate() {
        debug!("Processing batch {} with {} items", batch_idx, batch.len());
        
        // Report heartbeat every batch
        ctx.record_heartbeat(Some(&serde_json::to_vec(&processed).unwrap()));
        
        // Simulate batch processing
        for _item in batch {
            // Simulate work with minimal delay
            tokio::time::sleep(Duration::from_micros(100)).await;
            processed += 1;
        }
        
        // Small yield to allow other tasks
        if batch_idx % 10 == 0 {
            tokio::task::yield_now().await;
        }
    }
    
    let duration = start_time.elapsed();
    let throughput = processed as f64 / duration.as_secs_f64();
    
    info!(
        "Batch processing complete: {} items in {:?} ({:.0} items/sec)",
        processed, duration, throughput
    );
    
    Ok(BatchResult {
        processed_count: processed,
        failed_count: failed,
        total_duration_ms: duration.as_millis() as u64,
    })
}

/// Transform and compress data efficiently
pub async fn data_transform_activity(
    _ctx: &ActivityContext,
    input: TransformInput,
) -> Result<TransformOutput, ActivityError> {
    let original_size = input.data.len();
    
    debug!(
        "Transforming {} bytes with compression level {}",
        original_size, input.compression_level
    );
    
    // Simulate data transformation with minimal allocation
    // In real scenarios, this could be compression, encryption, etc.
    let transformed = if input.compression_level > 0 {
        // Simulate compression by reducing size
        let target_size = original_size * (100 - input.compression_level as usize) / 100;
        input.data.iter().cycle().take(target_size).copied().collect()
    } else {
        input.data.clone()
    };
    
    let transformed_size = transformed.len();
    
    Ok(TransformOutput {
        transformed_data: transformed,
        original_size,
        transformed_size,
    })
}

/// Warm up caches for improved performance
pub async fn cache_warmup_activity(
    ctx: &ActivityContext,
    input: CacheWarmupInput,
) -> Result<usize, ActivityError> {
    info!("Warming up cache with {} keys", input.keys.len());
    
    let mut warmed = 0;
    
    for (idx, key) in input.keys.iter().enumerate() {
        // Simulate cache warming
        debug!("Preloading key: {}", key);
        warmed += 1;
        
        // Report progress periodically
        if idx % 100 == 0 {
            ctx.record_heartbeat(Some(&serde_json::to_vec(&idx).unwrap()));
        }
    }
    
    info!("Cache warmup complete: {} keys preloaded", warmed);
    Ok(warmed)
}

/// High-throughput data ingestion activity
pub async fn high_throughput_ingest_activity(
    ctx: &ActivityContext,
    records: Vec<String>,
) -> Result<usize, ActivityError> {
    let record_count = records.len();
    info!("High-throughput ingestion of {} records", record_count);
    
    let mut ingested = 0;
    let batch_size = 1000;
    
    for (idx, batch) in records.chunks(batch_size).enumerate() {
        // Process batch
        ingested += batch.len();
        
        // Report heartbeat every 10 batches
        if idx % 10 == 0 {
            ctx.record_heartbeat(Some(&serde_json::to_vec(&ingested).unwrap()));
        }
        
        // Simulate minimal processing time
        tokio::time::sleep(Duration::from_micros(50)).await;
    }
    
    info!("Ingestion complete: {} records processed", ingested);
    Ok(ingested)
}
