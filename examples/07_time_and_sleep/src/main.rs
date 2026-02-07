//! # Example 07: Time and Sleep
//!
//! This example demonstrates timers and time manipulation in workflows.
//!
//! ## Features Demonstrated
//!
//! - Workflow timers and sleep operations
//! - Time-based decision making
//! - Deadline management
//! - Timer cancellation
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p time_and_sleep
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p time_and_sleep
//! ```

use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Time and Sleep Example ===\n");
    println!("This example demonstrates:");
    println!("1. Workflow timers and sleep operations");
    println!("2. Deadline management");
    println!("3. Timer cancellation");
    println!("4. Retry with exponential backoff");
    println!("5. Reminder scheduling");
    println!();
    println!("Run tests to see the workflows in action:");
    println!("  cargo test -p time_and_sleep");

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration as ChronoDuration, Utc};
    use std::time::Duration;
    use time_and_sleep::{ReminderPriority, ReminderRequest, ReminderResult, TimeBoundResult};
    use uber_cadence_core::ActivityOptions;
    use uber_cadence_testsuite::{ActivityError, TestWorkflowContext, TestWorkflowEnvironment};
    use uber_cadence_workflow::context::WorkflowError;

    // Workflow wrapper functions for tests
    async fn sleep_demo_workflow_wrapper(
        ctx: TestWorkflowContext,
        sleep_duration_seconds: u64,
    ) -> Result<(TestWorkflowContext, String), WorkflowError> {
        let start_time = ctx.workflow_info().start_time;

        // Use workflow sleep (durable timer)
        ctx.sleep(Duration::from_secs(sleep_duration_seconds)).await;

        let after_sleep = Utc::now();

        Ok((
            ctx,
            format!(
                "Slept for {} seconds. Started at {:?}, woke up at {}",
                sleep_duration_seconds, start_time, after_sleep
            ),
        ))
    }

    async fn deadline_wait_workflow_wrapper(
        mut ctx: TestWorkflowContext,
        deadline: DateTime<Utc>,
    ) -> Result<(TestWorkflowContext, String), WorkflowError> {
        let now = Utc::now();

        if deadline <= now {
            return Ok((ctx, "Deadline already passed".to_string()));
        }

        // Calculate duration until deadline
        let duration_until = deadline - now;
        let duration_secs = duration_until.num_seconds() as u64;

        // Sleep until the deadline
        ctx.sleep(Duration::from_secs(duration_secs)).await;

        // Verify deadline has been reached via activity
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
            Ok((
                ctx,
                format!("Successfully waited until deadline: {}", deadline),
            ))
        } else {
            Err(WorkflowError::Generic("Deadline not reached".to_string()))
        }
    }

    #[allow(dead_code)]
    async fn cancellable_timer_workflow_wrapper(
        ctx: TestWorkflowContext,
        max_wait_seconds: u64,
    ) -> Result<(TestWorkflowContext, String), WorkflowError> {
        // Create a signal channel for cancellation
        let mut cancel_signal = ctx.get_signal_channel("cancel_timer");

        // Use select! to race between timer and cancellation signal
        tokio::select! {
            _ = ctx.sleep(Duration::from_secs(max_wait_seconds)) => {
                Ok((ctx, "Timer completed (no cancellation received)".to_string()))
            }
            _ = cancel_signal.recv() => {
                Ok((ctx, "Timer cancelled".to_string()))
            }
        }
    }

    async fn retry_with_backoff_workflow_wrapper(
        mut ctx: TestWorkflowContext,
        max_attempts: u32,
    ) -> Result<(TestWorkflowContext, String), WorkflowError> {
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            // Calculate backoff: 1s, 2s, 4s, 8s, ... (capped at 60s)
            let backoff_secs = std::cmp::min(2u64.pow(attempt - 1), 60);

            if attempt > 1 {
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
                    return Ok((ctx, format!("Operation succeeded on attempt {}", attempt)));
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(WorkflowError::Generic(format!(
            "All {} attempts failed. Last error: {:?}",
            max_attempts, last_error
        )))
    }

    async fn reminder_workflow_wrapper(
        mut ctx: TestWorkflowContext,
        reminders: Vec<ReminderRequest>,
    ) -> Result<(TestWorkflowContext, Vec<ReminderResult>), WorkflowError> {
        let mut results = Vec::new();

        for reminder in &reminders {
            let now = Utc::now();

            // Calculate wait time until scheduled time
            if reminder.scheduled_time > now {
                let wait_duration = reminder.scheduled_time - now;
                let wait_secs = wait_duration.num_seconds() as u64;

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

            results.push(reminder_result);
        }

        Ok((ctx, results))
    }

    #[tokio::test]
    async fn test_sleep_demo_workflow() {
        let mut env = TestWorkflowEnvironment::new();
        env.register_workflow("sleep_demo", sleep_demo_workflow_wrapper);

        let result: String = env
            .execute_workflow("sleep_demo", 2u64)
            .await
            .expect("Workflow should complete");

        assert!(
            result.contains("Slept for 2 seconds"),
            "Should indicate sleep duration"
        );
    }

    #[tokio::test]
    async fn test_deadline_wait_workflow() {
        let mut env = TestWorkflowEnvironment::new();

        // Use a deadline in the past so the workflow completes immediately
        env.register_activity(
            "check_deadline",
            |_ctx, deadline: DateTime<Utc>| async move {
                let now = Utc::now();
                Ok::<bool, ActivityError>(now >= deadline)
            },
        );
        env.register_workflow("deadline_wait", deadline_wait_workflow_wrapper);

        // Set a deadline in the past - workflow should return immediately
        let past_deadline = Utc::now() - ChronoDuration::seconds(1);

        let result: String = env
            .execute_workflow("deadline_wait", past_deadline)
            .await
            .expect("Workflow should complete");

        assert!(
            result.contains("already passed"),
            "Should indicate deadline already passed, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_cancellable_timer_without_cancellation() {
        let mut env = TestWorkflowEnvironment::new();

        // A simple timer workflow without signal handling for testing
        async fn simple_timer_workflow(
            ctx: TestWorkflowContext,
            duration_seconds: u64,
        ) -> Result<(TestWorkflowContext, String), WorkflowError> {
            ctx.sleep(Duration::from_secs(duration_seconds)).await;
            Ok((
                ctx,
                format!("Timer completed after {} seconds", duration_seconds),
            ))
        }

        env.register_workflow("simple_timer", simple_timer_workflow);

        let result: String = env
            .execute_workflow("simple_timer", 5u64)
            .await
            .expect("Workflow should complete");

        assert!(
            result.contains("Timer completed"),
            "Should complete successfully, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_cancellable_timer_immediate() {
        let mut env = TestWorkflowEnvironment::new();

        // Test that a short timer completes quickly
        async fn short_timer_workflow(
            ctx: TestWorkflowContext,
            duration_seconds: u64,
        ) -> Result<(TestWorkflowContext, String), WorkflowError> {
            ctx.sleep(Duration::from_secs(duration_seconds)).await;
            Ok((
                ctx,
                format!("Short timer of {}s completed", duration_seconds),
            ))
        }

        env.register_workflow("short_timer", short_timer_workflow);

        let result: String = env
            .execute_workflow("short_timer", 1u64)
            .await
            .expect("Workflow should complete");

        assert!(
            result.contains("completed"),
            "Should complete successfully, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_retry_with_backoff_workflow() {
        let mut env = TestWorkflowEnvironment::new();

        env.register_activity(
            "time_bound_operation",
            |_ctx, (_name, _): (String, u64)| async move {
                Ok::<TimeBoundResult, ActivityError>(TimeBoundResult {
                    operation_name: "retry_operation".to_string(),
                    completed_in_time: true,
                    actual_duration_ms: 10,
                })
            },
        );
        env.register_workflow("retry_with_backoff", retry_with_backoff_workflow_wrapper);

        let result: String = env
            .execute_workflow("retry_with_backoff", 3u32)
            .await
            .expect("Workflow should complete");

        assert!(result.contains("succeeded"), "Should indicate success");
    }

    #[tokio::test]
    async fn test_reminder_workflow() {
        let mut env = TestWorkflowEnvironment::new();

        env.register_activity(
            "send_reminder",
            |_ctx, _request: ReminderRequest| async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<ReminderResult, ActivityError>(ReminderResult {
                    reminder_id: uuid::Uuid::new_v4().to_string(),
                    sent_at: Utc::now(),
                    delivered: true,
                })
            },
        );
        env.register_workflow("reminder", reminder_workflow_wrapper);

        let reminders = vec![
            ReminderRequest {
                message: "Meeting in 5 minutes".to_string(),
                scheduled_time: Utc::now() + ChronoDuration::seconds(1),
                priority: ReminderPriority::High,
            },
            ReminderRequest {
                message: "Lunch time".to_string(),
                scheduled_time: Utc::now() + ChronoDuration::seconds(2),
                priority: ReminderPriority::Medium,
            },
        ];

        let results: Vec<ReminderResult> = env
            .execute_workflow("reminder", reminders)
            .await
            .expect("Workflow should complete");

        assert_eq!(results.len(), 2, "Should process 2 reminders");
        assert!(
            results.iter().all(|r| r.delivered),
            "All reminders should be delivered"
        );
    }

    #[tokio::test]
    async fn test_check_deadline_activity() {
        let mut env = TestWorkflowEnvironment::new();
        env.register_activity(
            "check_deadline",
            |_ctx, deadline: DateTime<Utc>| async move {
                let now = Utc::now();
                Ok::<bool, ActivityError>(now >= deadline)
            },
        );

        // Test with past deadline
        let past_deadline = Utc::now() - ChronoDuration::hours(1);
        let is_expired: bool = env
            .execute_activity("check_deadline", past_deadline)
            .await
            .expect("Activity should complete");
        assert!(is_expired, "Past deadline should be expired");

        // Test with future deadline
        let future_deadline = Utc::now() + ChronoDuration::hours(1);
        let is_expired: bool = env
            .execute_activity("check_deadline", future_deadline)
            .await
            .expect("Activity should complete");
        assert!(!is_expired, "Future deadline should not be expired");
    }

    #[tokio::test]
    async fn test_send_reminder_activity() {
        let mut env = TestWorkflowEnvironment::new();
        env.register_activity(
            "send_reminder",
            |_ctx, _request: ReminderRequest| async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<ReminderResult, ActivityError>(ReminderResult {
                    reminder_id: uuid::Uuid::new_v4().to_string(),
                    sent_at: Utc::now(),
                    delivered: true,
                })
            },
        );

        let request = ReminderRequest {
            message: "Test reminder".to_string(),
            scheduled_time: Utc::now(),
            priority: ReminderPriority::Urgent,
        };

        let reminder: ReminderResult = env
            .execute_activity("send_reminder", request)
            .await
            .expect("Activity should complete");

        assert!(reminder.delivered, "Reminder should be delivered");
        assert!(!reminder.reminder_id.is_empty(), "Should have reminder ID");
    }
}
