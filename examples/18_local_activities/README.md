# Example 18: Local Activities

## Overview

This example demonstrates local activity execution patterns - activities that run synchronously in the workflow thread without task queue overhead.

## Features Demonstrated

- **LocalActivityOptions**: Configuring local activity execution
- **execute_local_activity**: Running activities synchronously
- **Retry Policies**: Configuring retry behavior for local activities
- **Use Cases**: Short, fast activities that don't need task queue

## Key Concepts

### Local vs Regular Activities

| Feature | Local Activity | Regular Activity |
|---------|---------------|------------------|
| Execution | Synchronous | Asynchronous |
| Task Queue | No | Yes |
| Heartbeat | No | Yes |
| Worker Load Balancing | No | Yes |
| Overhead | Low | Higher |
| Max Duration | ~5 seconds | Unlimited |

### When to Use Local Activities

**Use Local Activities For:**
- Short execution time (< 5 seconds)
- No need for heartbeats
- Low failure rate
- Don't need distribution across workers
- Fast CPU-bound operations
- Simple data transformations

**Use Regular Activities For:**
- Long-running operations
- Need heartbeats for progress
- May fail and need retries
- Should be distributed across workers
- External API calls
- Database operations
- Network I/O

### Local Activity Execution

```rust
let options = LocalActivityOptions {
    schedule_to_close_timeout: Duration::from_secs(5),
    retry_policy: Some(RetryPolicy {
        initial_interval: Duration::from_millis(100),
        backoff_coefficient: 1.5,
        maximum_attempts: 3,
        ..Default::default()
    }),
};

let result = ctx
    .execute_local_activity("validate_data", args, options)
    .await?;
```

### Common Local Activity Patterns

1. **Data Validation** - Fast validation of input data
2. **Data Transformation** - Simple data format conversions
3. **Data Enrichment** - Adding computed fields
4. **Business Logic Decisions** - Quick rule-based decisions

### Performance Benefits

- No task queue latency
- No serialization overhead for task transfer
- No network round-trip
- Immediate execution
- Lower memory footprint

## Running the Example

```bash
cargo run -p local_activities
```

## Running Tests

```bash
cargo test -p local_activities
```

## Code Structure

- `src/main.rs` - Example entry point and demonstration
- `src/lib.rs` - Library exports
- `src/activities/` - Local activity implementations
- `src/workflows/` - Workflow implementations using local activities

## Related Examples

- Previous: [17_worker_configuration](../17_worker_configuration) - Worker setup
- See also: [02_activity_basics](../02_activity_basics) - Activity fundamentals
