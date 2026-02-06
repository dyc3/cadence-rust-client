use cadence_core::ActivityOptions;
use cadence_workflow::WorkflowContext;
use cadence_workflow::WorkflowError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::activities::{Job, JobResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitMergeInput {
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitMergeOutput {
    pub results: Vec<JobResult>,
}

/// Split-merge workflow: fan-out to process multiple jobs in parallel, then fan-in to collect results
pub async fn split_merge_workflow(
    ctx: WorkflowContext,
    input: SplitMergeInput,
) -> Result<SplitMergeOutput, WorkflowError> {
    let job_count = input.jobs.len();
    tracing::info!("Starting split-merge workflow with {} jobs", job_count);

    // Create a channel to collect results
    let (tx, rx) = ctx.new_channel(job_count);

    // Fan-out: spawn a task for each job
    for job in input.jobs {
        let ctx_clone = ctx.clone();
        let tx_clone = tx.clone();

        ctx.spawn(async move {
            tracing::info!("Spawned task for job {}", job.id);

            // Execute activity
            let options = ActivityOptions {
                task_list: "default".to_string(),
                schedule_to_close_timeout: Duration::from_secs(60),
                schedule_to_start_timeout: Duration::from_secs(10),
                start_to_close_timeout: Duration::from_secs(30),
                heartbeat_timeout: Duration::from_secs(10),
                retry_policy: None,
                local_activity: false,
                wait_for_cancellation: false,
            };

            let job_bytes = serde_json::to_vec(&job).unwrap();
            let result_bytes = ctx_clone
                .execute_activity("process_job", Some(job_bytes), options)
                .await?;

            let result: JobResult = serde_json::from_slice(&result_bytes).unwrap();
            tracing::info!("Job {} completed: {:?}", job.id, result);

            // Send result through channel
            let _ = tx_clone.send(result).await;

            Ok::<_, WorkflowError>(())
        });
    }

    // Drop the original sender so the channel closes when all spawned tasks are done
    drop(tx);

    // Fan-in: collect all results from the channel
    let mut results = Vec::new();
    while let Some(result) = rx.recv().await {
        tracing::info!("Received result for job {}", result.job_id);
        results.push(result);
    }

    tracing::info!(
        "Split-merge workflow completed with {} results",
        results.len()
    );

    Ok(SplitMergeOutput { results })
}

/// Simple parallel workflow using spawn without channels
pub async fn parallel_workflow(
    ctx: WorkflowContext,
    values: Vec<u32>,
) -> Result<Vec<u32>, WorkflowError> {
    tracing::info!("Starting parallel workflow with {} values", values.len());

    // Spawn tasks for each value
    let mut handles = Vec::new();
    for value in values {
        let ctx_clone = ctx.clone();

        let handle = ctx.spawn(async move {
            let options = ActivityOptions {
                task_list: "default".to_string(),
                schedule_to_close_timeout: Duration::from_secs(60),
                schedule_to_start_timeout: Duration::from_secs(10),
                start_to_close_timeout: Duration::from_secs(30),
                heartbeat_timeout: Duration::from_secs(10),
                retry_policy: None,
                local_activity: false,
                wait_for_cancellation: false,
            };

            let value_bytes = serde_json::to_vec(&value).unwrap();
            match ctx_clone
                .execute_activity("fast_process", Some(value_bytes), options)
                .await
            {
                Ok(result_bytes) => {
                    let result: u32 = serde_json::from_slice(&result_bytes).unwrap();
                    result
                }
                Err(_e) => {
                    // In case of error, return 0
                    0
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut results = Vec::new();
    for handle in handles {
        let value = handle
            .join()
            .await
            .map_err(|e| WorkflowError::Generic(e.to_string()))?;
        results.push(value);
    }

    tracing::info!("Parallel workflow completed with {} results", results.len());

    Ok(results)
}
