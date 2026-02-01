# Example 01: Hello Workflow

## Overview

This is the foundational example demonstrating the core concepts of the Cadence Rust client: workflows, activities, serialization, and basic signal handling.

## Features Demonstrated

- **Workflow Definition**: Creating a durable, fault-tolerant workflow function
- **Activity Definition**: Creating retryable business logic units
- **Serialization**: Using serde for input/output marshaling
- **Heartbeats**: Reporting progress from activities
- **Signals**: Handling external events in workflows

## Key Concepts

### Workflow

A workflow is a durable, fault-tolerant function that orchestrates activities and other workflows. Workflows maintain state and can survive process restarts.

```rust
pub async fn hello_workflow(
    ctx: &mut WorkflowContext,
    input: HelloInput,
) -> Result<HelloOutput, WorkflowError>
```

### Activity

An activity is a business logic unit that can fail and be retried. Activities are the building blocks of workflows.

```rust
pub async fn format_greeting_activity(
    ctx: &ActivityContext,
    input: HelloInput,
) -> Result<HelloOutput, ActivityError>
```

### Heartbeat

Long-running activities should report progress via heartbeats. If an activity times out, Cadence can resume from the last heartbeat.

```rust
ctx.record_heartbeat(None);
```

### Signal

Workflows can receive external signals to handle events from outside the workflow execution.

```rust
let mut signal_channel = ctx.get_signal_channel("greeting_style");
let signal_data = signal_channel.recv().await;
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p hello_workflow

# Run all tests
cargo test -p hello_workflow
```

## Code Structure

- `src/main.rs` - Example entry point and demonstration
- `src/lib.rs` - Library exports
- `src/activities/` - Activity implementations
- `src/workflows/` - Workflow implementations

## Test Examples

The tests demonstrate how to use `TestWorkflowEnvironment` for unit testing workflows without needing a real Cadence server:

```rust
let mut env = TestWorkflowEnvironment::new();
env.register_activity("format_greeting", format_greeting_activity);
env.register_workflow("hello_workflow", hello_workflow);

let result = env.execute_workflow("hello_workflow", input).await?;
```

## Related Examples

- Next: [02_activity_basics](../02_activity_basics) - Activity chaining and composition
- See also: [04_workflow_signals](../04_workflow_signals) - Advanced signal handling
