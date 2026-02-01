# Cadence Rust Client Examples Structure Design

## Overview

This document outlines a comprehensive 25+ example structure for the Cadence Rust client library, organized into 12 logical groups. Each example is a standalone crate that demonstrates specific features and builds on previous examples.

## Architecture Principles

1. **Test-First**: All examples use `TestWorkflowEnvironment` - no real server required
2. **Progressive Complexity**: Examples build on each other logically
3. **Consistency**: All examples follow the same template/structure
4. **Minimal Duplication**: Shared utilities extracted to a common crate
5. **Documentation-First**: Heavy inline documentation explaining concepts

## Directory Structure

```
examples/
├── examples-common/          # Shared utilities and helpers
├── 01_hello_workflow/        # Already exists - basic workflow + activity
├── 02_activity_basics/       # Activity execution patterns
├── 03_activity_advanced/     # Heartbeats, cancellation, deadlines
├── 04_workflow_signals/      # Internal signal handling
├── 05_workflow_external/     # External signals and cancellation
├── 06_child_workflows/       # Child workflow patterns
├── 07_time_and_sleep/        # Timers, sleep, now, time manipulation
├── 08_versioning/            # get_version, side_effect, patches
├── 09_continue_as_new/       # Continue-as-new workflows
├── 10_error_handling/        # Custom errors, timeouts, cancellation
├── 11_retry_policies/        # Retry configuration patterns
├── 12_query_operations/      # Query workflows
├── 13_workflow_options/      # Workflow ID reuse, parent policies
├── 14_search_attributes/     # Upsert search attributes
├── 15_client_operations/     # Client API patterns
├── 16_domain_management/     # Domain registration and config
├── 17_worker_configuration/  # Worker setup and options
├── 18_local_activities/      # Local activity execution
├── 19_workflow_patterns/     # Saga, fan-out/fan-in, batch
├── 20_testing_basics/        # TestWorkflowEnvironment deep dive
├── 21_workflow_replay/       # WorkflowReplayer for debugging
├── 22_data_converter/        # Custom serialization
├── 23_performance/           # Performance patterns (sticky cache)
├── 24_complete_application/  # Full end-to-end example
└── 25_best_practices/        # Idiomatic patterns and anti-patterns
```

## Example Groups & Implementation Order

### Phase 1: Foundations (Examples 1-3)

#### Group 1: Getting Started
**Example: `01_hello_workflow`** (Already exists)
- **Features**: Basic workflow, basic activity, worker setup
- **Dependencies**: None
- **Concepts**: Workflow definition, activity definition, serialization

#### Group 2: Activity Fundamentals
**Example: `02_activity_basics`**
- **Features**: Multiple activities, activity chaining, activity results
- **Dependencies**: 01_hello_workflow
- **Concepts**: Activity composition, passing data between activities, error propagation

**Example: `03_activity_advanced`**
- **Features**: Heartbeats, get_heartbeat_details, deadline tracking, cancellation detection, ErrResultPending
- **Dependencies**: 02_activity_basics
- **Concepts**: Long-running activities, progress tracking, graceful cancellation

### Phase 2: Workflow Core (Examples 4-9)

#### Group 3: Signal Handling
**Example: `04_workflow_signals`**
- **Features**: Internal signals, signal channels, waiting for signals
- **Dependencies**: 01_hello_workflow
- **Concepts**: Signal-driven workflows, signal multiplexing

**Example: `05_workflow_external`**
- **Features**: External signals, external cancellation, signal with start
- **Dependencies**: 04_workflow_signals
- **Concepts**: Workflow coordination, cancellation patterns

#### Group 4: Workflow Composition
**Example: `06_child_workflows`**
- **Features**: execute_child_workflow, ChildWorkflowOptions, parent-child communication
- **Dependencies**: 02_activity_basics
- **Concepts**: Workflow decomposition, parent close policies

#### Group 5: Time Management
**Example: `07_time_and_sleep`**
- **Features**: sleep, now, timer futures, TestTime manipulation
- **Dependencies**: 04_workflow_signals
- **Concepts**: Temporal logic, testing time-based workflows

#### Group 6: Workflow Evolution
**Example: `08_versioning`**
- **Features**: get_version, side_effect, mutable_side_effect
- **Dependencies**: 02_activity_basics
- **Concepts**: Workflow code changes without breaking history

**Example: `09_continue_as_new`**
- **Features**: continue_as_new, large history handling
- **Dependencies**: 06_child_workflows
- **Concepts**: Infinite workflows, periodic processing

### Phase 3: Error Handling & Resilience (Examples 10-11)

#### Group 7: Error Patterns
**Example: `10_error_handling`**
- **Features**: CustomError, CanceledError, TimeoutError, PanicError, ActivityTaskError, ChildWorkflowExecutionError
- **Dependencies**: 03_activity_advanced, 06_child_workflows
- **Concepts**: Error classification, recovery strategies

**Example: `11_retry_policies`**
- **Features**: RetryPolicy, ActivityOptions.retry_policy, exponential backoff
- **Dependencies**: 10_error_handling
- **Concepts**: Transient failure handling, retry configuration

### Phase 4: Configuration & Management (Examples 12-17)

#### Group 8: Workflow Configuration
**Example: `12_query_operations`**
- **Features**: Query workflows, query handlers, QueryConsistencyLevel
- **Dependencies**: 01_hello_workflow
- **Concepts**: Runtime workflow inspection

**Example: `13_workflow_options`**
- **Features**: WorkflowIdReusePolicy, ParentClosePolicy, ChildWorkflowOptions
- **Dependencies**: 06_child_workflows
- **Concepts**: Workflow lifecycle policies

**Example: `14_search_attributes`**
- **Features**: upsert_search_attributes, searchable workflows
- **Dependencies**: 01_hello_workflow
- **Concepts**: Workflow discoverability, attribute updates

#### Group 9: Client & Domain Operations
**Example: `15_client_operations`**
- **Features**: start_workflow, execute_workflow, signal_workflow, signal_with_start, cancel, terminate, query, get_history, list_executions
- **Dependencies**: 05_workflow_external, 12_query_operations
- **Concepts**: Full client API surface

**Example: `16_domain_management`**
- **Features**: Domain registration, description, configuration
- **Dependencies**: None (standalone)
- **Concepts**: Multi-tenant setup, domain lifecycle

#### Group 10: Worker Configuration
**Example: `17_worker_configuration`**
- **Features**: Worker registration, configuration, sticky execution, cache size
- **Dependencies**: 01_hello_workflow
- **Concepts**: Worker optimization, sticky task lists

### Phase 5: Advanced Features (Examples 18-19)

#### Group 11: Advanced Execution
**Example: `18_local_activities`**
- **Features**: execute_local_activity, LocalActivityOptions, when to use
- **Dependencies**: 02_activity_basics
- **Concepts**: Low-latency activity execution

**Example: `19_workflow_patterns`**
- **Features**: Saga pattern, fan-out/fan-in, batch processing
- **Dependencies**: 06_child_workflows, 10_error_handling
- **Concepts**: Common workflow design patterns

### Phase 6: Testing & Debugging (Examples 20-22)

#### Group 12: Testing Deep Dive
**Example: `20_testing_basics`**
- **Features**: TestWorkflowEnvironment comprehensive usage, TestActivityEnvironment
- **Dependencies**: 01_hello_workflow
- **Concepts**: Unit testing workflows, mocking activities

**Example: `21_workflow_replay`**
- **Features**: WorkflowReplayer, debugging workflow histories
- **Dependencies**: 20_testing_basics
- **Concepts**: Non-determinism detection, history replay

**Example: `22_data_converter`**
- **Features**: Custom DataConverter, JsonDataConverter, serialization strategies
- **Dependencies**: 01_hello_workflow
- **Concepts**: Custom serialization, encryption at rest

### Phase 7: Production Patterns (Examples 23-25)

#### Group 13: Production Readiness
**Example: `23_performance`**
- **Features**: Sticky cache tuning, worker options, performance optimization
- **Dependencies**: 17_worker_configuration
- **Concepts**: Production performance tuning

**Example: `24_complete_application`**
- **Features**: Full application with all components integrated
- **Dependencies**: All previous examples
- **Concepts**: End-to-end architecture, best practices integration

**Example: `25_best_practices`**
- **Features**: Idiomatic patterns, anti-patterns to avoid, design guidelines
- **Dependencies**: All previous examples
- **Concepts**: Production guidelines, common pitfalls

## Shared Utilities Crate

### `examples-common` Structure

```
examples-common/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── test_helpers.rs       # Test environment setup
    ├── workflows/
    │   ├── common.rs         # Shared workflow definitions
    │   └── order_processing.rs # Reusable domain example
    ├── activities/
    │   ├── common.rs         # Shared activity definitions
    │   └── mock_activities.rs # Mock implementations for tests
    ├── types/
    │   ├── order.rs          # Shared domain types
    │   └── payment.rs        # Payment-related types
    └── assertions.rs         # Test assertions helpers
```

### Common Utilities Provided

1. **Test Environment Setup**:
   ```rust
   pub fn setup_test_env() -> TestWorkflowEnvironment
   pub fn setup_test_env_with_time(time: DateTime<Utc>) -> TestWorkflowEnvironment
   ```

2. **Common Workflow Types**:
   - Order processing domain (Order, Payment, Inventory types)
   - Standard input/output structs
   - Error types

3. **Mock Activities**:
   - `mock_payment_activity()` - simulates payment processing
   - `mock_inventory_activity()` - simulates inventory operations
   - `mock_notification_activity()` - simulates sending notifications

4. **Assertion Helpers**:
   - `assert_workflow_completed_ok()`
   - `assert_activity_executed_n_times()`
   - `assert_workflow_timed_out()`

5. **Tracing Setup**:
   - `init_test_tracing()` - consistent tracing initialization

## Example Template Structure

Each example crate follows this exact structure:

```
examples/XX_example_name/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs              # Example entry point (demonstrates the feature)
    ├── lib.rs               # Library exports (for integration with other examples)
    └── tests/
        ├── integration_tests.rs  # Integration tests
        └── unit_tests.rs        # Unit tests with TestWorkflowEnvironment
```

### Cargo.toml Template

```toml
[package]
name = "XX_example_name"
version = "0.1.0"
edition = "2021"
description = "Brief description of what this example demonstrates"

[dependencies]
# Core Cadence crates
cadence-client = { path = "../../cadence-client" }
cadence-worker = { path = "../../cadence-worker" }
cadence-workflow = { path = "../../cadence-workflow" }
cadence-activity = { path = "../../cadence-activity" }
cadence-core = { path = "../../cadence-core" }
cadence-testsuite = { path = "../../cadence-testsuite" }

# Shared examples utilities
examples-common = { path = "../examples-common" }

# Async runtime
tokio = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Utilities
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = "0.3"
chrono = { workspace = true }
uuid = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
```

### main.rs Template

```rust
//! # Example XX: Description
//!
//! This example demonstrates:
//! - Feature 1
//! - Feature 2
//! - Feature 3
//!
//! ## Concepts Covered
//!
//! 1. **Concept 1**: Brief explanation
//! 2. **Concept 2**: Brief explanation
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p XX_example_name
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p XX_example_name
//! ```

use cadence_activity::{ActivityContext, ActivityError, ActivityInfo};
use cadence_core::{ActivityOptions, WorkflowExecution};
use cadence_workflow::{WorkflowContext, WorkflowError};
use examples_common::test_helpers::setup_test_env;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

/// Example input type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleInput {
    pub field: String,
}

/// Example output type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleOutput {
    pub result: String,
}

/// Example activity implementation
pub async fn example_activity(
    ctx: &ActivityContext,
    input: ExampleInput,
) -> Result<ExampleOutput, ActivityError> {
    info!("Executing example_activity with input: {:?}", input);
    
    // Activity logic here
    
    Ok(ExampleOutput {
        result: format!("Processed: {}", input.field),
    })
}

/// Example workflow implementation
pub async fn example_workflow(
    ctx: &mut WorkflowContext,
    input: ExampleInput,
) -> Result<ExampleOutput, WorkflowError> {
    info!("Starting example_workflow");
    
    // Workflow logic here
    let result = ctx.execute_activity(
        "example_activity",
        Some(serde_json::to_vec(&input).unwrap()),
        ActivityOptions::default(),
    ).await?;
    
    let output: ExampleOutput = serde_json::from_slice(&result.unwrap_or_default())
        .map_err(|e| WorkflowError::Generic(format!("Parse error: {}", e)))?;
    
    info!("Workflow completed: {:?}", output);
    Ok(output)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("\n=== Example XX: Description ===\n");
    
    // Demonstrate the feature
    let mut env = setup_test_env();
    
    // Register workflow and activities
    // env.register_workflow(...);
    // env.register_activity(...);
    
    // Run the example
    let input = ExampleInput {
        field: "test".to_string(),
    };
    
    println!("Input: {:?}", input);
    println!("\nExample completed successfully!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadence_testsuite::TestWorkflowEnvironment;
    
    #[tokio::test]
    async fn test_example_feature() {
        let mut env = TestWorkflowEnvironment::new();
        
        // Register mocks
        // env.register_activity(...);
        // env.register_workflow(...);
        
        // Execute and verify
        // let result = env.execute_workflow(...).await;
        // assert!(result.is_ok());
    }
}
```

## Implementation Roadmap

### Phase 1 (Week 1-2): Foundations
1. Create `examples-common` crate with shared utilities
2. Update existing `01_hello_workflow` to use common utilities
3. Implement `02_activity_basics`
4. Implement `03_activity_advanced`

### Phase 2 (Week 3-4): Workflow Core
5. Implement `04_workflow_signals`
6. Implement `05_workflow_external`
7. Implement `06_child_workflows`
8. Implement `07_time_and_sleep`
9. Implement `08_versioning`
10. Implement `09_continue_as_new`

### Phase 3 (Week 5): Error Handling
11. Implement `10_error_handling`
12. Implement `11_retry_policies`

### Phase 4 (Week 6-7): Configuration
13. Implement `12_query_operations`
14. Implement `13_workflow_options`
15. Implement `14_search_attributes`
16. Implement `15_client_operations`
17. Implement `16_domain_management`
18. Implement `17_worker_configuration`

### Phase 5 (Week 8): Advanced Features
19. Implement `18_local_activities`
20. Implement `19_workflow_patterns`

### Phase 6 (Week 9): Testing
21. Implement `20_testing_basics`
22. Implement `21_workflow_replay`
23. Implement `22_data_converter`

### Phase 7 (Week 10): Production
24. Implement `23_performance`
25. Implement `24_complete_application`
26. Implement `25_best_practices`

## Dependencies Graph

```
examples-common (foundation)
    ↓
01_hello_workflow
    ↓
    ├─→ 02_activity_basics ──→ 03_activity_advanced
    │                            ↓
    │                         10_error_handling
    │                            ↓
    │                         11_retry_policies
    │
    ├─→ 04_workflow_signals ──→ 05_workflow_external ──→ 15_client_operations
    │                            ↓
    ├─→ 07_time_and_sleep
    │
    ├─→ 17_worker_configuration ──→ 23_performance
    │
    └─→ 12_query_operations ──→ 15_client_operations
         ↓
    20_testing_basics ──→ 21_workflow_replay

02_activity_basics
    ├─→ 06_child_workflows ──→ 09_continue_as_new
    │                            ↓
    ├─→ 08_versioning
    │
    ├─→ 18_local_activities
    │
    └─→ 19_workflow_patterns

03_activity_advanced + 06_child_workflows → 10_error_handling
06_child_workflows + 10_error_handling → 19_workflow_patterns

Standalone (no dependencies):
- 16_domain_management
- 22_data_converter (can reference any)

Final Integration:
All examples → 24_complete_application, 25_best_practices
```

## Testing Strategy

Each example includes:

1. **Unit Tests** (in `src/tests/unit_tests.rs`):
   - Test with `TestWorkflowEnvironment`
   - Mock all activities
   - Test happy path and error cases
   - Test time manipulation where applicable

2. **Integration Tests** (in `src/tests/integration_tests.rs`):
   - Test workflow + activity integration
   - Test multiple workflow patterns
   - Verify serialization/deserialization

3. **Documentation Tests**:
   - All code blocks in README.md and inline docs are tested
   - Example snippets compile and run

## README.md Template

Each example includes a comprehensive README:

```markdown
# Example XX: Title

## Overview

Brief description of what this example demonstrates and why it matters.

## Features Demonstrated

- Feature 1: Brief explanation
- Feature 2: Brief explanation
- Feature 3: Brief explanation

## Key Concepts

### Concept 1
Detailed explanation of the first concept with code snippet.

### Concept 2
Detailed explanation of the second concept with code snippet.

## Running the Example

```bash
cargo run -p XX_example_name
```

## Running Tests

```bash
cargo test -p XX_example_name
```

## Key APIs Used

- `api_name()` - Purpose
- `another_api()` - Purpose

## Related Examples

- Previous: [Example XX-1](../XX-1_example_name)
- Next: [Example XX+1](../XX+1_example_name)

## Learn More

- [Cadence Documentation](link)
- [Temporal SDK Documentation](link) (similar concepts)
```

## CI/CD Integration

All examples should:

1. **Compile** successfully in CI
2. **Pass tests** in CI
3. **Follow linting** rules (clippy)
4. **Have code coverage** measured
5. **Build in release mode** to verify optimizations

Add to workspace root `Cargo.toml`:

```toml
[workspace]
members = [
    "cadence-proto",
    "cadence-core",
    "cadence-client",
    "cadence-worker",
    "cadence-workflow",
    "cadence-activity",
    "cadence-testsuite",
    "examples/examples-common",
    "examples/01_hello_workflow",
    "examples/02_activity_basics",
    # ... etc
]
```

## Summary

This structure provides:

- **25 comprehensive examples** covering all features
- **Logical progression** from basic to advanced
- **Minimal duplication** via shared utilities
- **Consistent structure** across all examples
- **Test-first approach** with TestWorkflowEnvironment
- **Production-ready patterns** in later examples
- **Complete documentation** with inline comments and READMEs

Total estimated lines of code: ~5,000-8,000 (very manageable)
Total development time: ~10 weeks (can parallelize across groups)
