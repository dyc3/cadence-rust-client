//! Workflow implementations for time and sleep example.
//!
//! This example demonstrates various timer and time manipulation patterns
//! in Cadence workflows.

use crate::activities::*;
use chrono::{DateTime, Utc};
use std::time::Duration;
use tracing::{info, warn};
use uber_cadence_core::ActivityOptions;
use uber_cadence_workflow::context::WorkflowError;
use uber_cadence_workflow::WorkflowContext;

/// A workflow that demonstrates basic sleep/timer functionality
pub async fn sleep_demo_workflow(
    ctx: &mut WorkflowContext,
    sleep_duration_seconds: u64,
) -> Result<String, WorkflowError> {
    info!(
        "Starting sleep demo workflow ({} seconds)",
        sleep_duration_seconds
    );

    let start_time = ctx.workflow_info().start_time;
    info!("Workflow started at: {:?}", start_time);

    // Use workflow sleep (durable timer)
    info!("Sleeping for {} seconds...", sleep_duration_seconds);
    ctx.sleep(Duration::from_secs(sleep_duration_seconds)).await;

    let after_sleep = Utc::now();
    info!("Woke up at: {}", after_sleep);

    Ok(format!(
        "Slept for {} seconds. Started at {:?}, woke up at {}",
        sleep_duration_seconds, start_time, after_sleep
    ))
}

/// A workflow that waits for a specific deadline
pub async fn deadline_wait_workflow(
    ctx: &mut WorkflowContext,
    deadline: DateTime<Utc>,
) -> Result<String, WorkflowError> {
    info!("Starting deadline wait workflow. Target: {}", deadline);

    let now = Utc::now();

    if deadline <= now {
        return Ok("Deadline already passed".to_string());
    }

    // Calculate duration until deadline
    let duration_until = deadline - now;
    let duration_secs = duration_until.num_seconds() as u64;

    info!("Waiting {} seconds until deadline...", duration_secs);

    // Sleep until the deadline
    ctx.sleep(Duration::from_secs(duration_secs)).await;

    // Verify deadline has been reached
    let check_result = ctx
        .execute_activity(
            "check_deadline",
            Some(serde_json::to_vec(&deadline).unwrap()),
            ActivityOptions::default(),
        )
        .await?;

    let is_expired: bool = serde_json::from_slice(&check_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse check result: {}", e)))?;

    if is_expired {
        Ok(format!("Successfully waited until deadline: {}", deadline))
    } else {
        Err(WorkflowError::Generic("Deadline not reached".to_string()))
    }
}

/// A workflow that demonstrates timer cancellation
pub async fn cancellable_timer_workflow(
    ctx: &mut WorkflowContext,
    max_wait_seconds: u64,
) -> Result<String, WorkflowError> {
    info!(
        "Starting cancellable timer workflow (max {}s)",
        max_wait_seconds
    );

    // Create a signal channel for cancellation
    let mut cancel_signal = ctx.get_signal_channel("cancel_timer");

    // Use select! to race between timer and cancellation signal
    tokio::select! {
        _ = ctx.sleep(Duration::from_secs(max_wait_seconds)) => {
            info!("Timer completed after {} seconds", max_wait_seconds);
            Ok("Timer completed (no cancellation received)".to_string())
        }
        _ = cancel_signal.recv() => {
            info!("Timer cancelled by signal");
            Ok("Timer cancelled".to_string())
        }
    }
}

/// A workflow that implements retry with exponential backoff
pub async fn retry_with_backoff_workflow(
    ctx: &mut WorkflowContext,
    max_attempts: u32,
) -> Result<String, WorkflowError> {
    info!(
        "Starting retry with backoff workflow (max {} attempts)",
        max_attempts
    );

    let mut last_error = None;

    for attempt in 1..=max_attempts {
        info!("Attempt {} of {}", attempt, max_attempts);

        // Calculate backoff: 1s, 2s, 4s, 8s, ... (capped at 60s)
        let backoff_secs = std::cmp::min(2u64.pow(attempt - 1), 60);

        if attempt > 1 {
            info!("Waiting {} seconds before retry...", backoff_secs);
            ctx.sleep(Duration::from_secs(backoff_secs)).await;
        }

        // Try the operation
        let result = ctx
            .execute_activity(
                "time_bound_operation",
                Some(serde_json::to_vec(&("retry_operation".to_string(), 100u64)).unwrap()),
                ActivityOptions {
                    start_to_close_timeout: Duration::from_secs(10),
                    ..Default::default()
                },
            )
            .await;

        match result {
            Ok(_) => {
                info!("Operation succeeded on attempt {}", attempt);
                return Ok(format!("Operation succeeded on attempt {}", attempt));
            }
            Err(e) => {
                warn!("Attempt {} failed: {:?}", attempt, e);
                last_error = Some(e);
            }
        }
    }

    Err(WorkflowError::Generic(format!(
        "All {} attempts failed. Last error: {:?}",
        max_attempts, last_error
    )))
}

/// A workflow that schedules and manages reminders
pub async fn reminder_workflow(
    ctx: &mut WorkflowContext,
    reminders: Vec<ReminderRequest>,
) -> Result<Vec<ReminderResult>, WorkflowError> {
    info!(
        "Starting reminder workflow with {} reminders",
        reminders.len()
    );

    let mut results = Vec::new();

    for reminder in &reminders {
        let now = Utc::now();

        // Calculate wait time until scheduled time
        if reminder.scheduled_time > now {
            let wait_duration = reminder.scheduled_time - now;
            let wait_secs = wait_duration.num_seconds() as u64;

            info!(
                "Waiting {} seconds to send reminder: '{}'",
                wait_secs, reminder.message
            );

            ctx.sleep(Duration::from_secs(wait_secs)).await;
        }

        // Send the reminder
        let result = ctx
            .execute_activity(
                "send_reminder",
                Some(serde_json::to_vec(reminder).unwrap()),
                ActivityOptions::default(),
            )
            .await?;

        let reminder_result: ReminderResult = serde_json::from_slice(&result).map_err(|e| {
            WorkflowError::Generic(format!("Failed to parse reminder result: {}", e))
        })?;

        info!(
            "Reminder {} sent at {}",
            reminder_result.reminder_id, reminder_result.sent_at
        );

        results.push(reminder_result);
    }

    info!("All {} reminders processed", results.len());
    Ok(results)
}
