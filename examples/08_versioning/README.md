# Example 08: Versioning

## Overview

This example demonstrates workflow versioning and side effects in Cadence. Versioning allows you to evolve workflow logic over time while maintaining backwards compatibility with existing workflow executions.

## Features Demonstrated

- **Workflow Versioning**: Managing changes to workflow logic over time
- **Side Effects**: Handling non-deterministic operations
- **Version-Specific Behavior**: Different behavior based on workflow version
- **Conditional Logic**: Making decisions based on version flags

## Key Concepts

### Workflow Versioning

Workflow versioning ensures that existing workflow executions continue to use the logic they started with, while new executions can use updated logic:

```rust
// V1: Simple workflow
pub async fn versioned_workflow_v1(ctx: &mut WorkflowContext, input: String) -> Result<String, WorkflowError> {
    // Basic API call
}

// V2: Enhanced workflow with additional steps
pub async fn versioned_workflow_v2(ctx: &mut WorkflowContext, input: String) -> Result<String, WorkflowError> {
    // API call + database storage
}
```

### Side Effects

Side effects are non-deterministic operations (like getting current time or generating UUIDs) that must be handled specially to ensure deterministic replay:

```rust
// These are side effects - non-deterministic values
let timestamp = chrono::Utc::now().timestamp();
let uuid = uuid::Uuid::new_v4().to_string();
```

### Conditional Version Logic

Workflows can branch based on version flags or configuration:

```rust
let api_version = if use_v2_api && config.api_version == "v2" {
    "v2"
} else {
    "v1"
};
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p versioning

# Run all tests
cargo test -p versioning
```

## Code Structure

- `src/main.rs` - Example entry point and tests
- `src/lib.rs` - Library exports
- `src/activities/` - Activity implementations
- `src/workflows/` - Workflow implementations

## Workflows

### versioned_workflow_v1

Basic workflow demonstrating V1 behavior with simple API calls.

### versioned_workflow_v2

Enhanced workflow with additional database operations.

### side_effect_workflow

Demonstrates handling of non-deterministic side effects.

### conditional_version_workflow

Shows conditional behavior based on version flags and service configuration.

## Activities

### external_api_call_activity

Makes external API calls with different versions.

### database_query_activity

Executes database queries with version-specific schema handling.

### get_service_config_activity

Retrieves service configuration including API versions.

## Related Examples

- Previous: [07_time_and_sleep](../07_time_and_sleep) - Timers and time manipulation
- Next: [09_continue_as_new](../09_continue_as_new) - Continue-as-new workflows
