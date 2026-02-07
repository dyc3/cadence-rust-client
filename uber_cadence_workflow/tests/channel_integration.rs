use uber_cadence_core::{WorkflowExecution, WorkflowInfo, WorkflowType};
use uber_cadence_workflow::context::WorkflowContext;
use uber_cadence_workflow::dispatcher::{WorkflowDispatcher, WorkflowTask};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_channel_with_dispatcher() {
    // Create a workflow context
    let workflow_info = create_test_workflow_info();
    let ctx = WorkflowContext::new(workflow_info);

    // Create dispatcher
    let dispatcher = Arc::new(Mutex::new(WorkflowDispatcher::new()));
    ctx.set_dispatcher(dispatcher.clone());

    // Create a channel
    let (tx, rx) = ctx.new_channel::<i32>(2);

    // Spawn sender task
    let send_task = WorkflowTask::new(0, "sender".to_string(), async move {
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        tx.send(3).await.unwrap();
        drop(tx);
    });

    // Spawn receiver task
    let recv_task = WorkflowTask::new(1, "receiver".to_string(), async move {
        let mut results = Vec::new();
        while let Some(val) = rx.recv().await {
            results.push(val);
        }
        results
    });

    // Add tasks to dispatcher
    {
        let mut disp = dispatcher.lock().unwrap();
        disp.spawn_task(send_task);
        disp.spawn_task(recv_task);
    }

    // Execute until all blocked
    let all_done = {
        let mut disp = dispatcher.lock().unwrap();
        disp.execute_until_all_blocked().unwrap()
    };

    assert!(all_done, "All tasks should complete");

    // Get receiver result
    let disp = dispatcher.lock().unwrap();
    let result = disp.get_task_result(1).unwrap();
    let values = result.downcast::<Vec<i32>>().unwrap();

    assert_eq!(*values, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_spawn_with_channels() {
    // Create a workflow context
    let workflow_info = create_test_workflow_info();
    let ctx = WorkflowContext::new(workflow_info);

    // Create dispatcher
    let dispatcher = Arc::new(Mutex::new(WorkflowDispatcher::new()));
    ctx.set_dispatcher(dispatcher.clone());

    // Create main workflow task
    let ctx_clone = ctx.clone();
    let workflow_task = WorkflowTask::new(0, "workflow".to_string(), async move {
        // Create channel for collecting results
        let (tx, rx) = ctx_clone.new_channel::<u32>(5);

        // Spawn 5 tasks that each send a value
        for i in 1..=5 {
            let tx_clone = tx.clone();
            ctx_clone.spawn(async move {
                tx_clone.send(i * 10).await.unwrap();
            });
        }

        // Drop original sender
        drop(tx);

        // Collect all results
        let mut results = Vec::new();
        while let Some(val) = rx.recv().await {
            results.push(val);
        }

        results.sort(); // Sort for deterministic comparison
        results
    });

    // Add workflow task to dispatcher
    {
        let mut disp = dispatcher.lock().unwrap();
        disp.spawn_task(workflow_task);
    }

    // Execute until all blocked
    let all_done = {
        let mut disp = dispatcher.lock().unwrap();
        disp.execute_until_all_blocked().unwrap()
    };

    assert!(all_done, "All tasks should complete");

    // Get workflow result
    let disp = dispatcher.lock().unwrap();
    let result = disp.get_task_result(0).unwrap();
    let values = result.downcast::<Vec<u32>>().unwrap();

    assert_eq!(*values, vec![10, 20, 30, 40, 50]);
}

fn create_test_workflow_info() -> WorkflowInfo {
    WorkflowInfo {
        workflow_execution: WorkflowExecution {
            workflow_id: "test-workflow-123".to_string(),
            run_id: "test-run-456".to_string(),
        },
        workflow_type: WorkflowType {
            name: "TestWorkflow".to_string(),
        },
        task_list: "test-task-list".to_string(),
        start_time: chrono::Utc::now(),
        execution_start_to_close_timeout: std::time::Duration::from_secs(3600),
        task_start_to_close_timeout: std::time::Duration::from_secs(10),
        attempt: 1,
        continued_execution_run_id: None,
        parent_workflow_execution: None,
        cron_schedule: None,
        memo: None,
        search_attributes: None,
    }
}
