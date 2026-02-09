//! Local activity executor for running local activities in-process.
//!
//! This module provides the executor for local activities, which runs
//! activities synchronously in the workflow worker process without
//! scheduling through the Cadence server.

use crate::local_activity_queue::{LocalActivityQueue, LocalActivityTask};
use crate::registry::{ActivityError, Registry};
use crabdance_activity::ActivityContext;
use crabdance_core::RetryPolicy;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, error, info, warn};

/// Local activity executor
///
/// Polls local activity tasks from the queue and executes them using
/// the activity registry. Results are sent back to the workflow via
/// the task's result channel.
pub struct LocalActivityExecutor {
    registry: Arc<dyn Registry>,
    queue: LocalActivityQueue,
}

impl LocalActivityExecutor {
    /// Create a new local activity executor
    pub fn new(registry: Arc<dyn Registry>, queue: LocalActivityQueue) -> Self {
        Self { registry, queue }
    }

    /// Run the executor loop
    ///
    /// This method continuously polls for local activity tasks and executes them.
    /// It runs until the queue is closed (all senders dropped).
    pub async fn run(&self) {
        info!("starting local activity executor loop");

        while let Some(task) = self.queue.recv().await {
            info!(
                activity_id = %task.activity_id,
                activity_type = %task.activity_type,
                attempt = task.attempt,
                "received local activity task"
            );

            // Execute with retry - result is sent directly to the workflow via task.result_sender
            let _ = self.execute_with_retry(task).await;
        }

        info!("local activity executor loop ended");
    }

    /// Execute a local activity with retry logic
    async fn execute_with_retry(
        &self,
        mut task: LocalActivityTask,
    ) -> Result<Vec<u8>, ActivityError> {
        let mut attempt = task.attempt;
        let retry_policy = task.options.retry_policy.clone();
        let deadline = task.scheduled_time + task.options.schedule_to_close_timeout;

        loop {
            // Check timeout before attempting execution
            if SystemTime::now() >= deadline {
                warn!(
                    activity_type = %task.activity_type,
                    activity_id = %task.activity_id,
                    attempt,
                    "local activity timed out"
                );
                let error =
                    ActivityError::Timeout(crabdance_proto::shared::TimeoutType::ScheduleToClose);
                let _ = task.result_sender.send(Err(error.clone()));
                return Err(error);
            }

            // Execute the activity
            let result = self.execute_once(&task, attempt).await;

            match result {
                Ok(output) => {
                    info!(
                        activity_type = %task.activity_type,
                        activity_id = %task.activity_id,
                        attempt,
                        "local activity succeeded"
                    );
                    let _ = task.result_sender.send(Ok(output.clone()));
                    return Ok(output);
                }
                Err(err) if !should_retry(&err, &retry_policy, attempt) => {
                    error!(
                        activity_type = %task.activity_type,
                        activity_id = %task.activity_id,
                        error = %err,
                        "local activity failed permanently"
                    );
                    let _ = task.result_sender.send(Err(err.clone()));
                    return Err(err);
                }
                Err(err) => {
                    warn!(
                        activity_type = %task.activity_type,
                        activity_id = %task.activity_id,
                        attempt,
                        error = %err,
                        "local activity failed, may retry"
                    );

                    // Calculate backoff and check if we have time to retry
                    let backoff = calculate_backoff(&retry_policy, attempt);
                    let backoff_deadline = SystemTime::now() + backoff;

                    if backoff_deadline >= deadline {
                        warn!(
                            backoff_ms = backoff.as_millis(),
                            "no time for backoff, failing activity"
                        );
                        let timeout_error = ActivityError::Timeout(
                            crabdance_proto::shared::TimeoutType::ScheduleToClose,
                        );
                        let _ = task.result_sender.send(Err(timeout_error.clone()));
                        return Err(timeout_error);
                    }

                    info!(
                        activity_type = %task.activity_type,
                        backoff_ms = backoff.as_millis(),
                        "retrying local activity after backoff"
                    );

                    // Wait for backoff
                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                    task.attempt = attempt;
                }
            }
        }
    }

    /// Execute a single attempt of a local activity task
    async fn execute_once(
        &self,
        task: &LocalActivityTask,
        attempt: i32,
    ) -> Result<Vec<u8>, ActivityError> {
        // Get activity from registry
        let activity = self
            .registry
            .get_activity(&task.activity_type)
            .ok_or_else(|| {
                ActivityError::ExecutionFailed(format!(
                    "Activity '{}' not registered",
                    task.activity_type
                ))
            })?;

        // Create activity context for local activity
        let ctx = ActivityContext::new_for_local_activity(
            task.workflow_info.clone(),
            task.activity_type.clone(),
            task.activity_id.clone(),
            attempt,
            task.scheduled_time,
        );

        info!(
            activity_type = %task.activity_type,
            activity_id = %task.activity_id,
            attempt,
            "executing local activity"
        );

        // Calculate remaining time until deadline
        let deadline = task.scheduled_time + task.options.schedule_to_close_timeout;
        let now = SystemTime::now();
        let remaining = deadline
            .duration_since(now)
            .unwrap_or(Duration::from_secs(0));

        // Clone args before moving into spawn
        let args = task.args.clone();
        let ctx_ref = &ctx;
        let future = activity.execute(ctx_ref, args);

        // Execute with timeout
        match tokio::time::timeout(remaining, async move {
            tokio::spawn(future)
                .await
                .map_err(|e| ActivityError::Panic(format!("Activity panicked: {}", e)))?
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(ActivityError::Timeout(
                crabdance_proto::shared::TimeoutType::ScheduleToClose,
            )),
        }
    }
}

/// Check if an activity error should be retried
fn should_retry(error: &ActivityError, retry_policy: &Option<RetryPolicy>, attempt: i32) -> bool {
    let Some(policy) = retry_policy else {
        return false;
    };

    // Check max attempts (0 means unlimited)
    if policy.maximum_attempts > 0 && attempt >= policy.maximum_attempts - 1 {
        warn!(
            attempt = attempt + 1,
            max_attempts = policy.maximum_attempts,
            "max attempts reached, not retrying"
        );
        return false;
    }

    // Check error type - non-retryable errors should not be retried
    match error {
        ActivityError::NonRetryable(_) => {
            debug!("error is explicitly non-retryable");
            false
        }
        ActivityError::Cancelled => {
            debug!("activity was cancelled, not retrying");
            false
        }
        ActivityError::Timeout(_) => {
            debug!("timeout error, not retrying");
            false
        }
        ActivityError::Retryable(_) | ActivityError::RetryableWithDelay(_, _) => {
            debug!("error is retryable");
            true
        }
        ActivityError::ExecutionFailed(_)
        | ActivityError::Panic(_)
        | ActivityError::Application(_) => {
            // TODO: Check against non_retryable_error_types in policy
            debug!("execution error, will retry");
            true
        }
    }
}

/// Calculate backoff duration for a retry attempt
fn calculate_backoff(retry_policy: &Option<RetryPolicy>, attempt: i32) -> Duration {
    let Some(policy) = retry_policy else {
        return Duration::from_secs(1);
    };

    // Calculate exponential backoff: initial_interval * (backoff_coefficient ^ attempt)
    let backoff_millis =
        policy.initial_interval.as_millis() as f64 * policy.backoff_coefficient.powi(attempt);

    let backoff = Duration::from_millis(backoff_millis as u64);

    // Cap at maximum interval
    if backoff > policy.maximum_interval {
        debug!(
            max_interval_ms = policy.maximum_interval.as_millis(),
            "backoff capped at maximum"
        );
        policy.maximum_interval
    } else {
        backoff
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Activity, WorkflowRegistry};
    use crabdance_core::{WorkflowExecution, WorkflowInfo, WorkflowType};
    use crabdance_workflow::LocalActivityOptions;
    use std::future::Future;
    use std::pin::Pin;
    use std::time::{Duration, SystemTime};
    use tokio::sync::oneshot;

    // Test activity that returns success
    #[derive(Clone)]
    struct SuccessActivity;
    impl Activity for SuccessActivity {
        fn execute(
            &self,
            _ctx: &ActivityContext,
            input: Option<Vec<u8>>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
            Box::pin(async move { Ok(input.unwrap_or_else(|| b"success".to_vec())) })
        }
    }

    // Test activity that returns an error
    #[derive(Clone)]
    struct FailActivity;
    impl Activity for FailActivity {
        fn execute(
            &self,
            _ctx: &ActivityContext,
            _input: Option<Vec<u8>>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
            Box::pin(async move { Err(ActivityError::ExecutionFailed("test error".to_string())) })
        }
    }

    fn create_test_workflow_info() -> WorkflowInfo {
        WorkflowInfo {
            workflow_execution: WorkflowExecution {
                workflow_id: "test-workflow".to_string(),
                run_id: "test-run".to_string(),
            },
            workflow_type: WorkflowType {
                name: "TestWorkflow".to_string(),
            },
            task_list: "test-task-list".to_string(),
            start_time: chrono::Utc::now(),
            execution_start_to_close_timeout: Duration::from_secs(3600),
            task_start_to_close_timeout: Duration::from_secs(10),
            attempt: 1,
            continued_execution_run_id: None,
            parent_workflow_execution: None,
            cron_schedule: None,
            memo: None,
            search_attributes: None,
        }
    }

    #[tokio::test]
    async fn test_execute_success() {
        let registry = Arc::new(WorkflowRegistry::new());
        registry.register_activity("SuccessActivity", Box::new(SuccessActivity));

        let queue = LocalActivityQueue::new();
        let executor = LocalActivityExecutor::new(registry, queue.clone());

        let (result_tx, result_rx) = oneshot::channel();
        let task = LocalActivityTask {
            activity_id: "test-1".to_string(),
            activity_type: "SuccessActivity".to_string(),
            args: Some(b"test input".to_vec()),
            options: LocalActivityOptions {
                schedule_to_close_timeout: Duration::from_secs(10),
                retry_policy: None,
            },
            workflow_info: create_test_workflow_info(),
            header: None,
            attempt: 0,
            scheduled_time: SystemTime::now(),
            result_sender: result_tx,
        };

        queue.send(task).expect("Failed to send task");

        // Run executor in background for limited time
        let executor_handle = tokio::spawn(async move {
            // Run executor but limit how long it runs
            tokio::select! {
                _ = executor.run() => {},
                _ = tokio::time::sleep(Duration::from_secs(5)) => {},
            }
        });

        // Wait for result
        let result = result_rx.await.expect("Failed to receive result");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"test input");

        // Close queue to stop executor
        drop(queue);
        let _ = tokio::time::timeout(Duration::from_secs(1), executor_handle).await;
    }

    #[tokio::test]
    async fn test_execute_failure() {
        let registry = Arc::new(WorkflowRegistry::new());
        registry.register_activity("FailActivity", Box::new(FailActivity));

        let queue = LocalActivityQueue::new();
        let executor = LocalActivityExecutor::new(registry, queue.clone());

        let (result_tx, result_rx) = oneshot::channel();
        let task = LocalActivityTask {
            activity_id: "test-2".to_string(),
            activity_type: "FailActivity".to_string(),
            args: None,
            options: LocalActivityOptions {
                schedule_to_close_timeout: Duration::from_secs(10),
                retry_policy: None,
            },
            workflow_info: create_test_workflow_info(),
            header: None,
            attempt: 0,
            scheduled_time: SystemTime::now(),
            result_sender: result_tx,
        };

        queue.send(task).expect("Failed to send task");

        // Run executor in background for limited time
        let executor_handle = tokio::spawn(async move {
            tokio::select! {
                _ = executor.run() => {},
                _ = tokio::time::sleep(Duration::from_secs(5)) => {},
            }
        });

        // Wait for result
        let result = result_rx.await.expect("Failed to receive result");
        assert!(result.is_err());

        // Close queue to stop executor
        drop(queue);
        let _ = tokio::time::timeout(Duration::from_secs(1), executor_handle).await;
    }

    #[tokio::test]
    async fn test_execute_not_registered() {
        let registry = Arc::new(WorkflowRegistry::new());
        // Don't register any activity

        let queue = LocalActivityQueue::new();
        let executor = LocalActivityExecutor::new(registry, queue.clone());

        let (result_tx, result_rx) = oneshot::channel();
        let task = LocalActivityTask {
            activity_id: "test-3".to_string(),
            activity_type: "UnknownActivity".to_string(),
            args: None,
            options: LocalActivityOptions {
                schedule_to_close_timeout: Duration::from_secs(10),
                retry_policy: None,
            },
            workflow_info: create_test_workflow_info(),
            header: None,
            attempt: 0,
            scheduled_time: SystemTime::now(),
            result_sender: result_tx,
        };

        queue.send(task).expect("Failed to send task");

        // Run executor in background for limited time
        let executor_handle = tokio::spawn(async move {
            tokio::select! {
                _ = executor.run() => {},
                _ = tokio::time::sleep(Duration::from_secs(5)) => {},
            }
        });

        // Wait for result
        let result = result_rx.await.expect("Failed to receive result");
        assert!(result.is_err());
        if let Err(ActivityError::ExecutionFailed(msg)) = result {
            assert!(msg.contains("not registered"));
        }

        // Close queue to stop executor
        drop(queue);
        let _ = tokio::time::timeout(Duration::from_secs(1), executor_handle).await;
    }

    // Test activity that fails twice then succeeds
    #[derive(Clone)]
    struct RetryableActivity {
        attempts: Arc<std::sync::atomic::AtomicI32>,
    }
    impl Activity for RetryableActivity {
        fn execute(
            &self,
            ctx: &ActivityContext,
            _input: Option<Vec<u8>>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
            let attempt = ctx.get_info().attempt;
            let attempts = self.attempts.clone();

            Box::pin(async move {
                let prev_attempts = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                if prev_attempts < 2 {
                    Err(ActivityError::Retryable(format!(
                        "Temporary failure on attempt {}",
                        attempt
                    )))
                } else {
                    Ok(b"success after retries".to_vec())
                }
            })
        }
    }

    #[tokio::test]
    async fn test_execute_with_retry() {
        let registry = Arc::new(WorkflowRegistry::new());
        let attempts = Arc::new(std::sync::atomic::AtomicI32::new(0));
        registry.register_activity(
            "RetryableActivity",
            Box::new(RetryableActivity {
                attempts: attempts.clone(),
            }),
        );

        let queue = LocalActivityQueue::new();
        let executor = LocalActivityExecutor::new(registry, queue.clone());

        let (result_tx, result_rx) = oneshot::channel();
        let task = LocalActivityTask {
            activity_id: "test-4".to_string(),
            activity_type: "RetryableActivity".to_string(),
            args: None,
            options: LocalActivityOptions {
                schedule_to_close_timeout: Duration::from_secs(10),
                retry_policy: Some(crabdance_core::RetryPolicy {
                    initial_interval: Duration::from_millis(10),
                    backoff_coefficient: 2.0,
                    maximum_interval: Duration::from_secs(1),
                    maximum_attempts: 5,
                    non_retryable_error_types: vec![],
                    expiration_interval: Duration::from_secs(0),
                }),
            },
            workflow_info: create_test_workflow_info(),
            header: None,
            attempt: 0,
            scheduled_time: SystemTime::now(),
            result_sender: result_tx,
        };

        queue.send(task).expect("Failed to send task");

        // Run executor in background for limited time
        let executor_handle = tokio::spawn(async move {
            tokio::select! {
                _ = executor.run() => {},
                _ = tokio::time::sleep(Duration::from_secs(5)) => {},
            }
        });

        // Wait for result
        let result = result_rx.await.expect("Failed to receive result");
        assert!(result.is_ok(), "Expected success after retries");
        assert_eq!(result.unwrap(), b"success after retries");

        // Verify it actually retried
        let total_attempts = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(total_attempts, 3, "Should have made 3 attempts total");

        // Close queue to stop executor
        drop(queue);
        let _ = tokio::time::timeout(Duration::from_secs(1), executor_handle).await;
    }
}
