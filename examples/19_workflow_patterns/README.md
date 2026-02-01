# Example 19: Advanced Workflow Patterns

## Overview

This example demonstrates advanced workflow patterns for building resilient, scalable distributed applications with Cadence.

## Patterns Demonstrated

### 1. Saga Pattern

The Saga pattern manages distributed transactions across multiple services. If any step fails, previous steps are compensated (rolled back).

**Use Cases:**
- E-commerce order processing (payment → inventory → shipping)
- Financial transactions across multiple accounts
- Multi-service data consistency

**Implementation:**
```rust
pub async fn order_saga_workflow(
    ctx: &mut WorkflowContext,
    order_input: OrderSagaInput,
) -> Result<OrderSagaResult, WorkflowError> {
    // Execute steps sequentially
    let payment = process_payment(ctx, &order_input).await?;
    let inventory = reserve_inventory(ctx, &order_input).await
        .or_else(|e| {
            compensate_payment(ctx, &order_input).await?;
            Err(e)
        })?;
    // ... more steps
}
```

### 2. Fan-out/Fan-in Pattern

Distributes work across multiple parallel activities and aggregates results.

**Use Cases:**
- Batch data processing
- Parallel API calls
- Map-reduce operations

**Implementation:**
```rust
// Fan-out: Create parallel tasks
let futures: Vec<_> = records.iter()
    .map(|r| process_record(ctx, r))
    .collect();

// Fan-in: Wait for all results
let results = join_all(futures).await;
```

### 3. Circuit Breaker Pattern

Prevents cascading failures by failing fast when a service is unhealthy.

**Use Cases:**
- External API calls
- Database connections
- Third-party service integrations

**Implementation:**
```rust
let failure_count = ctx.mutable_side_effect("circuit_count", || 0).await;
if failure_count >= threshold {
    return Err("Circuit open".into());
}
```

## Running the Example

```bash
# Run the demonstration
cargo run -p workflow_patterns

# Run all tests
cargo test -p workflow_patterns
```

## Key Concepts

### Compensation

Compensation is the rollback action for a completed step in a saga:

- **Payment** → Refund
- **Inventory Reserve** → Release
- **Shipment Create** → Cancel

### Deterministic Execution

All workflow logic must be deterministic:
- Use `ctx.side_effect()` for non-deterministic operations
- Use `ctx.mutable_side_effect()` for cached side effects
- Use `ctx.get_version()` for workflow versioning

### Error Handling

```rust
match execute_activity(ctx, "operation", input).await {
    Ok(result) => result,
    Err(e) => {
        // Log error
        // Compensate if needed
        // Return appropriate error
    }
}
```

## Code Structure

- `src/workflows/mod.rs` - Workflow implementations (Saga, Fan-out/Fan-in, Circuit Breaker)
- `src/activities/mod.rs` - Activity implementations (Payment, Inventory, Shipping, etc.)
- `src/main.rs` - Example demonstrations and tests

## Related Examples

- Previous: [18_scalable_workers](../18_scalable_workers) - Worker scaling patterns
- Next: [20_testing_basics](../20_testing_basics) - Testing fundamentals
