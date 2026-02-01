# Example 15: Client Operations

## Overview

This example demonstrates the Client API for workflow management operations including starting workflows, sending signals, querying state, and managing workflow lifecycle.

## Features Demonstrated

- **StartWorkflowOptions**: Configuring workflow execution parameters
- **Signal Patterns**: Sending signals to running workflows
- **Query Patterns**: Querying workflow state
- **Lifecycle Management**: Cancel, terminate, and reset workflows
- **Workflow Discovery**: List, scan, and count workflows

## Key Concepts

### Starting Workflows

```rust
let options = StartWorkflowOptions {
    id: "workflow-001".to_string(),
    task_list: "example-task-list".to_string(),
    execution_start_to_close_timeout: Some(Duration::from_secs(60)),
    workflow_id_reuse_policy: WorkflowIdReusePolicy::AllowDuplicateFailedOnly,
    retry_policy: Some(RetryPolicy::default()),
    ..Default::default()
};

let execution = client.start_workflow(options, "workflow_type", args).await?;
```

### Workflow ID Reuse Policies

- `AllowDuplicateFailedOnly`: Allow if last execution failed/terminated
- `AllowDuplicate`: Allow if not currently running
- `RejectDuplicate`: Never allow duplicate IDs
- `TerminateIfRunning`: Terminate existing and start new

### Signaling Workflows

```rust
// Send signal to running workflow
client.signal_workflow(
    workflow_id,
    run_id,
    "signal_name",
    signal_data
).await?;
```

### Querying Workflows

```rust
// Query workflow state
let result = client.query_workflow(
    workflow_id,
    run_id,
    "__stack_trace",
    None
).await?;
```

### Workflow Lifecycle

```rust
// Cancel (graceful)
client.cancel_workflow(workflow_id, run_id, "reason").await?;

// Terminate (immediate)
client.terminate_workflow(workflow_id, run_id, "reason", details).await?;

// Reset
let reset_response = client.reset_workflow(reset_request).await?;
```

## Running the Example

```bash
cargo run -p client_operations
```

## Running Tests

```bash
cargo test -p client_operations
```

## Code Structure

- `src/main.rs` - Example entry point and demonstration
- `src/lib.rs` - Library exports
- `src/workflows/` - Workflow implementations for client operations

## Related Examples

- Previous: [14_workflow_testing](../14_workflow_testing) - Testing patterns
- Next: [16_domain_management](../16_domain_management) - Domain lifecycle
