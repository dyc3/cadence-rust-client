# Example 03: Activity Advanced

## Overview

This example demonstrates advanced activity patterns including heartbeats, cancellation handling, and deadline management. These patterns are essential for long-running activities that need to report progress, handle interruptions gracefully, and respect time limits.

## Features Demonstrated

- **Activity Heartbeats**: Long-running activities report progress periodically to prevent timeouts and enable resumption
- **Cancellation Handling**: Activities can be cancelled and should cleanup state appropriately
- **Deadline Management**: Activities check remaining time and stop gracefully when approaching deadlines
- **Heartbeat Details**: State is saved in heartbeats to resume from previous attempts after failures

## Key Concepts

### Activity Heartbeats

Heartbeats allow long-running activities to report progress and prevent timeouts:

```rust
// Record progress periodically
for chunk in chunks {
    process_chunk(chunk)?;
    
    // Save progress in heartbeat
    let progress = serde_json::to_vec(&ProcessingProgress { ... })?;
    ctx.record_heartbeat(Some(&progress));
}
```

### Cancellation Handling

Activities should check for cancellation and cleanup:

```rust
if ctx.is_cancelled() {
    // Save current progress
    let details = serde_json::to_vec(&progress)?;
    ctx.record_heartbeat(Some(&details));
    return Err(ActivityError::ExecutionFailed("Cancelled".to_string()));
}
```

### Deadline Management

Activities should respect their time limits:

```rust
if let Some(remaining) = ctx.get_remaining_time() {
    if remaining < Duration::from_secs(5) {
        // Stop gracefully
        break;
    }
}
```

### Heartbeat Recovery

On retry, activities can resume from saved state:

```rust
let mut progress = if ctx.has_heartbeat_details() {
    if let Some(details) = ctx.get_heartbeat_details() {
        serde_json::from_slice::<Progress>(details)?
    } else { Progress::default() }
} else { Progress::default() };

// Resume from saved position
for i in progress.completed_items..total_items {
    // ...
}
```

## Activities

### process_large_file_activity

Processes large files in chunks with heartbeat reporting:

- Records progress every 3 chunks
- Saves state for resumption on retry
- Handles cancellation gracefully
- Monitors deadline

### migrate_data_activity

Migrates data between tables with cancellation support:

- Processes data in batches
- Records heartbeats every 20 records
- Handles worker shutdown signals
- Saves partial progress on cancellation

### deadline_aware_activity

Demonstrates respecting time limits:

- Checks remaining time before each item
- Stops gracefully when deadline approaches
- Reports partial results
- Records periodic heartbeats

## Workflows

### file_processing_workflow

Executes file processing with appropriate timeouts:

```rust
let options = ActivityOptions {
    schedule_to_close_timeout: Duration::from_secs(300),
    heartbeat_timeout: Duration::from_secs(30),
    ..Default::default()
};
```

### data_migration_workflow

Executes data migration with longer timeouts:

```rust
let options = ActivityOptions {
    schedule_to_close_timeout: Duration::from_secs(600),
    heartbeat_timeout: Duration::from_secs(60),
    ..Default::default()
};
```

### deadline_aware_workflow

Demonstrates tight deadline handling:

```rust
let options = ActivityOptions {
    schedule_to_close_timeout: Duration::from_secs(5),
    heartbeat_timeout: Duration::from_secs(2),
    ..Default::default()
};
```

### resilient_processing_workflow

Shows retry policy with heartbeat recovery:

```rust
let retry_policy = RetryPolicy {
    maximum_attempts: 3,
    backoff_coefficient: 2.0,
    ..
};
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p activity_advanced

# Run all tests
cargo test -p activity_advanced
```

## Code Structure

- `src/main.rs` - Example demonstration and tests
- `src/lib.rs` - Library exports
- `src/activities/` - Activity implementations with heartbeats
- `src/workflows/` - Workflow implementations

## When to Use These Patterns

### Use Heartbeats When:
- Activity runs longer than the heartbeat timeout
- You need to track progress
- Activity should be resumable after failure
- You want to prevent timeouts during long operations

### Handle Cancellation When:
- Activity can be interrupted by user request
- Activity needs to cleanup resources
- Partial results should be preserved
- Activity must respect stop signals

### Check Deadlines When:
- Activity has time-sensitive operations
- Partial completion is acceptable
- Resource cleanup is needed before timeout
- You want graceful degradation

## Related Examples

- Previous: [02_activity_basics](../02_activity_basics) - Activity chaining
- Next: [04_workflow_signals](../04_workflow_signals) - Signal handling patterns
