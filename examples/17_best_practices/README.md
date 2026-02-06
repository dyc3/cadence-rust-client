# Example 25: Best Practices

## Overview

This example demonstrates idiomatic patterns and best practices for building production-grade Cadence workflows in Rust. It serves as a reference implementation for teams adopting Cadence in their production systems.

## Features Demonstrated

### Type Safety
- **Newtype Patterns**: Prevent mixing up different ID types
- **Validated Types**: Ensure data validity at construction
- **Strongly-Typed Enums**: Exhaustive pattern matching
- **Builder Patterns**: Type-safe complex object construction

### Error Handling
- **Structured Error Types**: Using `thiserror` for ergonomic error handling
- **Idempotency**: Exactly-once semantics with deduplication
- **Circuit Breakers**: Fail-fast for failing dependencies
- **Retry Policies**: Exponential backoff with jitter

### Observability
- **Structured Logging**: Consistent, searchable log format
- **Distributed Tracing**: Context propagation across activities
- **Correlation IDs**: Request tracking through the system
- **Metrics**: Performance and health monitoring

### Configuration
- **Environment-Based**: 12-factor app configuration
- **Validation**: Fail fast on invalid config
- **Defaults**: Sensible defaults for all options
- **Feature Flags**: Gradual rollout capabilities

## Code Organization

```
src/
├── lib.rs           # Module exports and documentation
├── main.rs          # Example demonstration and comprehensive tests
├── types/           # Type-safe domain types
│   └── mod.rs       # Newtypes, validated types, builders
├── activities/      # Best practice activity implementations
│   └── mod.rs       # Idempotent, validated, robust activities
├── workflows/       # Best practice workflow implementations
│   └── mod.rs       # Saga, context propagation, versioning
├── utils/           # Utility functions and helpers
│   └── mod.rs       # Retry, circuit breaker, logging
└── config/          # Configuration management
    └── mod.rs       # Environment config, validation
```

## Key Patterns

### Newtype Pattern

Prevent mixing different ID types at compile time:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WorkflowId(String);

impl WorkflowId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

Usage:
```rust
let workflow_id = WorkflowId::new("wf_123");
let user_id = UserId::new("user_456");

// This won't compile - type-safe!
// function_expecting_workflow_id(user_id); // Error!
```

### Validated Types

Ensure data validity at construction:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn new(email: impl Into<String>) -> Result<Self, EmailError> {
        let email = email.into();
        if email.contains('@') && email.contains('.') {
            Ok(Self(email))
        } else {
            Err(EmailError::InvalidFormat(email))
        }
    }
}
```

### Builder Pattern

Type-safe construction of complex objects:

```rust
let metadata = WorkflowMetadataBuilder::new()
    .workflow_id(WorkflowId::new("wf_123"))
    .workflow_type("order_processing")
    .version(WorkflowVersion::new(1, 0, 0))
    .build()?;
```

### Idempotency

Exactly-once semantics with idempotency keys:

```rust
pub async fn idempotent_process_activity(
    ctx: &ActivityContext,
    input: IdempotentProcessInput,
) -> Result<IdempotentProcessResult, ActivityError> {
    // Check if already processed
    if already_processed(&input.idempotency_key) {
        return Ok(cached_result(&input.idempotency_key));
    }
    
    // Process and cache result
    let result = process(&input).await?;
    cache_result(&input.idempotency_key, &result);
    Ok(result)
}
```

### Circuit Breaker

Protect against cascading failures:

```rust
let mut circuit_breaker = CircuitBreaker::new(
    3,                    // failure threshold
    2,                    // success threshold to close
    Duration::from_secs(30), // reset timeout
);

match circuit_breaker.call(|| async { risky_operation().await }).await {
    Ok(result) => result,
    Err(CircuitError::Open) => {
        // Fail fast when circuit is open
        Err(ServiceUnavailable)
    }
}
```

### Structured Logging

Consistent, contextual logging:

```rust
#[instrument(skip(ctx, input), fields(idempotency_key = %input.idempotency_key))]
pub async fn process_activity(
    ctx: &ActivityContext,
    input: ProcessInput,
) -> Result<ProcessResult, ActivityError> {
    info!("Starting processing");
    
    let result = process(&input).await?;
    
    info!(duration_ms = %start.elapsed().as_millis(), "Processing complete");
    Ok(result)
}
```

### Saga Pattern

Distributed transactions with compensation:

```rust
pub async fn saga_workflow(
    ctx: &mut WorkflowContext,
    input: OrderInput,
) -> Result<OrderResult, WorkflowError> {
    let mut compensations = Vec::new();
    
    // Step 1
    let step1 = ctx.execute_activity("step1", ...).await?;
    compensations.push(compensate_step1);
    
    // Step 2
    match ctx.execute_activity("step2", ...).await {
        Ok(result) => {
            compensations.push(compensate_step2);
            Ok(result)
        }
        Err(e) => {
            // Run compensations in reverse order
            for comp in compensations.into_iter().rev() {
                comp().await?;
            }
            Err(e)
        }
    }
}
```

### Configuration

Type-safe configuration with validation:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub worker: WorkerConfig,
    pub retry: RetryConfig,
    pub timeouts: TimeoutConfig,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        envy::from_env()
    }
    
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.worker.concurrency == 0 {
            return Err(ConfigError::Invalid("concurrency must be > 0"));
        }
        Ok(())
    }
}
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p best_practices

# Run all tests
cargo test -p best_practices

# Run specific test with output
cargo test -p best_practices test_all_patterns_summary -- --nocapture
```

## Testing Best Practices

### Unit Tests
- Test individual activities in isolation
- Mock external dependencies
- Test both success and failure cases

### Integration Tests
- Test complete workflow execution
- Use `TestWorkflowEnvironment` for end-to-end testing
- Test saga compensation flows

### Property-Based Tests
- Test invariants (e.g., idempotency)
- Generate random inputs
- Verify post-conditions

## Production Checklist

### Type Safety
- [ ] Newtype patterns for all IDs
- [ ] Validated types for domain concepts
- [ ] Exhaustive pattern matching
- [ ] No raw strings for IDs

### Error Handling
- [ ] Structured error types
- [ ] Idempotency keys on mutations
- [ ] Circuit breakers for external calls
- [ ] Comprehensive retry policies
- [ ] Saga compensation tested

### Observability
- [ ] Structured logging with context
- [ ] Distributed tracing enabled
- [ ] Correlation IDs propagated
- [ ] Metrics exposed
- [ ] Health checks implemented

### Configuration
- [ ] Environment-based config
- [ ] Validation at startup
- [ ] Sensible defaults
- [ ] Secrets externalized
- [ ] Feature flags for rollouts

### Documentation
- [ ] README with architecture
- [ ] Code comments for complex logic
- [ ] Examples for common patterns
- [ ] API documentation generated

## Related Examples

- Previous: [24_complete_application](../24_complete_application) - Full application example
- See also: All previous examples for specific patterns

## Additional Resources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Cadence Best Practices](https://cadenceworkflow.io/docs/concepts/workflows/)
- [Temporal Best Practices](https://docs.temporal.io/application-development/foundations)
