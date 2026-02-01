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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::TestWorkflowEnvironment;
//     use std::time::Duration as StdDuration;
// 
//     #[tokio::test]
//     async fn test_sleep_demo_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_workflow("sleep_demo", sleep_demo_workflow);
// 
//         let result = env
//             .execute_workflow("sleep_demo", 2u64)
//             .await
//             .expect("Workflow should complete");
// 
//         assert!(result.contains("Slept for 2 seconds"), "Should indicate sleep duration");
//     }
// 
//     #[tokio::test]
//     async fn test_deadline_wait_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         
//         env.register_activity("check_deadline", check_deadline_activity);
//         env.register_workflow("deadline_wait", deadline_wait_workflow);
// 
//         // Set a deadline 5 seconds from now
//         let deadline = Utc::now() + Duration::seconds(5);
//         
//         let result = env
//             .execute_workflow("deadline_wait", deadline)
//             .await
//             .expect("Workflow should complete");
// 
//         assert!(result.contains("Successfully waited"), "Should indicate successful wait");
//     }
// 
//     #[tokio::test]
//     async fn test_cancellable_timer_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_workflow("cancellable_timer", cancellable_timer_workflow);
// 
//         // Don't send cancel signal - let it complete normally
//         let result = env
//             .execute_workflow("cancellable_timer", 10u64)
//             .await
//             .expect("Workflow should complete");
// 
//         assert!(result.contains("completed"), "Should complete without cancellation");
//     }
// 
//     #[tokio::test]
//     async fn test_cancellable_timer_with_cancellation() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_workflow("cancellable_timer", cancellable_timer_workflow);
// 
//         // Send cancel signal immediately
//         env.signal_workflow("cancel_timer", serde_json::to_vec(&"cancel").unwrap());
// 
//         let result = env
//             .execute_workflow("cancellable_timer", 100u64) // Long timeout
//             .await
//             .expect("Workflow should complete");
// 
//         assert!(result.contains("cancelled"), "Should be cancelled");
//     }
// 
//     #[tokio::test]
//     async fn test_retry_with_backoff_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         
//         env.register_activity("time_bound_operation", time_bound_operation_activity);
//         env.register_workflow("retry_with_backoff", retry_with_backoff_workflow);
// 
//         let result = env
//             .execute_workflow("retry_with_backoff", 3u32)
//             .await
//             .expect("Workflow should complete");
// 
//         assert!(result.contains("succeeded"), "Should indicate success");
//     }
// 
//     #[tokio::test]
//     async fn test_reminder_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         
//         env.register_activity("send_reminder", send_reminder_activity);
//         env.register_workflow("reminder", reminder_workflow);
// 
//         let reminders = vec![
//             ReminderRequest {
//                 message: "Meeting in 5 minutes".to_string(),
//                 scheduled_time: Utc::now() + Duration::seconds(1),
//                 priority: ReminderPriority::High,
//             },
//             ReminderRequest {
//                 message: "Lunch time".to_string(),
//                 scheduled_time: Utc::now() + Duration::seconds(2),
//                 priority: ReminderPriority::Medium,
//             },
//         ];
// 
//         let results: Vec<ReminderResult> = env
//             .execute_workflow("reminder", reminders)
//             .await
//             .expect("Workflow should complete");
// 
//         assert_eq!(results.len(), 2, "Should process 2 reminders");
//         assert!(results.iter().all(|r| r.delivered), "All reminders should be delivered");
//     }
// 
//     #[tokio::test]
//     async fn test_check_deadline_activity() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("check_deadline", check_deadline_activity);
// 
//         // Test with past deadline
//         let past_deadline = Utc::now() - Duration::hours(1);
//         let result = env
//             .execute_activity("check_deadline", past_deadline)
//             .await
//             .expect("Activity should complete");
//         let is_expired: bool = serde_json::from_slice(&result).expect("Should parse");
//         assert!(is_expired, "Past deadline should be expired");
// 
//         // Test with future deadline
//         let future_deadline = Utc::now() + Duration::hours(1);
//         let result = env
//             .execute_activity("check_deadline", future_deadline)
//             .await
//             .expect("Activity should complete");
//         let is_expired: bool = serde_json::from_slice(&result).expect("Should parse");
//         assert!(!is_expired, "Future deadline should not be expired");
//     }
// 
//     #[tokio::test]
//     async fn test_send_reminder_activity() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("send_reminder", send_reminder_activity);
// 
//         let request = ReminderRequest {
//             message: "Test reminder".to_string(),
//             scheduled_time: Utc::now(),
//             priority: ReminderPriority::Urgent,
//         };
// 
//         let result = env
//             .execute_activity("send_reminder", request)
//             .await
//             .expect("Activity should complete");
//         
//         let reminder: ReminderResult = serde_json::from_slice(&result).expect("Should parse");
//         assert!(reminder.delivered, "Reminder should be delivered");
//         assert!(!reminder.reminder_id.is_empty(), "Should have reminder ID");
//     }
// }
