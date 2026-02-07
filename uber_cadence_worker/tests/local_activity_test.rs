//! Integration test for local activity execution
//!
//! This test verifies that the local activity components work together:
//! 1. Queue can send/receive tasks
//! 2. Executor can process tasks from the queue
//! 3. Results are returned properly

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::oneshot;
use uber_cadence_activity::ActivityContext;
use uber_cadence_core::{WorkflowExecution, WorkflowInfo, WorkflowType};
use uber_cadence_worker::executor::local_activity::LocalActivityExecutor;
use uber_cadence_worker::local_activity_queue::{LocalActivityQueue, LocalActivityTask};
use uber_cadence_worker::registry::{Activity, Registry};
use uber_cadence_worker::{ActivityError, WorkflowRegistry};
use uber_cadence_workflow::LocalActivityOptions;

// Test activity that echoes input
#[derive(Clone)]
struct EchoActivity;
impl Activity for EchoActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move { Ok(input.unwrap_or_default()) })
    }
}

#[tokio::test]
async fn test_local_activity_queue_and_executor() {
    // Setup registry with activity
    let registry = WorkflowRegistry::new();
    registry.register_activity("echo", Box::new(EchoActivity));
    let registry = Arc::new(registry);

    // Create queue and executor
    let queue = LocalActivityQueue::new();
    let executor = LocalActivityExecutor::new(registry, queue.clone());

    // Start executor in background
    let executor_handle = tokio::spawn(async move {
        executor.run().await;
    });

    // Give executor time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Create a task
    let (tx, rx) = oneshot::channel();

    let workflow_info = WorkflowInfo {
        workflow_execution: WorkflowExecution {
            workflow_id: "test-wf".to_string(),
            run_id: "test-run".to_string(),
        },
        workflow_type: WorkflowType {
            name: "TestWorkflow".to_string(),
        },
        task_list: "test-list".to_string(),
        start_time: chrono::Utc::now(),
        execution_start_to_close_timeout: Duration::from_secs(10),
        task_start_to_close_timeout: Duration::from_secs(5),
        attempt: 1,
        continued_execution_run_id: None,
        parent_workflow_execution: None,
        cron_schedule: None,
        memo: None,
        search_attributes: None,
    };

    let task = LocalActivityTask {
        activity_id: "act-1".to_string(),
        activity_type: "echo".to_string(),
        args: Some(b"test data".to_vec()),
        options: LocalActivityOptions {
            schedule_to_close_timeout: Duration::from_secs(5),
            retry_policy: None,
        },
        workflow_info,
        header: None,
        attempt: 0,
        scheduled_time: SystemTime::now(),
        result_sender: tx,
    };

    // Submit task
    queue.send(task).expect("Failed to send task");

    // Wait for result with timeout
    let result = tokio::time::timeout(Duration::from_secs(2), rx)
        .await
        .expect("Timeout waiting for result")
        .expect("Failed to receive result");

    // Verify result
    assert!(result.is_ok(), "Activity should succeed");
    let output = result.unwrap();
    assert_eq!(output, b"test data", "Output should match input");

    // Clean up
    executor_handle.abort();

    println!("✅ Local activity queue and executor test passed!");
}

#[tokio::test]
async fn test_local_activity_not_found() {
    // Setup empty registry (no activities registered)
    let registry = Arc::new(WorkflowRegistry::new());

    // Create queue and executor
    let queue = LocalActivityQueue::new();
    let executor = LocalActivityExecutor::new(registry, queue.clone());

    // Start executor
    let executor_handle = tokio::spawn(async move {
        executor.run().await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Create task for non-existent activity
    let (tx, rx) = oneshot::channel();

    let workflow_info = WorkflowInfo {
        workflow_execution: WorkflowExecution {
            workflow_id: "test-wf".to_string(),
            run_id: "test-run".to_string(),
        },
        workflow_type: WorkflowType {
            name: "TestWorkflow".to_string(),
        },
        task_list: "test-list".to_string(),
        start_time: chrono::Utc::now(),
        execution_start_to_close_timeout: Duration::from_secs(10),
        task_start_to_close_timeout: Duration::from_secs(5),
        attempt: 1,
        continued_execution_run_id: None,
        parent_workflow_execution: None,
        cron_schedule: None,
        memo: None,
        search_attributes: None,
    };

    let task = LocalActivityTask {
        activity_id: "act-2".to_string(),
        activity_type: "non_existent".to_string(),
        args: None,
        options: LocalActivityOptions {
            schedule_to_close_timeout: Duration::from_secs(5),
            retry_policy: None,
        },
        workflow_info,
        header: None,
        attempt: 0,
        scheduled_time: SystemTime::now(),
        result_sender: tx,
    };

    // Submit task
    queue.send(task).expect("Failed to send task");

    // Wait for result
    let result = tokio::time::timeout(Duration::from_secs(2), rx)
        .await
        .expect("Timeout waiting for result")
        .expect("Failed to receive result");

    // Verify error
    assert!(
        result.is_err(),
        "Should return error for non-existent activity"
    );
    match result.unwrap_err() {
        ActivityError::ExecutionFailed(msg) => {
            assert!(
                msg.contains("not registered"),
                "Error message should mention 'not registered'"
            );
            assert!(
                msg.contains("non_existent"),
                "Error message should mention activity name"
            );
        }
        _ => panic!("Expected ExecutionFailed error"),
    }

    // Clean up
    executor_handle.abort();

    println!("✅ Local activity not found test passed!");
}

#[tokio::test]
async fn test_local_activity_execution_count() {
    // This test verifies activities execute the expected number of times
    use std::sync::atomic::{AtomicUsize, Ordering};

    let counter = Arc::new(AtomicUsize::new(0));

    // Activity that increments counter
    #[derive(Clone)]
    struct CountingActivity {
        counter: Arc<AtomicUsize>,
    }

    impl Activity for CountingActivity {
        fn execute(
            &self,
            _ctx: &ActivityContext,
            input: Option<Vec<u8>>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
            let counter = self.counter.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(input.unwrap_or_default())
            })
        }
    }

    let registry = WorkflowRegistry::new();
    registry.register_activity(
        "counter",
        Box::new(CountingActivity {
            counter: counter.clone(),
        }),
    );
    let registry = Arc::new(registry);

    let queue = LocalActivityQueue::new();
    let executor = LocalActivityExecutor::new(registry, queue.clone());

    let executor_handle = tokio::spawn(async move {
        executor.run().await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Submit 3 tasks
    for i in 0..3 {
        let (tx, rx) = oneshot::channel();

        let workflow_info = WorkflowInfo {
            workflow_execution: WorkflowExecution {
                workflow_id: format!("test-wf-{}", i),
                run_id: format!("test-run-{}", i),
            },
            workflow_type: WorkflowType {
                name: "TestWorkflow".to_string(),
            },
            task_list: "test-list".to_string(),
            start_time: chrono::Utc::now(),
            execution_start_to_close_timeout: Duration::from_secs(10),
            task_start_to_close_timeout: Duration::from_secs(5),
            attempt: 1,
            continued_execution_run_id: None,
            parent_workflow_execution: None,
            cron_schedule: None,
            memo: None,
            search_attributes: None,
        };

        let task = LocalActivityTask {
            activity_id: format!("act-{}", i),
            activity_type: "counter".to_string(),
            args: Some(format!("data-{}", i).into_bytes()),
            options: LocalActivityOptions {
                schedule_to_close_timeout: Duration::from_secs(5),
                retry_policy: None,
            },
            workflow_info,
            header: None,
            attempt: 0,
            scheduled_time: SystemTime::now(),
            result_sender: tx,
        };

        queue.send(task).expect("Failed to send task");

        // Wait for result
        let result = tokio::time::timeout(Duration::from_secs(2), rx)
            .await
            .expect("Timeout waiting for result")
            .expect("Failed to receive result");

        assert!(result.is_ok(), "Task {} should succeed", i);
    }

    // Verify all 3 activities executed
    let final_count = counter.load(Ordering::SeqCst);
    assert_eq!(final_count, 3, "Should have executed 3 activities");

    // Clean up
    executor_handle.abort();

    println!("✅ Local activity execution count test passed!");
}
