# Cadence Rust Client Examples

This directory contains 25+ comprehensive examples demonstrating all features of the Cadence Rust client library.

## Quick Start

```bash
# Run the hello world example
cargo run -p hello_workflow

# Run all example tests
cargo test --workspace --examples
```

## Example Categories

### 1. Getting Started (01-03)
Basic workflow and activity fundamentals

- **01_hello_workflow** - Basic workflow + activity (exists)
- **02_activity_basics** - Multiple activities and chaining
- **03_activity_advanced** - Heartbeats, cancellation, deadlines

### 2. Signal Handling (04-05)
Internal and external signals

- **04_workflow_signals** - Internal signal handling
- **05_workflow_external** - External signals and cancellation

### 3. Workflow Composition (06-09)
Advanced workflow patterns

- **06_child_workflows** - Child workflow execution
- **07_time_and_sleep** - Timers and time manipulation
- **08_versioning** - Workflow versioning and patches
- **09_continue_as_new** - Long-running workflows

### 4. Error Handling (10-11)
Resilience and recovery patterns

- **10_error_handling** - Error types and handling
- **11_retry_policies** - Retry configuration

### 5. Configuration (12-17)
Client, worker, and domain configuration

- **12_query_operations** - Querying workflow state
- **13_workflow_options** - Workflow lifecycle policies
- **14_search_attributes** - Searchable workflows
- **15_client_operations** - Full client API
- **16_domain_management** - Domain lifecycle
- **17_worker_configuration** - Worker setup and tuning

### 6. Advanced Features (18-19)
Production patterns

- **18_local_activities** - Low-latency activities
- **19_workflow_patterns** - Saga, fan-out/fan-in, batch

### 7. Testing (20-22)
Testing and debugging

- **20_testing_basics** - TestWorkflowEnvironment deep dive
- **21_workflow_replay** - Debugging with replay
- **22_data_converter** - Custom serialization

### 8. Production (23-25)
Performance and best practices

- **23_performance** - Performance tuning
- **24_complete_application** - Full integration example
- **25_best_practices** - Idiomatic patterns

## Structure

```
examples/
├── examples-common/          # Shared utilities and helpers
├── 01_hello_workflow/        # (exists)
├── 02_activity_basics/
├── ... (more examples)
└── 25_best_practices/
```

Each example is a standalone crate with:
- `Cargo.toml` - Dependencies
- `src/main.rs` - Example implementation
- `src/lib.rs` - Library exports (for reuse)
- `src/tests/` - Unit and integration tests
- `README.md` - Documentation

## Shared Utilities

The `examples-common` crate provides:
- Test environment setup helpers
- Mock activity implementations
- Common domain types (Order, Payment, etc.)
- Assertion helpers
- Tracing setup utilities

## Dependencies

See [EXAMPLES_DEPENDENCIES.md](../EXAMPLES_DEPENDENCIES.md) for the complete dependency graph.

## Development

### Creating a New Example

Use the helper script:

```bash
./scripts/create-example.sh <number> <name> [dependencies]
```

Example:
```bash
./scripts/create-example.sh 02 activity_basics "01_hello_workflow"
```

### Adding to Workspace

Edit the root `Cargo.toml` to add the new example to the workspace members.

## Documentation

- [EXAMPLES_DESIGN.md](../EXAMPLES_DESIGN.md) - Comprehensive design document
- [EXAMPLES_QUICK_REF.md](../EXAMPLES_QUICK_REF.md) - Quick reference guide
- [EXAMPLES_DEPENDENCIES.md](../EXAMPLES_DEPENDENCIES.md) - Dependency graph

## Learning Path

### Beginner
1. 01_hello_workflow
2. 02_activity_basics
3. 04_workflow_signals
4. 20_testing_basics

### Intermediate
5. 03_activity_advanced
6. 06_child_workflows
7. 10_error_handling
8. 12_query_operations

### Advanced
9. 08_versioning
9. 19_workflow_patterns
10. 21_workflow_replay

### Production
11. 17_worker_configuration
12. 23_performance
13. 24_complete_application
14. 25_best_practices
