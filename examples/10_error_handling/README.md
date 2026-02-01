# Example 10: Error Handling

## Overview

This example demonstrates error types and handling in Cadence workflows. Proper error handling is crucial for building robust workflows that can recover from failures and handle edge cases gracefully.

## Features Demonstrated

- **Activity Error Types**: Retryable and non-retryable errors
- **Workflow Error Propagation**: How errors flow through workflow execution
- **Retry Policies**: Configuring automatic retry behavior
- **Error Classification**: Distinguishing between retryable and non-retryable failures
- **Fallback Patterns**: Alternative execution paths when primary methods fail
- **Validation**: Early validation to prevent unnecessary processing

## Key Concepts

### ActivityError Types

Cadence activities can return different types of errors:

```rust
// Retryable error - will be retried according to retry policy
Err(ActivityError::Retryable("Temporary network error".to_string()))

// Non-retryable error - will fail immediately without retries
Err(ActivityError::NonRetryable("Invalid input".to_string()))

// Retryable with specific delay
Err(ActivityError::RetryableWithDelay {
    message: "Rate limited".to_string(),
    retry_after: Duration::from_secs(60),
})
```

### Retry Policies

Configure retry behavior for activities:

```rust
let retry_policy = RetryPolicy {
    initial_interval: Duration::from_secs(1),
    backoff_coefficient: 2.0,
    maximum_interval: Duration::from_secs(30),
    maximum_attempts: 3,
    non_retryable_error_types: vec!["NonRetryable".to_string()],
};
```

### Fallback Patterns

Implement fallback logic when primary execution fails:

```rust
match ctx.execute_activity("primary", input, options).await {
    Ok(result) => Ok(result),
    Err(_) => {
        // Try fallback
        ctx.execute_activity("fallback", input, fallback_options).await
    }
}
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p error_handling

# Run all tests
cargo test -p error_handling
```

## Code Structure

- `src/main.rs` - Example entry point and tests
- `src/lib.rs` - Library exports
- `src/activities/` - Activity implementations
- `src/workflows/` - Workflow implementations

## Workflows

### error_handling_workflow

Demonstrates comprehensive error handling with retry logic for batch processing.

### fallback_workflow

Shows fallback patterns - trying alternative approaches when primary method fails.

### validation_workflow

Demonstrates early validation to fail fast and avoid unnecessary processing.

## Activities

### unreliable_process_activity

Simulates an unreliable activity that can fail in various ways for testing error handling.

### external_service_call_activity

Demonstrates error handling for external service calls with different failure scenarios.

### validate_data_activity

Validates input data and returns validation errors as non-retryable failures.

## Error Classification

Understanding when to use each error type:

### Retryable Errors

Use for transient failures that might succeed on retry:
- Network timeouts
- Service temporarily unavailable
- Rate limiting (with delay)

### Non-Retryable Errors

Use for permanent failures that won't succeed on retry:
- Invalid input data
- Authentication failures
- Not found errors
- Validation errors

## Best Practices

1. **Fail Fast**: Validate inputs early to avoid wasting resources
2. **Classify Errors**: Distinguish between retryable and non-retryable errors
3. **Use Backoff**: Implement exponential backoff to avoid overwhelming services
4. **Set Limits**: Configure maximum retry attempts to prevent infinite loops
5. **Log Details**: Include context in error messages for debugging

## Related Examples

- Previous: [09_continue_as_new](../09_continue_as_new) - Continue-as-new workflows
- See also: [02_activity_basics](../02_activity_basics) - Activity chaining and error propagation
