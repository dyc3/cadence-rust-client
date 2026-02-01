# Example 07: Time and Sleep

## Overview

This example demonstrates timers and time manipulation in Cadence workflows. Timers are durable time-based events that survive workflow restarts and are a fundamental building block for time-based operations.

## Features Demonstrated

- **Workflow Sleep**: Durable timer operations that survive restarts
- **Deadline Management**: Waiting for specific deadlines
- **Timer Cancellation**: Cancelling timers via signals
- **Exponential Backoff**: Retry logic with increasing delays
- **Reminder Scheduling**: Time-based reminder delivery

## Key Concepts

### Timer

A timer is a durable time-based event that survives workflow restarts:

```rust
// Sleep for 10 seconds (durable timer)
ctx.sleep(Duration::from_secs(10)).await;
```

### Deadline Management

Calculate and wait for specific deadlines:

```rust
let duration_until = deadline - Utc::now();
let wait_secs = duration_until.num_seconds() as u64;
ctx.sleep(Duration::from_secs(wait_secs)).await;
```

### Timer Cancellation

Timers can be cancelled using signals:

```rust
tokio::select! {
    _ = ctx.sleep(Duration::from_secs(60)) => {
        // Timer completed
    }
    _ = cancel_signal.recv() => {
        // Timer cancelled
    }
}
```

### Exponential Backoff

Implement retry logic with increasing delays:

```rust
let backoff_secs = std::cmp::min(2u64.pow(attempt - 1), 60);
ctx.sleep(Duration::from_secs(backoff_secs)).await;
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p time_and_sleep

# Run all tests
cargo test -p time_and_sleep
```

## Code Structure

- `src/main.rs` - Example entry point and tests
- `src/lib.rs` - Library exports
- `src/activities/` - Activity implementations
- `src/workflows/` - Workflow implementations

## Workflows

### sleep_demo_workflow

Basic sleep/timer demonstration.

### deadline_wait_workflow

Waits for a specific deadline using durable timers.

### cancellable_timer_workflow

Demonstrates timer cancellation via signals.

### retry_with_backoff_workflow

Implements exponential backoff retry logic.

### reminder_workflow

Schedules and manages time-based reminders.

## Activities

### check_deadline_activity

Checks if a deadline has been reached.

### send_reminder_activity

Sends reminder notifications.

### time_bound_operation_activity

Performs operations with time constraints.

## Related Examples

- Previous: [06_query_handling](../06_query_handling) - Query handling patterns
- Next: [08_versioning](../08_versioning) - Workflow versioning and side effects
