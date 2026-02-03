# Agent Guidelines for Cadence Rust Client

This document provides guidelines for AI coding agents working on the Cadence Rust Client codebase.

## Conformance and Compatibility

- This implementation is based on the golang Cadence client. It has been cloned into `./cadence-go-client` for reference. It should be used as the source of truth for behavior and API surface.

## Build, Test, and Lint Commands

### Building
```bash
# Build all workspace crates
cargo build

# Build in release mode
cargo build --release

# Build a specific crate
cargo build -p cadence-core
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p cadence-core

# Run a specific test by name
cargo test test_name_here

# Run tests in a specific test file
cargo test --test grpc_integration

# Run integration tests (requires running Cadence server)
cargo test --test grpc_integration -- --ignored --test-threads=1

# Run integration tests with output
cargo test --test grpc_integration -- --ignored --nocapture --test-threads=1

# Run tests using just (alternative runner)
just test-grpc-integration
just test-ecommerce-saga
```

### Linting and Formatting
```bash
# Check code without building
cargo check

# Run Clippy lints
cargo clippy

# Run Clippy with all features
cargo clippy --all-features

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check
```

### Documentation
```bash
# Build documentation
cargo doc

# Build and open documentation
cargo doc --open

# Build docs for all features
cargo doc --all-features
```

## Code Style Guidelines

### Imports Ordering
1. Standard library imports (`std::`)
2. Third-party crate imports (e.g., `tokio::`, `serde::`)
3. Workspace crate imports (`cadence_core::`, `cadence_proto::`)
4. Local module imports (`crate::`)

Separate each group with a blank line. Use `cargo fmt` to automatically format imports.

Example:
```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use cadence_core::{CadenceError, CadenceResult, WorkflowExecution};
use cadence_proto::shared::WorkflowType;

use crate::registry::WorkflowRegistry;
```

### Naming Conventions
- **Types (structs, enums, traits)**: PascalCase (e.g., `WorkflowOptions`, `CadenceError`)
- **Functions, methods, variables**: snake_case (e.g., `start_workflow`, `workflow_id`)
- **Constants**: UPPER_SNAKE_CASE (e.g., `QUERY_TYPE_STACK_TRACE`)
- **Type parameters**: PascalCase, single letters preferred (e.g., `T`, `E`)
- **Error types**: Suffix with `Error` (e.g., `CustomError`, `TimeoutError`)
- **Result types**: Use `CadenceResult<T>` alias from `cadence_core`

### Types and Type Safety
- Use strong typing: avoid `Box<dyn std::error::Error>`, use `CadenceError` instead
- Use `Option<T>` and `Result<T, E>` for nullable/ fallible operations
- Use `#[derive(Debug, Clone)]` for most types
- Use `#[derive(Default)]` for configuration types with sensible defaults
- Document public types with doc comments (`///`)

### Error Handling
- Use `thiserror::Error` for defining error types
- Use `CadenceError` as the primary error type throughout the codebase
- Prefer `?` operator for error propagation
- Provide context with `map_err()` when converting errors
- Use error factory functions in `cadence_core::error::factory` module
- Match on specific error types using helper functions (e.g., `is_custom_error()`)

### Async/Await
- Use `async_trait::async_trait` for trait methods
- Prefer `tokio` runtime (already configured in workspace)
- Use `Arc<>` for shared state in async contexts
- Use `tokio::sync` primitives (Mutex, RwLock, channels) instead of std equivalents

### Module Structure
- Each crate should have a clear `lib.rs` that re-exports public modules
- Use `pub mod` to expose submodules
- Use `pub use` to re-export commonly used items at crate root
- Module documentation should start with `//!` (inner doc comments)

### Testing Conventions
- Unit tests go in `tests/` subdirectory of each crate
- Integration tests go in `tests/` at crate root
- Use `tokio::test` for async tests
- Mock external dependencies using `mockall` crate
- Integration tests requiring Cadence server should be marked `#[ignore]`
- Use descriptive test names: `test_<scenario>_<expected_behavior>()`

## Workspace Structure

The project is organized as a Cargo workspace with these crates:

| Crate | Purpose |
|-------|---------|
| `cadence-proto` | Protocol definitions and generated types |
| `cadence-core` | Core types, errors, serialization |
| `cadence-client` | Client for workflow operations |
| `cadence-worker` | Worker for hosting workflows/activities |
| `cadence-workflow` | Workflow authoring SDK |
| `cadence-activity` | Activity authoring SDK |
| `cadence-testsuite` | Testing utilities |

## Development Environment

### Running Cadence Server (for integration tests)
```bash
docker compose up -d
# Wait 2-3 minutes for services to be ready
docker compose ps
```

### Access Points
- Cadence gRPC: `http://localhost:7833`
- Cadence Web UI: `http://localhost:8088`

## Key Dependencies

- **Async**: `tokio` (v1.43), `futures`, `async-trait`
- **Serialization**: `serde`, `serde_json`
- **Protocol**: `tonic` (gRPC), `prost` (protobuf)
- **Error Handling**: `thiserror`, `anyhow`
- **Testing**: `mockall`, `tokio-test`

## Pre-commit Checklist

Before committing changes:
1. Run `cargo fmt` to ensure consistent formatting
2. Run `cargo clippy` to catch common issues
3. Run `cargo test` to ensure tests pass
4. Run `cargo doc` to ensure documentation builds
5. Check that integration tests compile (they require running server)
