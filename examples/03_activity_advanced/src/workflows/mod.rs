//! Workflow implementations for activity advanced example.

use crate::activities::*;
use cadence_core::ActivityOptions;
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use std::time::Duration;
use tracing::info;

/// Workflow that demonstrates activity heartbeats and resumption
pub async fn file_processing_workflow(
    ctx: &mut WorkflowContext,
    input: FileProcessingInput,
) -> Result<FileProcessingResult, WorkflowError> {
    info!("Starting file processing workflow for: {}", input.file_path);

    // Execute the file processing activity with a long timeout
    let activity_options = ActivityOptions {
        schedule_to_close_timeout: Duration::from_secs(300),
        heartbeat_timeout: Duration::from_secs(30),
        ..Default::default()
    };

    let result = ctx
        .execute_activity(
            "process_large_file",
            Some(serde_json::to_vec(&input).unwrap()),
            activity_options,
        )
        .await?;

    let processing_result: FileProcessingResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

    info!(
        "File processing completed: {} chunks processed",
        processing_result.chunks_processed
    );

    Ok(processing_result)
}

/// Workflow that demonstrates data migration with cancellation support
pub async fn data_migration_workflow(
    ctx: &mut WorkflowContext,
    input: DataMigrationInput,
) -> Result<MigrationResult, WorkflowError> {
    info!(
        "Starting data migration workflow: {} -> {}",
        input.source_table,
        input.destination_table
    );

    let activity_options = ActivityOptions {
        schedule_to_close_timeout: Duration::from_secs(600),
        heartbeat_timeout: Duration::from_secs(60),
        ..Default::default()
    };

    let result = ctx
        .execute_activity(
            "migrate_data",
            Some(serde_json::to_vec(&input).unwrap()),
            activity_options,
        )
        .await?;

    let migration_result: MigrationResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

    info!(
        "Data migration completed: {} records migrated",
        migration_result.records_migrated
    );

    Ok(migration_result)
}

/// Workflow that demonstrates deadline handling
pub async fn deadline_aware_workflow(
    ctx: &mut WorkflowContext,
    input: DeadlineAwareInput,
) -> Result<DeadlineAwareResult, WorkflowError> {
    info!("Starting deadline-aware workflow for task: {}", input.task_name);

    // Set a tight deadline to demonstrate deadline handling
    let activity_options = ActivityOptions {
        schedule_to_close_timeout: Duration::from_secs(5),
        heartbeat_timeout: Duration::from_secs(2),
        ..Default::default()
    };

    let result = ctx
        .execute_activity(
            "deadline_aware_task",
            Some(serde_json::to_vec(&input).unwrap()),
            activity_options,
        )
        .await?;

    let task_result: DeadlineAwareResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

    if task_result.stopped_by_deadline {
        info!("Task stopped gracefully due to deadline");
    } else if task_result.completed {
        info!("Task completed successfully before deadline");
    }

    Ok(task_result)
}

/// Workflow that demonstrates retry with heartbeat recovery
/// 
/// This workflow handles the case where an activity fails partway through
/// and needs to resume from where it left off.
pub async fn resilient_processing_workflow(
    ctx: &mut WorkflowContext,
    input: FileProcessingInput,
) -> Result<FileProcessingResult, WorkflowError> {
    info!(
        "Starting resilient processing workflow for: {}",
        input.file_path
    );

    // Configure retry policy
    let retry_policy = cadence_core::RetryPolicy {
        initial_interval: Duration::from_secs(1),
        backoff_coefficient: 2.0,
        maximum_interval: Duration::from_secs(60),
        maximum_attempts: 3,
        non_retryable_error_types: vec![],
        expiration_interval: Duration::from_secs(0),
    };

    let activity_options = ActivityOptions {
        schedule_to_close_timeout: Duration::from_secs(300),
        heartbeat_timeout: Duration::from_secs(30),
        retry_policy: Some(retry_policy),
        ..Default::default()
    };

    // Execute activity with retry
    let result = ctx
        .execute_activity(
            "process_large_file",
            Some(serde_json::to_vec(&input).unwrap()),
            activity_options,
        )
        .await?;

    let processing_result: FileProcessingResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;

    info!(
        "Resilient processing completed with {} chunks",
        processing_result.chunks_processed
    );

    Ok(processing_result)
}
