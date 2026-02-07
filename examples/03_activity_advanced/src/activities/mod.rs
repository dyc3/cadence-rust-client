//! Activity implementations for activity advanced example.
//!
//! This example demonstrates long-running activities with heartbeats,
//! cancellation support, and deadline management.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};
use uber_cadence_activity::ActivityContext;
use uber_cadence_worker::ActivityError;

/// Input for file processing activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProcessingInput {
    pub file_path: String,
    pub chunk_size: usize,
}

/// Progress information for heartbeats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingProgress {
    pub chunks_processed: usize,
    pub total_chunks: usize,
    pub bytes_processed: u64,
    pub current_file: String,
}

/// Result of file processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProcessingResult {
    pub file_path: String,
    pub chunks_processed: usize,
    pub total_bytes: u64,
    pub checksum: String,
}

/// Long-running file processing activity with heartbeats
///
/// This activity simulates processing a large file in chunks,
/// recording heartbeats periodically to report progress.
pub async fn process_large_file_activity(
    ctx: &ActivityContext,
    input: FileProcessingInput,
) -> Result<FileProcessingResult, ActivityError> {
    let info = ctx.get_info();
    info!(
        "Starting file processing for {} (attempt {})",
        input.file_path, info.attempt
    );

    // Check for heartbeat details from previous attempt
    let mut progress = if ctx.has_heartbeat_details() {
        if let Some(details) = ctx.get_heartbeat_details() {
            match serde_json::from_slice::<ProcessingProgress>(details) {
                Ok(saved_progress) => {
                    info!(
                        "Resuming from previous attempt: {} chunks already processed",
                        saved_progress.chunks_processed
                    );
                    saved_progress
                }
                Err(_) => {
                    warn!("Failed to parse heartbeat details, starting from beginning");
                    ProcessingProgress {
                        chunks_processed: 0,
                        total_chunks: 10, // Simulated
                        bytes_processed: 0,
                        current_file: input.file_path.clone(),
                    }
                }
            }
        } else {
            ProcessingProgress {
                chunks_processed: 0,
                total_chunks: 10,
                bytes_processed: 0,
                current_file: input.file_path.clone(),
            }
        }
    } else {
        ProcessingProgress {
            chunks_processed: 0,
            total_chunks: 10,
            bytes_processed: 0,
            current_file: input.file_path.clone(),
        }
    };

    // Simulate processing chunks
    let total_chunks = progress.total_chunks;
    let mut checksum = format!("init-{}", progress.chunks_processed);

    for chunk_idx in progress.chunks_processed..total_chunks {
        // Check if activity has been cancelled
        if ctx.is_cancelled() {
            info!("Activity cancelled, saving progress and exiting");
            // Save current progress as heartbeat details
            let details = serde_json::to_vec(&progress).map_err(|e| {
                ActivityError::ExecutionFailed(format!("Failed to serialize progress: {}", e))
            })?;
            ctx.record_heartbeat(Some(&details));
            return Err(ActivityError::ExecutionFailed(
                "Activity cancelled by request".to_string(),
            ));
        }

        // Check deadline
        if let Some(remaining) = ctx.get_remaining_time() {
            if remaining < Duration::from_secs(5) {
                warn!("Approaching deadline, saving progress");
                let details = serde_json::to_vec(&progress).map_err(|e| {
                    ActivityError::ExecutionFailed(format!("Failed to serialize progress: {}", e))
                })?;
                ctx.record_heartbeat(Some(&details));
            }
        }

        info!(
            "Processing chunk {}/{} for {}",
            chunk_idx + 1,
            total_chunks,
            input.file_path
        );

        // Simulate work
        sleep(Duration::from_millis(100)).await;

        // Update progress
        progress.chunks_processed = chunk_idx + 1;
        progress.bytes_processed += input.chunk_size as u64;
        checksum = format!("checksum-{}-{}", chunk_idx, progress.bytes_processed);

        // Record heartbeat every 3 chunks
        if (chunk_idx + 1) % 3 == 0 || chunk_idx == total_chunks - 1 {
            let details = serde_json::to_vec(&progress).map_err(|e| {
                ActivityError::ExecutionFailed(format!("Failed to serialize progress: {}", e))
            })?;
            ctx.record_heartbeat(Some(&details));
            info!(
                "Recorded heartbeat: {} chunks processed",
                progress.chunks_processed
            );
        }
    }

    info!(
        "File processing completed: {} chunks, {} bytes",
        progress.chunks_processed, progress.bytes_processed
    );

    Ok(FileProcessingResult {
        file_path: input.file_path,
        chunks_processed: progress.chunks_processed,
        total_bytes: progress.bytes_processed,
        checksum,
    })
}

/// Input for data migration activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMigrationInput {
    pub source_table: String,
    pub destination_table: String,
    pub batch_size: usize,
}

/// Migration progress for heartbeats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationProgress {
    pub records_migrated: usize,
    pub total_records: usize,
    pub last_record_id: String,
}

/// Result of data migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub records_migrated: usize,
    pub source_table: String,
    pub destination_table: String,
    pub duration_secs: u64,
}

/// Data migration activity with cancellation support
///
/// This activity demonstrates handling cancellation gracefully,
/// allowing partial work to be saved via heartbeats.
pub async fn migrate_data_activity(
    ctx: &ActivityContext,
    input: DataMigrationInput,
) -> Result<MigrationResult, ActivityError> {
    let info = ctx.get_info();
    info!(
        "Starting data migration from {} to {} (attempt {})",
        input.source_table, input.destination_table, info.attempt
    );

    // Resume from previous progress if available
    let mut progress = if ctx.has_heartbeat_details() {
        if let Some(details) = ctx.get_heartbeat_details() {
            serde_json::from_slice::<MigrationProgress>(details).unwrap_or(MigrationProgress {
                records_migrated: 0,
                total_records: 100, // Simulated
                last_record_id: "start".to_string(),
            })
        } else {
            MigrationProgress {
                records_migrated: 0,
                total_records: 100,
                last_record_id: "start".to_string(),
            }
        }
    } else {
        MigrationProgress {
            records_migrated: 0,
            total_records: 100,
            last_record_id: "start".to_string(),
        }
    };

    let start_time = std::time::Instant::now();
    let total_records = progress.total_records;

    while progress.records_migrated < total_records {
        // Check for cancellation
        if ctx.is_cancelled() {
            info!("Migration cancelled, saving progress");
            let details = serde_json::to_vec(&progress).map_err(|e| {
                ActivityError::ExecutionFailed(format!("Failed to serialize progress: {}", e))
            })?;
            ctx.record_heartbeat(Some(&details));
            return Err(ActivityError::ExecutionFailed(format!(
                "Migration cancelled after {} records",
                progress.records_migrated
            )));
        }

        // Check worker stop signal
        if let Some(stop_rx) = ctx.get_worker_stop_channel() {
            if *stop_rx.borrow() {
                info!("Worker stopping, saving migration progress");
                let details = serde_json::to_vec(&progress).map_err(|e| {
                    ActivityError::ExecutionFailed(format!("Failed to serialize progress: {}", e))
                })?;
                ctx.record_heartbeat(Some(&details));
                return Err(ActivityError::ExecutionFailed(
                    "Worker shutting down".to_string(),
                ));
            }
        }

        // Process a batch
        let batch_count = input
            .batch_size
            .min(total_records - progress.records_migrated);
        info!(
            "Migrating batch of {} records ({} of {})",
            batch_count,
            progress.records_migrated + batch_count,
            total_records
        );

        // Simulate migration work
        sleep(Duration::from_millis(50)).await;

        // Update progress
        progress.records_migrated += batch_count;
        progress.last_record_id = format!("record_{}", progress.records_migrated);

        // Record heartbeat every 20 records
        if progress.records_migrated % 20 == 0 {
            let details = serde_json::to_vec(&progress).map_err(|e| {
                ActivityError::ExecutionFailed(format!("Failed to serialize progress: {}", e))
            })?;
            ctx.record_heartbeat(Some(&details));
            info!(
                "Recorded heartbeat: {} records migrated",
                progress.records_migrated
            );
        }
    }

    let duration = start_time.elapsed();

    info!(
        "Data migration completed: {} records in {:?}",
        progress.records_migrated, duration
    );

    Ok(MigrationResult {
        records_migrated: progress.records_migrated,
        source_table: input.source_table,
        destination_table: input.destination_table,
        duration_secs: duration.as_secs(),
    })
}

/// Input for deadline-aware activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlineAwareInput {
    pub task_name: String,
    pub estimated_duration_secs: u64,
}

/// Result from deadline-aware processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlineAwareResult {
    pub task_name: String,
    pub completed: bool,
    pub items_processed: usize,
    pub stopped_by_deadline: bool,
}

/// Activity that respects deadlines
///
/// This activity checks its deadline and stops gracefully when
/// running out of time, reporting partial results.
pub async fn deadline_aware_activity(
    ctx: &ActivityContext,
    input: DeadlineAwareInput,
) -> Result<DeadlineAwareResult, ActivityError> {
    info!(
        "Starting deadline-aware task: {} (estimated {}s)",
        input.task_name, input.estimated_duration_secs
    );

    // Check initial deadline
    if let Some(remaining) = ctx.get_remaining_time() {
        info!("Initial time remaining: {:?}", remaining);

        if remaining < Duration::from_secs(input.estimated_duration_secs) {
            warn!(
                "Warning: remaining time ({:?}) is less than estimated duration",
                remaining
            );
        }
    } else {
        info!("No deadline set for this activity");
    }

    let mut items_processed = 0;
    let mut stopped_by_deadline = false;

    // Simulate processing items with deadline checks
    for _i in 0..100 {
        // Check deadline before each item
        if let Some(remaining) = ctx.get_remaining_time() {
            if remaining < Duration::from_millis(200) {
                info!(
                    "Approaching deadline with {}ms remaining, stopping gracefully",
                    remaining.as_millis()
                );
                stopped_by_deadline = true;
                break;
            }
        }

        // Check cancellation
        if ctx.is_cancelled() {
            info!("Activity cancelled after {} items", items_processed);
            return Err(ActivityError::ExecutionFailed("Cancelled".to_string()));
        }

        // Simulate processing
        sleep(Duration::from_millis(50)).await;
        items_processed += 1;

        // Record progress periodically
        if items_processed % 10 == 0 {
            ctx.record_heartbeat(Some(&(items_processed as u32).to_le_bytes()));
        }
    }

    let completed = !stopped_by_deadline && items_processed == 100;

    info!(
        "Task {} finished: {} items processed, completed: {}",
        input.task_name, items_processed, completed
    );

    Ok(DeadlineAwareResult {
        task_name: input.task_name,
        completed,
        items_processed,
        stopped_by_deadline,
    })
}
