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

### 4. Error Handling (10)
Resilience and recovery patterns

- **10_error_handling** - Error types and handling

### 5. Configuration (11-14)
Client, worker, and domain configuration

- **11_client_operations** - Full client API
- **12_domain_management** - Domain lifecycle
- **13_worker_configuration** - Worker setup and tuning
- **14_local_activities** - Low-latency activities

### 6. Advanced Features (15-18)
Production patterns

- **15_performance** - Performance tuning
- **16_complete_application** - Full integration example
- **17_best_practices** - Idiomatic patterns
- **18_channels_and_spawn** - Advanced concurrency patterns

## Structure

```
examples/
├── examples-common/          # Shared utilities and helpers
├── 01_hello_workflow/
├── 02_activity_basics/
├── 03_activity_advanced/
├── 04_workflow_signals/
├── 05_workflow_external/
├── 06_child_workflows/
├── 07_time_and_sleep/
├── 08_versioning/
├── 09_continue_as_new/
├── 10_error_handling/
├── 11_client_operations/
├── 12_domain_management/
├── 13_worker_configuration/
├── 14_local_activities/
├── 15_performance/
├── 16_complete_application/
├── 17_best_practices/
├── 18_channels_and_spawn/
└── examples-common/          # Shared utilities
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
8. 11_client_operations

### Advanced
9. 08_versioning
10. 14_local_activities

### Production
11. 13_worker_configuration
12. 15_performance
13. 16_complete_application
14. 17_best_practices
15. 18_channels_and_spawn
