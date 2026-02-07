//! Activity implementations for time and sleep example.
//!
//! This example demonstrates time-related activities that can be used
//! with workflow timers for time-based operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;
use uber_cadence_activity::ActivityContext;
use uber_cadence_worker::ActivityError;

/// Reminder request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderRequest {
    pub message: String,
    pub scheduled_time: DateTime<Utc>,
    pub priority: ReminderPriority,
}

/// Reminder priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReminderPriority {
    Low,
    Medium,
    High,
    Urgent,
}

/// Reminder result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderResult {
    pub reminder_id: String,
    pub sent_at: DateTime<Utc>,
    pub delivered: bool,
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub total_timeout_seconds: u64,
    pub retry_interval_seconds: u64,
    pub max_retries: u32,
}

/// Check if a deadline has been reached
pub async fn check_deadline_activity(
    _ctx: &ActivityContext,
    deadline: DateTime<Utc>,
) -> Result<bool, ActivityError> {
    info!("Checking deadline: {}", deadline);

    let now = Utc::now();
    let is_expired = now >= deadline;

    info!("Deadline check: now={}, expired={}", now, is_expired);

    Ok(is_expired)
}

/// Send a reminder notification
pub async fn send_reminder_activity(
    ctx: &ActivityContext,
    request: ReminderRequest,
) -> Result<ReminderResult, ActivityError> {
    info!(
        "Sending reminder: '{}' with priority {:?}",
        request.message, request.priority
    );

    // Simulate delivery delay based on priority
    let delay_ms = match request.priority {
        ReminderPriority::Urgent => 10,
        ReminderPriority::High => 50,
        ReminderPriority::Medium => 100,
        ReminderPriority::Low => 200,
    };

    tokio::time::sleep(Duration::from_millis(delay_ms)).await;

    // Record heartbeat
    ctx.record_heartbeat(None);

    let reminder_id = uuid::Uuid::new_v4().to_string();

    info!("Reminder {} sent successfully", reminder_id);

    Ok(ReminderResult {
        reminder_id,
        sent_at: Utc::now(),
        delivered: true,
    })
}

/// Perform a time-bound operation
pub async fn time_bound_operation_activity(
    ctx: &ActivityContext,
    (operation_name, max_duration_ms): (String, u64),
) -> Result<TimeBoundResult, ActivityError> {
    info!(
        "Executing time-bound operation: {} (max {}ms)",
        operation_name, max_duration_ms
    );

    let start_time = std::time::Instant::now();

    // Simulate variable processing time
    let actual_duration_ms = (max_duration_ms / 2) + (max_duration_ms / 4);
    tokio::time::sleep(Duration::from_millis(actual_duration_ms)).await;

    // Record progress heartbeat
    ctx.record_heartbeat(Some(
        &serde_json::to_vec(&serde_json::json!({
            "progress": 50,
            "operation": &operation_name,
        }))
        .unwrap(),
    ));

    tokio::time::sleep(Duration::from_millis(actual_duration_ms / 2)).await;

    let elapsed = start_time.elapsed();
    let completed_in_time = elapsed.as_millis() <= max_duration_ms as u128;

    info!(
        "Operation {} completed in {:?} (within deadline: {})",
        operation_name, elapsed, completed_in_time
    );

    Ok(TimeBoundResult {
        operation_name,
        completed_in_time,
        actual_duration_ms: elapsed.as_millis() as u64,
    })
}

/// Result from time-bound operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBoundResult {
    pub operation_name: String,
    pub completed_in_time: bool,
    pub actual_duration_ms: u64,
}
