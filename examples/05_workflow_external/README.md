# Example 05: Workflow External

## Overview

This example demonstrates external workflow interactions including signaling and cancellation. These patterns are essential for building complex distributed workflows that coordinate across multiple workflow instances.

## Features Demonstrated

- **External Workflow Signaling**: Sending signals to workflows by ID
- **External Cancellation**: Requesting cancellation of other workflows
- **Workflow Orchestration**: Managing and coordinating multiple workflows
- **Producer-Consumer Pattern**: Workflows producing and consuming data
- **Timer Workflows**: Delayed signaling between workflows

## Key Concepts

### Signaling External Workflows

Send signals to workflows by their ID:

```rust
ctx.signal_external_workflow(
    target_workflow_id,
    None, // run_id (optional)
    "signal_name",
    Some(serde_json::to_vec(&signal_data).unwrap()),
).await?;
```

### Cancelling External Workflows

Request cancellation of other workflows:

```rust
ctx.request_cancel_external_workflow(
    target_workflow_id,
    None, // run_id (optional)
).await?;
```

### Orchestration Pattern

Manage multiple workflows from a parent orchestrator:

```rust
pub async fn orchestrator_workflow(ctx: &mut WorkflowContext) -> Result<()> {
    // Signal multiple workflows to start
    for workflow_id in &workflow_ids {
        ctx.signal_external_workflow(workflow_id, None, "start", ...).await?;
    }
    
    // Monitor and cancel if needed
    for workflow_id in &workflow_ids {
        if should_cancel {
            ctx.request_cancel_external_workflow(workflow_id, None).await?;
        }
    }
}
```

### Producer-Consumer Pattern

Workflows notify each other about data availability:

```rust
// Producer
pub async fn producer_workflow(ctx: &mut WorkflowContext) -> Result<()> {
    // Produce data
    let data = produce_data();
    
    // Notify all consumers
    for consumer_id in &consumers {
        ctx.signal_external_workflow(
            consumer_id,
            None,
            "data_ready",
            Some(serde_json::to_vec(&data).unwrap()),
        ).await?;
    }
}

// Consumer
pub async fn consumer_workflow(ctx: &mut WorkflowContext) -> Result<()> {
    let mut channel = ctx.get_signal_channel("data_ready");
    
    // Wait for data from producers
    let data = channel.recv().await;
    // Process data...
}
```

## Workflows

### orchestrator_workflow

Manages and coordinates multiple child workflows:

**Behavior:**
1. Sends `execute_task` signals to all managed workflows
2. Monitors workflow execution (simulated)
3. Cancells dependent workflows when one fails
4. Tracks completion and cancellation status

**APIs Used:**
- `signal_external_workflow()` - Send signals to managed workflows
- `request_cancel_external_workflow()` - Cancel failing workflows

### producer_workflow

Creates data and notifies consumer workflows:

**Behavior:**
1. Produces data (simulated)
2. Sends `data_ready` signals to all consumers
3. Tracks which consumers were successfully notified

**APIs Used:**
- `signal_external_workflow()` - Notify consumers about data

### consumer_workflow

Processes data produced by other workflows:

**Behavior:**
1. Waits for `data_ready` signals from producers
2. Processes received data
3. Handles timeout if not all data received

**APIs Used:**
- `get_signal_channel()` - Create channel for data_ready signals

### timer_workflow

Delays and signals another workflow:

**Behavior:**
1. Sleeps for specified duration
2. Sends `timer_expired` signal to target workflow
3. Reports success/failure of notification

**APIs Used:**
- `sleep()` - Workflow-aware timer
- `signal_external_workflow()` - Notify target workflow

## Activities

### log_activity_execution

Logs workflow and activity execution details:

```rust
let log_input = LogActivityInput {
    workflow_id: "orch_123".to_string(),
    activity_name: "orchestrate".to_string(),
    message: "Signaled workflow wf_001".to_string(),
    level: LogLevel::Info,
};
```

### send_external_notification

Sends external notifications (webhooks, etc.):

```rust
let notif_input = NotifyInput {
    target_workflow_id: "wf_001".to_string(),
    notification_type: "cancellation".to_string(),
    message: "Cancelled due to dependency failure".to_string(),
};
```

## Signal Types

### ExecuteTaskSignal
```rust
pub struct ExecuteTaskSignal {
    pub task_id: String,
    pub task_type: String,
    pub parameters: serde_json::Value,
}
```

### DataReadySignal
```rust
pub struct DataReadySignal {
    pub data_id: String,
    pub data_location: String,
    pub producer_workflow_id: String,
}
```

### TimerExpiredSignal
```rust
pub struct TimerExpiredSignal {
    pub timer_id: String,
    pub target_workflow_id: String,
}
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p workflow_external

# Run all tests
cargo test -p workflow_external
```

## Use Cases

### Use External Signaling When:
- You need to coordinate multiple workflows
- Workflows depend on each other's progress
- One workflow needs to trigger another
- Implementing pub/sub patterns

### Use External Cancellation When:
- A workflow failure should stop related workflows
- Implementing saga patterns with compensation
- Parent workflow needs to cleanup children
- Resource constraints require stopping workflows

## Common Patterns

### Saga Pattern
```rust
// Start saga steps
for step in &saga_steps {
    ctx.signal_external_workflow(&step.id, None, "execute", ...).await?;
}

// If any step fails, cancel all
if failure_detected {
    for step in &saga_steps {
        ctx.request_cancel_external_workflow(&step.id, None).await?;
    }
}
```

### Fan-Out / Fan-In
```rust
// Fan-out: Signal multiple workflows
for worker_id in &workers {
    ctx.signal_external_workflow(worker_id, None, "process_chunk", ...).await?;
}

// Fan-in: Wait for all to complete (via signals)
for _ in &workers {
    let completion = completion_channel.recv().await;
    results.push(completion);
}
```

## Related Examples

- Previous: [04_workflow_signals](../04_workflow_signals) - Signal handling patterns
- Next: [06_child_workflows](../06_child_workflows) - Child workflow patterns
