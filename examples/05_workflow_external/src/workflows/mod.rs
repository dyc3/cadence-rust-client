//! Workflow implementations for workflow external example.
//!
//! This example demonstrates signaling and canceling external workflows.

use crate::activities::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};
use uber_cadence_core::ActivityOptions;
use uber_cadence_workflow::context::WorkflowError;
use uber_cadence_workflow::WorkflowContext;

/// Signal to notify about dependency completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCompletedSignal {
    pub dependency_workflow_id: String,
    pub result: String,
}

/// Signal to request task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteTaskSignal {
    pub task_id: String,
    pub task_type: String,
    pub parameters: serde_json::Value,
}

/// Result from orchestrator workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorResult {
    pub orchestrator_id: String,
    pub managed_workflows: Vec<String>,
    pub completed_workflows: Vec<String>,
    pub cancelled_workflows: Vec<String>,
}

/// Orchestrator workflow that manages multiple child workflows
///
/// This workflow demonstrates:
/// - Starting multiple related workflows
/// - Signaling workflows to coordinate execution
/// - Cancelling workflows when dependencies fail
/// - Monitoring workflow completion
pub async fn orchestrator_workflow(
    ctx: &mut WorkflowContext,
    workflow_ids: Vec<String>,
) -> Result<OrchestratorResult, WorkflowError> {
    let orchestrator_id = format!("orch_{}", chrono::Utc::now().timestamp());
    info!(
        "Starting orchestrator {} for {} workflows",
        orchestrator_id,
        workflow_ids.len()
    );

    let mut completed_workflows = vec![];
    let mut cancelled_workflows = vec![];

    // Track which workflows are still running
    let mut running_workflows: Vec<String> = workflow_ids.clone();

    // Simulate workflow coordination
    for (idx, workflow_id) in workflow_ids.iter().enumerate() {
        info!(
            "Managing workflow {}/{}: {}",
            idx + 1,
            workflow_ids.len(),
            workflow_id
        );

        // Send execution signal to the workflow
        let signal = ExecuteTaskSignal {
            task_id: format!("task_{}", idx),
            task_type: "process".to_string(),
            parameters: serde_json::json!({
                "orchestrator_id": orchestrator_id,
                "sequence": idx,
            }),
        };

        ctx.signal_external_workflow(
            workflow_id,
            None,
            "execute_task",
            Some(serde_json::to_vec(&signal).unwrap()),
        )
        .await?;

        info!("Sent execute_task signal to {}", workflow_id);

        // Log the coordination activity
        let log_input = LogActivityInput {
            workflow_id: orchestrator_id.clone(),
            activity_name: "orchestrate".to_string(),
            message: format!("Signaled workflow {}", workflow_id),
            level: LogLevel::Info,
        };

        let _ = ctx
            .execute_activity(
                "log_execution",
                Some(serde_json::to_vec(&log_input).unwrap()),
                ActivityOptions::default(),
            )
            .await;
    }

    // Simulate monitoring completion (in real scenario, would poll or wait for completion signals)
    for workflow_id in &workflow_ids {
        ctx.sleep(Duration::from_millis(100)).await;

        // Simulate success for most, failure for some
        if workflow_id.contains("fail") {
            warn!(
                "Workflow {} failed, cancelling dependent workflows",
                workflow_id
            );

            // Cancel dependent workflows
            for other_id in &running_workflows {
                if other_id != workflow_id {
                    info!("Cancelling workflow {} due to dependency failure", other_id);

                    ctx.request_cancel_external_workflow(other_id, None).await?;

                    cancelled_workflows.push(other_id.clone());

                    // Notify about cancellation
                    let notif_input = NotifyInput {
                        target_workflow_id: other_id.clone(),
                        notification_type: "cancellation".to_string(),
                        message: format!("Cancelled due to failure in {}", workflow_id),
                    };

                    let _ = ctx
                        .execute_activity(
                            "send_external_notification",
                            Some(serde_json::to_vec(&notif_input).unwrap()),
                            ActivityOptions::default(),
                        )
                        .await;
                }
            }

            // Remove cancelled workflows from running list
            running_workflows.retain(|id| !cancelled_workflows.contains(id));
        } else {
            completed_workflows.push(workflow_id.clone());
        }
    }

    info!(
        "Orchestrator {} completed: {} completed, {} cancelled",
        orchestrator_id,
        completed_workflows.len(),
        cancelled_workflows.len()
    );

    Ok(OrchestratorResult {
        orchestrator_id,
        managed_workflows: workflow_ids,
        completed_workflows,
        cancelled_workflows,
    })
}

/// Signal to notify about data availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataReadySignal {
    pub data_id: String,
    pub data_location: String,
    pub producer_workflow_id: String,
}

/// Result from producer workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerResult {
    pub producer_id: String,
    pub data_id: String,
    pub data_location: String,
    pub consumers_notified: Vec<String>,
}

/// Producer workflow that notifies consumers when data is ready
///
/// This demonstrates the producer-consumer pattern using external signals.
pub async fn producer_workflow(
    ctx: &mut WorkflowContext,
    (data_id, consumer_workflow_ids): (String, Vec<String>),
) -> Result<ProducerResult, WorkflowError> {
    let producer_id = format!("producer_{}", chrono::Utc::now().timestamp());
    info!("Starting producer {} for data {}", producer_id, data_id);

    // Simulate data production
    ctx.sleep(Duration::from_millis(200)).await;

    let data_location = format!("s3://bucket/data/{}.parquet", data_id);

    info!("Data {} produced at {}", data_id, data_location);

    // Notify all consumers
    let mut consumers_notified = vec![];

    for consumer_id in &consumer_workflow_ids {
        let signal = DataReadySignal {
            data_id: data_id.clone(),
            data_location: data_location.clone(),
            producer_workflow_id: producer_id.clone(),
        };

        match ctx
            .signal_external_workflow(
                consumer_id,
                None,
                "data_ready",
                Some(serde_json::to_vec(&signal).unwrap()),
            )
            .await
        {
            Ok(_) => {
                info!("Notified consumer {} about data {}", consumer_id, data_id);
                consumers_notified.push(consumer_id.clone());
            }
            Err(e) => {
                warn!("Failed to notify consumer {}: {}", consumer_id, e);
            }
        }

        // Log the notification
        let log_input = LogActivityInput {
            workflow_id: producer_id.clone(),
            activity_name: "notify_consumer".to_string(),
            message: format!("Notified {} about data {}", consumer_id, data_id),
            level: LogLevel::Info,
        };

        let _ = ctx
            .execute_activity(
                "log_execution",
                Some(serde_json::to_vec(&log_input).unwrap()),
                ActivityOptions::default(),
            )
            .await;
    }

    info!(
        "Producer {} completed: notified {} of {} consumers",
        producer_id,
        consumers_notified.len(),
        consumer_workflow_ids.len()
    );

    Ok(ProducerResult {
        producer_id,
        data_id,
        data_location,
        consumers_notified,
    })
}

/// Input for consumer workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerInput {
    pub consumer_id: String,
    pub expected_data_types: Vec<String>,
}

/// Result from consumer workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerResult {
    pub consumer_id: String,
    pub data_processed: Vec<String>,
    pub processing_completed: bool,
}

/// Consumer workflow that processes data from producers
///
/// This demonstrates receiving external signals and processing data.
pub async fn consumer_workflow(
    ctx: &mut WorkflowContext,
    input: ConsumerInput,
) -> Result<ConsumerResult, WorkflowError> {
    info!(
        "Starting consumer {} expecting {} data types",
        input.consumer_id,
        input.expected_data_types.len()
    );

    // Create signal channel for data_ready signals
    let mut data_channel = ctx.get_signal_channel("data_ready");

    let mut data_processed = vec![];
    let timeout = Duration::from_secs(30);
    let start_time = ctx.now();

    // Wait for data from producers
    loop {
        // Check timeout
        let elapsed = ctx.now() - start_time;
        if elapsed > chrono::Duration::from_std(timeout).unwrap_or_default() {
            info!("Consumer {} timed out waiting for data", input.consumer_id);
            break;
        }

        // Check if we have all expected data
        if data_processed.len() >= input.expected_data_types.len() {
            info!("Consumer {} received all expected data", input.consumer_id);
            break;
        }

        // Wait for data_ready signal
        match data_channel.recv().await {
            Some(data) => {
                if let Ok(signal) = serde_json::from_slice::<DataReadySignal>(&data) {
                    info!(
                        "Consumer {} received data {} from producer {}",
                        input.consumer_id, signal.data_id, signal.producer_workflow_id
                    );

                    // Simulate processing
                    ctx.sleep(Duration::from_millis(100)).await;

                    let data_id = signal.data_id.clone();
                    data_processed.push(data_id.clone());

                    // Log processing
                    let log_input = LogActivityInput {
                        workflow_id: input.consumer_id.clone(),
                        activity_name: "process_data".to_string(),
                        message: format!(
                            "Processed data {} from {}",
                            data_id, signal.producer_workflow_id
                        ),
                        level: LogLevel::Info,
                    };

                    let _ = ctx
                        .execute_activity(
                            "log_execution",
                            Some(serde_json::to_vec(&log_input).unwrap()),
                            ActivityOptions::default(),
                        )
                        .await;
                }
            }
            None => {
                // No signal yet, continue waiting
                ctx.sleep(Duration::from_millis(50)).await;
            }
        }
    }

    let processing_completed = data_processed.len() >= input.expected_data_types.len();

    info!(
        "Consumer {} completed: processed {} data items, completed: {}",
        input.consumer_id,
        data_processed.len(),
        processing_completed
    );

    Ok(ConsumerResult {
        consumer_id: input.consumer_id,
        data_processed,
        processing_completed,
    })
}

/// Signal for timer expiration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerExpiredSignal {
    pub timer_id: String,
    pub target_workflow_id: String,
}

/// Result from timer workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerResult {
    pub timer_id: String,
    pub target_notified: bool,
    pub duration_ms: u64,
}

/// Timer workflow that signals another workflow after a delay
///
/// This demonstrates using workflows as timers for other workflows.
pub async fn timer_workflow(
    ctx: &mut WorkflowContext,
    (timer_id, target_workflow_id, duration_ms): (String, String, u64),
) -> Result<TimerResult, WorkflowError> {
    info!(
        "Starting timer {} for workflow {} (delay: {}ms)",
        timer_id, target_workflow_id, duration_ms
    );

    // Sleep for the specified duration
    ctx.sleep(Duration::from_millis(duration_ms)).await;

    info!(
        "Timer {} expired, signaling workflow {}",
        timer_id, target_workflow_id
    );

    // Send signal to target workflow
    let signal = TimerExpiredSignal {
        timer_id: timer_id.clone(),
        target_workflow_id: target_workflow_id.clone(),
    };

    let target_notified = ctx
        .signal_external_workflow(
            &target_workflow_id,
            None,
            "timer_expired",
            Some(serde_json::to_vec(&signal).unwrap()),
        )
        .await
        .is_ok();

    if target_notified {
        info!("Successfully notified workflow {}", target_workflow_id);
    } else {
        warn!("Failed to notify workflow {}", target_workflow_id);
    }

    Ok(TimerResult {
        timer_id,
        target_notified,
        duration_ms,
    })
}
