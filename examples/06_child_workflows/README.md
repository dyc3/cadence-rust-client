# Example 06: Child Workflows

## Overview

This example demonstrates child workflow patterns. Child workflows are workflows started by other workflows (parent workflows), enabling complex orchestration patterns like fan-out, fan-in, and hierarchical workflows.

## Features Demonstrated

- **Child Workflow Execution**: Starting and managing child workflows
- **Fan-Out Pattern**: Running multiple child workflows in parallel
- **Fan-In Pattern**: Collecting and aggregating results from children
- **Error Handling**: Handling child workflow failures gracefully
- **Parent-Child Relationships**: Managing child lifecycle

## Key Concepts

### Executing Child Workflows

Start a child workflow from a parent:

```rust
let child_options = ChildWorkflowOptions {
    workflow_id: Some("child_001".to_string()),
    task_list: "child-tasks".to_string(),
    execution_start_to_close_timeout: Duration::from_secs(60),
    ..Default::default()
};

let result = ctx
    .execute_child_workflow(
        "child_workflow_type",
        Some(serde_json::to_vec(&child_input).unwrap()),
        child_options,
    )
    .await?;
```

### Fan-Out Pattern

Run multiple child workflows in parallel:

```rust
let mut child_handles = vec![];

// Start all children
for item in &items {
    let handle = ctx.execute_child_workflow(
        "process_item",
        Some(serde_json::to_vec(item).unwrap()),
        options,
    );
    child_handles.push(handle);
}

// Wait for all to complete
for handle in child_handles {
    let result = handle.await?;
    // Process result...
}
```

### Fan-In Pattern

Aggregate results from multiple children:

```rust
// Collect results from children
let mut child_results = vec![];
for handle in child_handles {
    let result = handle.await?;
    child_results.push(result);
}

// Aggregate using an activity
let aggregate_input = AggregateInput { results: child_results };
let final_result = ctx
    .execute_activity("aggregate", ...)
    .await?;
```

### Error Handling

Handle child workflow failures:

```rust
match ctx.execute_child_workflow("child", input, options).await {
    Ok(result) => {
        // Success - process result
    }
    Err(e) => {
        // Handle failure - log, compensate, retry, etc.
        error!("Child failed: {}", e);
        failed_children.push(child_id);
    }
}
```

## Workflows

### parent_workflow

Orchestrates multiple child workflows:

**Behavior:**
1. Takes a job with multiple data chunks
2. Starts a child workflow for each chunk
3. Collects results from all children
4. Tracks successful and failed children
5. Returns aggregated results

**APIs Used:**
- `execute_child_workflow()` - Start child workflows

### child_processor_workflow

Child workflow that processes a chunk of data:

**Behavior:**
1. Receives chunk of data from parent
2. Processes using activities
3. Returns results to parent
4. Handles cancellation from parent

**APIs Used:**
- `execute_activity()` - Process chunk
- `is_cancelled()` - Check for parent cancellation

### fan_out_workflow

Demonstrates the fan-out pattern:

**Behavior:**
1. Takes a list of items to process
2. Starts a child workflow for each item (in parallel)
3. Waits for all children to complete
4. Reports success/failure statistics

**Pattern:** Fan-Out (distribute work across children)

### fan_in_workflow

Demonstrates the fan-in pattern:

**Behavior:**
1. Takes outputs from multiple children
2. Aggregates results using an activity
3. Returns combined result

**Pattern:** Fan-In (aggregate results from children)

### process_single_item_workflow

Simple child workflow for fan-out pattern:

**Behavior:**
1. Processes a single item
2. Validates and transforms data
3. Returns result

## Activities

### process_chunk_activity

Processes a chunk of data:

```rust
let input = ProcessChunkInput {
    chunk_id: 0,
    data: vec!["item1", "item2", "item3"],
    processing_options: ProcessingOptions { ... },
};
```

### validate_data_activity

Validates data items:

```rust
let input = ValidateInput {
    data_id: "item_001".to_string(),
    data: serde_json::json!(data),
    validation_rules: vec!["required".to_string()],
};
```

### aggregate_results_activity

Aggregates results from multiple children:

```rust
let input = AggregateInput {
    results: vec![
        PartialResult { child_id: "c1", data: vec![...], count: 10 },
        PartialResult { child_id: "c2", data: vec![...], count: 15 },
    ],
};
```

## Child Workflow Options

```rust
ChildWorkflowOptions {
    workflow_id: Some("unique_id".to_string()),
    task_list: "child-task-list".to_string(),
    execution_start_to_close_timeout: Duration::from_secs(300),
    task_start_to_close_timeout: Duration::from_secs(60),
    retry_policy: Some(RetryPolicy { ... }),
    ..Default::default()
}
```

## Common Patterns

### Map-Reduce
```rust
// Map phase: Start children for each input
let handles: Vec<_> = inputs
    .into_iter()
    .map(|input| ctx.execute_child_workflow("map", input, options))
    .collect();

// Collect mapped results
let mapped: Vec<_> = handles
    .into_iter()
    .map(|h| h.await)
    .collect();

// Reduce phase: Aggregate results
let result = ctx.execute_activity("reduce", mapped, options).await?;
```

### Scatter-Gather
```rust
// Scatter: Send work to multiple children
for shard in shards {
    ctx.execute_child_workflow("process_shard", shard, options);
}

// Gather: Collect results
let mut results = vec![];
for _ in &shards {
    let result = completion_channel.recv().await;
    results.push(result);
}
```

### Hierarchical Processing
```rust
// Level 1: Parent
let regions = vec!["US", "EU", "APAC"];
for region in regions {
    ctx.execute_child_workflow("region_processor", region, options);
}

// Level 2: Region
let stores = get_stores_in_region(region);
for store in stores {
    ctx.execute_child_workflow("store_processor", store, options);
}

// Level 3: Store
process_store_inventory(store);
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p child_workflows

# Run all tests
cargo test -p child_workflows
```

## When to Use Child Workflows

### Use Child Workflows When:
- You need to parallelize work across multiple units
- Different parts of a workflow need different timeouts/retry policies
- You want independent failure domains
- You need hierarchical workflow organization
- You want to isolate different concerns

### Use Activities When:
- Single unit of work that doesn't need its own workflow state
- Simple operation without complex lifecycle
- No need for independent timeouts or retries

## Benefits of Child Workflows

1. **Isolation**: Children have independent state and history
2. **Scalability**: Parallel execution across many children
3. **Failure Domains**: Child failures don't affect parent
4. **Timeouts**: Each child can have its own timeout settings
5. **Retries**: Independent retry policies per child
6. **Cancellation**: Children can be cancelled independently

## Related Examples

- Previous: [05_workflow_external](../05_workflow_external) - External signals and cancellation
- This is the last example in the basic series
