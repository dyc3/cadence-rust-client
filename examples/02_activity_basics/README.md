# Example 02: Activity Basics

## Overview

This example demonstrates activity chaining and composition patterns. Activities can be composed together to create complex workflows, with the output of one activity feeding into the input of another.

## Features Demonstrated

- **Multiple Activities**: Using several activities in a single workflow
- **Activity Chaining**: Output of one activity as input to another
- **Error Propagation**: How activity errors flow up to workflows
- **Result Handling**: Processing activity results in workflows

## Order Processing Pipeline

This example implements a complete order processing pipeline:

1. **Validate Order** - Checks order validity
2. **Calculate Total** - Computes subtotal, tax, and total
3. **Process Payment** - Handles payment processing
4. **Send Confirmation** - Sends order confirmation

## Key Concepts

### Activity Composition

Workflows are built by composing activities together:

```rust
// Activity 1: Validate
let validation = ctx.execute_activity("validate_order", ...).await?;
let validation_result: ValidationResult = serde_json::from_slice(&validation)?;

// Activity 2: Calculate (uses validation output)
let calc_input = (validation.order_id, items);
let calculation = ctx.execute_activity("calculate_total", ...).await?;
```

### Data Flow

Data flows through the workflow as each activity completes:

```
OrderInput → ValidationResult → CalculationResult → PaymentResult → ConfirmationResult
```

### Error Propagation

Activity errors automatically propagate to the workflow:

```rust
let result = ctx.execute_activity("process", input).await?;
// If activity fails, the ? operator propagates the error
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p activity_basics

# Run all tests
cargo test -p activity_basics
```

## Code Structure

- `src/main.rs` - Example demonstration and tests
- `src/lib.rs` - Library exports
- `src/activities/` - Activity implementations
- `src/workflows/` - Workflow implementations

## Activities

### validate_order_activity
Validates order structure and data integrity.

### calculate_total_activity
Calculates order totals including taxes.

### process_payment_activity
Processes payment for the order.

### send_confirmation_activity
Sends order confirmation notification.

## Related Examples

- Previous: [01_hello_workflow](../01_hello_workflow) - Basic workflow and activity
- Next: [03_activity_advanced](../03_activity_advanced) - Heartbeats and cancellation
