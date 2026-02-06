# Example 17: Worker Configuration

## Overview

This example demonstrates worker setup and tuning including configuration options, registration patterns, and performance tuning.

## Features Demonstrated

- **WorkerOptions**: Configuring worker behavior
- **Registry**: Registering workflows and activities
- **Rate Limiting**: Activities per second controls
- **Concurrency**: Max concurrent execution settings
- **Sticky Execution**: Workflow caching for performance

## Key Concepts

### Worker Options

```rust
let options = WorkerOptions {
    max_concurrent_activity_execution_size: 1000,
    max_concurrent_decision_task_execution_size: 1000,
    worker_activities_per_second: 100.0,
    worker_decision_tasks_per_second: 10.0,
    max_concurrent_decision_task_pollers: 2,
    disable_sticky_execution: false,
    sticky_schedule_to_start_timeout: Duration::from_secs(5),
    worker_stop_timeout: Duration::from_secs(10),
    identity: "my-worker".to_string(),
    ..Default::default()
};
```

### Worker Presets

Three preset configurations are provided:

1. **High Performance** - Production with abundant resources
2. **Development** - Debugging with sticky execution disabled
3. **Resource-Limited** - Memory-constrained environments

### Registration

```rust
let registry = WorkflowRegistry::new();

// Register activities
registry.register_activity("compute", compute_activity);
registry.register_activity("process_data", process_data_activity);

// Register workflows
registry.register_workflow("data_processing", data_processing_workflow);
```

### Worker Setup

```rust
let worker = CadenceWorker::new(
    "my-domain",
    "my-task-list",
    options,
    Arc::new(registry),
);

// Start in non-blocking mode
worker.start()?;

// Or run in blocking mode
worker.run()?;

// Graceful shutdown
worker.stop();
```

### Configuration Guidelines

| Environment | Activities | Pollers | Sticky | Use Case |
|-------------|-----------|---------|--------|----------|
| Production | 1000+ | 4 | Enabled | High throughput |
| Development | 100 | 1 | Disabled | Debugging |
| Limited | 50 | 1 | Enabled | Low memory |

## Running the Example

```bash
cargo run -p worker_configuration
```

## Running Tests

```bash
cargo test -p worker_configuration
```

## Code Structure

- `src/main.rs` - Example entry point and demonstration
- `src/lib.rs` - Library exports
- `src/activities/` - Activity implementations
- `src/workflows/` - Workflow implementations
- `src/worker_setup/` - Worker configuration helpers

## Related Examples

- Previous: [16_domain_management](../16_domain_management) - Domain lifecycle
- Next: [18_local_activities](../18_local_activities) - Local activity execution
