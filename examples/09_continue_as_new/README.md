# Example 09: Continue As New

## Overview

This example demonstrates the continue-as-new pattern for long-running workflows. Continue-as-new allows workflows to run indefinitely by periodically completing and immediately starting a new workflow run with the current state.

## Features Demonstrated

- **Continue As New**: Completing a workflow and immediately starting a new one
- **State Transfer**: Passing state from the old workflow to the new one
- **Iteration Limits**: Controlling how much work is done per workflow run
- **Pagination**: Processing large datasets in chunks across workflow runs

## Key Concepts

### Continue As New

Continue-as-new completes the current workflow and starts a new one with the same workflow ID:

```rust
if iteration >= MAX_ITERATIONS && has_more_data {
    // Continue as new to prevent history from growing too large
    ctx.continue_as_new(new_input).await?;
}
```

### State Transfer

When continuing as new, pass the current state to the new workflow:

```rust
let checkpoint = CheckpointData {
    iteration: current_iteration,
    total_processed: total,
    last_cursor: cursor,
    metadata: current_metadata,
};
```

### Pagination

Process large datasets in manageable chunks:

```rust
while pages_processed < max_pages && has_more_data {
    let batch = fetch_data(cursor).await?;
    process_batch(batch).await?;
    cursor = batch.next_cursor;
}
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p continue_as_new

# Run all tests
cargo test -p continue_as_new
```

## Code Structure

- `src/main.rs` - Example entry point and tests
- `src/lib.rs` - Library exports
- `src/activities/` - Activity implementations
- `src/workflows/` - Workflow implementations

## Workflows

### batched_processor_workflow

Processes data in batches with periodic continue-as-new to manage workflow history size.

### paginated_processor_workflow

Demonstrates pagination pattern with limited pages per workflow run.

## Activities

### process_batch_activity

Processes a batch of items with progress tracking.

### fetch_data_activity

Fetches data from external source with cursor-based pagination.

### save_checkpoint_activity

Saves checkpoint data for state transfer between workflow runs.

## Why Continue As New?

Workflows accumulate event history as they execute. For long-running workflows:

1. **History Size Limits**: Cadence has limits on workflow history size
2. **Performance**: Large histories can slow down workflow replay
3. **State Management**: Periodic checkpoints provide recovery points

Continue-as-new solves these issues by:
- Starting fresh with empty history
- Preserving state through input parameters
- Maintaining the same workflow identity

## Related Examples

- Previous: [08_versioning](../08_versioning) - Workflow versioning and side effects
- Next: [10_error_handling](../10_error_handling) - Error types and handling
