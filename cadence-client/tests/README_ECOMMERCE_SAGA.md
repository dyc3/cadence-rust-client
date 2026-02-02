# Ecommerce Saga Integration Test

## Overview

This integration test demonstrates a real-world ecommerce order processing workflow with saga pattern compensation logic using a **real Cadence server**. It provides comprehensive infrastructure for testing distributed transactions, failure handling, and compensation patterns.

## What's Implemented

### ✅ Complete Infrastructure

1. **Test File**: `cadence-client/tests/ecommerce_saga_integration.rs`
   - 800+ lines of comprehensive test infrastructure
   - Real Cadence server integration (not mocked)
   - Production-ready data models

2. **Ecommerce Data Models**:
   - `OrderInput`, `OrderOutput`, `OrderItem`, `OrderStatus`
   - `PaymentInfo`, `PaymentResult`, `PaymentStatus`, `PaymentMethod`
   - `InventoryReservation`, `ReservedItem`
   - `NotificationRequest`, `NotificationResult`
   - `Address` for shipping

3. **Helper Functions**:
   - Domain and workflow setup
   - History retrieval with pagination
   - Activity execution verification
   - Compensation detection
   - Workflow completion waiting (with timeout)

4. **Working Tests** (3 tests, all passing):
   - ✅ `test_order_saga_workflow_start` - Verifies workflow creation and history
   - ✅ `test_inspect_workflow_history_structure` - Demonstrates detailed history inspection
   - ✅ `test_verification_helpers` - Validates assertion helper functions

### ⏳ Pending Implementation

To enable full saga pattern tests with actual workflow execution, the following components need to be implemented in the `cadence-worker` crate:

1. **Worker Polling Infrastructure**:
   - Decision task polling and processing
   - Activity task polling and execution
   - Response submission to Cadence server

2. **Workflow Execution Engine**:
   - History replay for deterministic execution
   - Decision generation based on workflow logic
   - Event sourcing state management

3. **Activity Registry**:
   - Dynamic activity lookup
   - Context creation and management
   - Result serialization

## Running the Tests

### Prerequisites

Start the Cadence server:

```bash
docker compose up -d
```

Wait for services to be ready (2-3 minutes):

```bash
docker compose ps
docker compose logs -f cadence
```

### Execute Tests

Run all ecommerce saga integration tests:

```bash
cargo test --test ecommerce_saga_integration -- --ignored --test-threads=1 --nocapture
```

Run a specific test:

```bash
cargo test --test ecommerce_saga_integration test_inspect_workflow_history_structure -- --ignored --nocapture
```

### Expected Output

```
running 3 tests

=== Testing Workflow History Structure ===
  Domain: ecommerce-test-domain-433ef5b0-b9db-473d-bb40-ba5ddfe6bc7e
  Workflow ID: history-inspection-23cf122a-75c2-48b0-9cde-a0fb210bda48
  ✓ Domain registered
  ✓ Workflow started
  Run ID: 7b46275a-5a54-4790-9618-e4c3c43b87e7
  ✓ Retrieved workflow history (2 events)
  ...
  ✓ History structure verified
ok

test result: ok. 3 passed; 0 failed; 0 ignored
```

## Test Scenarios

### Current Tests

1. **test_order_saga_workflow_start**
   - Creates unique domain and task list
   - Starts order processing workflow
   - Retrieves and verifies workflow history
   - Validates initial events (WorkflowExecutionStarted, DecisionTaskScheduled)

2. **test_inspect_workflow_history_structure**
   - Demonstrates detailed history event inspection
   - Parses workflow input from history
   - Extracts event attributes
   - Shows how to verify saga compensation patterns

3. **test_verification_helpers**
   - Tests all assertion helper functions
   - Validates helper behavior without worker
   - Documents helper usage patterns

### Future Tests (Documented in Code)

Once the worker is implemented, these tests will be enabled:

1. **test_order_saga_success_path**
   - Full order processing succeeds
   - Calculate → Reserve → Pay → Notify
   - Verifies all activities complete successfully

2. **test_order_saga_payment_failure**
   - Payment fails (amount > $10,000)
   - Inventory is released (compensation)
   - Verifies saga pattern compensation logic

3. **test_order_saga_inventory_failure**
   - Insufficient inventory (quantity > 1000)
   - No payment attempted
   - Verifies early failure handling

4. **test_order_saga_retry_success**
   - Payment retries and eventually succeeds
   - Verifies retry policy configuration

5. **test_verify_compensation_history**
   - Detailed inspection of compensation events
   - Verifies event sequence and timing

## Helper Functions

### Setup Helpers

```rust
// Create gRPC client
let client = create_grpc_client(&domain_name).await?;

// Register domain and wait for propagation
register_domain_and_wait(&client, &domain_name).await?;

// Generate unique IDs
let domain = generate_test_domain_name();
let workflow_id = generate_workflow_id("prefix");
let task_list = generate_task_list_name();

// Create workflow request
let request = create_order_workflow_request(&domain, &task_list, &workflow_id, &input);
```

### History Verification

```rust
// Get full workflow history (handles pagination)
let history = get_workflow_history(&client, &domain, &workflow_id, &run_id).await?;

// Wait for workflow completion (with timeout)
let result = wait_for_workflow_completion(&client, &domain, &workflow_id, &run_id, timeout).await?;

// Check activity execution
assert!(assert_activity_executed(&history, "process_payment"));
assert!(assert_activity_failed(&history, "process_payment"));

// Verify compensation
assert!(assert_compensation_executed(&history));

// Count retry attempts
let attempts = count_activity_attempts(&history, "process_payment");

// Print history for debugging
print_history_events(&history);
```

## Architecture

### Data Flow

```
Test → GrpcWorkflowServiceClient → Cadence Server
  ↓
Start Workflow Execution
  ↓
Retrieve History
  ↓
Verify Events

(Future with Worker)
  ↓
Worker Polls for Tasks
  ↓
Execute Activities
  ↓
Submit Results
  ↓
Verify Compensation
```

### Event Sequence (Success Path)

```
1. WorkflowExecutionStarted
2. DecisionTaskScheduled
3. DecisionTaskStarted
4. DecisionTaskCompleted
5. ActivityTaskScheduled (calculate_order_total)
6. ActivityTaskStarted (calculate_order_total)
7. ActivityTaskCompleted (calculate_order_total)
8. DecisionTaskScheduled
9. DecisionTaskCompleted
10. ActivityTaskScheduled (reserve_inventory)
11. ActivityTaskCompleted (reserve_inventory)
...
```

### Event Sequence (Compensation Path)

```
...
10. ActivityTaskScheduled (process_payment)
11. ActivityTaskStarted (process_payment)
12. ActivityTaskFailed (process_payment) ← Failure
13. DecisionTaskScheduled
14. DecisionTaskCompleted
15. ActivityTaskScheduled (release_inventory) ← Compensation
16. ActivityTaskCompleted (release_inventory)
17. WorkflowExecutionFailed
```

## Key Features

### 1. Real Cadence Server Integration
- Uses actual gRPC communication
- Tests against production Cadence server
- No mocks or test environments

### 2. Production-Ready Models
- Complete ecommerce domain models
- Realistic order processing flow
- Saga pattern with compensation

### 3. Comprehensive History Inspection
- Pagination support for large histories
- Event attribute parsing
- Pattern matching on event types

### 4. Saga Pattern Support
- Compensation detection
- Activity failure tracking
- Retry attempt counting

### 5. Test Isolation
- Unique domains per test (UUID-based)
- Unique task lists per test
- Sequential execution to avoid conflicts

## Implementation Notes

### Why No Worker Yet?

The `cadence-worker` crate currently has stub implementations:

```rust
impl Worker for CadenceWorker {
    fn start(&self) -> Result<(), WorkerError> {
        // TODO: Implement worker start
        Ok(())
    }
}
```

Implementing a production-grade worker requires:
- Decision task polling loop
- Activity task polling loop
- History replay for workflows
- Deterministic decision generation
- Task token management
- Graceful shutdown handling

### Testing Strategy

**Phase 1 (Current)**: Infrastructure Testing
- ✅ Verify gRPC connectivity
- ✅ Validate domain management
- ✅ Test workflow creation
- ✅ Inspect history events
- ✅ Demonstrate helper usage

**Phase 2 (Future)**: Full Saga Testing
- ⏳ Implement worker polling
- ⏳ Execute real workflows
- ⏳ Trigger failures
- ⏳ Verify compensation
- ⏳ Test retry policies

## Code Examples

### Starting a Workflow

```rust
let order_input = OrderInput {
    user_id: "user-123".to_string(),
    items: vec![OrderItem {
        product_id: "prod-456".to_string(),
        sku: "SKU-789".to_string(),
        quantity: 2,
        unit_price: 49.99,
    }],
    shipping_address: Address { /* ... */ },
    payment_method: PaymentMethod::CreditCard { /* ... */ },
};

let request = create_order_workflow_request(&domain, &task_list, &workflow_id, &order_input);
let response = client.start_workflow_execution(request).await?;
```

### Inspecting History

```rust
let history = get_workflow_history(&client, &domain, &workflow_id, &run_id).await?;

for event in &history {
    match event.event_type {
        EventType::WorkflowExecutionStarted => {
            if let Some(EventAttributes::WorkflowExecutionStartedEventAttributes(attrs)) = &event.attributes {
                let input: OrderInput = serde_json::from_slice(&attrs.input)?;
                println!("Order for user: {}", input.user_id);
            }
        }
        EventType::ActivityTaskCompleted => {
            // Verify activity execution
        }
        _ => {}
    }
}
```

## Future Enhancements

1. **Worker Implementation**
   - Complete polling infrastructure
   - Activity execution engine
   - Workflow decision engine

2. **Additional Test Scenarios**
   - Timeout handling
   - Cancellation patterns
   - Long-running workflows
   - Child workflow composition

3. **Performance Testing**
   - Throughput measurements
   - Latency tracking
   - Resource utilization

4. **CI/CD Integration**
   - Automated Cadence server startup
   - Test result reporting
   - Coverage tracking

## References

- **Existing Tests**: `cadence-client/tests/grpc_integration.rs`
- **Ecommerce Models**: `examples/24_complete_application/src/models/mod.rs`
- **Saga Workflows**: `examples/24_complete_application/src/workflows/mod.rs`
- **Activity Examples**: `examples/24_complete_application/src/activities/mod.rs`

## Contributing

To extend these tests:

1. Add new data models to the models section
2. Implement helper functions for common operations
3. Create new test scenarios following existing patterns
4. Document the test purpose and expected behavior
5. Ensure tests use unique domains and are isolated

## License

Same as the parent Cadence Rust client project.
