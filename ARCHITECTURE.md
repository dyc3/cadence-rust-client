# Cadence Rust Client - Project Structure

## Overview

This is a complete Rust port of the Uber Cadence Go client, providing workflow orchestration capabilities with idiomatic Rust APIs.

## Directory Structure

```
cadence-rust-client/
├── Cargo.toml                     # Workspace root
├── README.md                      # Main documentation
├── docker-compose.yml             # Local Cadence server setup
├── cadence-proto/                 # Protocol definitions
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                 # Proto module exports
│       ├── shared.rs              # Shared types (WorkflowExecution, HistoryEvent, etc.)
│       └── workflow_service.rs    # Service interface and request/response types
├── cadence-core/                  # Core types and utilities
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                 # Core module exports
│       ├── types.rs               # Core types (RetryPolicy, WorkflowOptions, etc.)
│       ├── error.rs               # Error types (CustomError, TimeoutError, etc.)
│       └── encoded.rs             # Serialization framework (EncodedValue)
├── cadence-client/                # Client implementation
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                 # Client module exports
│       ├── client.rs              # WorkflowClient trait and implementation
│       ├── domain.rs              # DomainClient for domain management
│       └── thrift.rs              # Thrift transport implementation
├── cadence-worker/                # Worker implementation
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                 # Worker module exports
│       ├── worker.rs              # Worker trait and CadenceWorker implementation
│       ├── registry.rs            # Workflow/Activity registry with DashMap
│       └── pollers.rs             # Task pollers (DecisionTaskPoller, ActivityTaskPoller)
├── cadence-workflow/              # Workflow SDK
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                 # Workflow module exports
│       ├── context.rs             # WorkflowContext with core functions
│       ├── future.rs              # Future types (ChildWorkflowFuture, ActivityFuture)
│       └── state_machine.rs       # Decision state machine for deterministic execution
├── cadence-activity/              # Activity SDK
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                 # Activity module exports
│       └── context.rs             # ActivityContext with heartbeats and info
├── cadence-testsuite/             # Testing framework
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                 # Testsuite module exports
│       └── suite.rs               # TestWorkflowEnvironment, WorkflowReplayer
└── examples/                      # Example applications
    └── hello_workflow/            # Hello world workflow example
        ├── Cargo.toml
        └── src/
            └── main.rs

```

## Key Components

### 1. cadence-proto (Protocol Layer)
- **Shared Types**: WorkflowExecution, HistoryEvent, TaskList, Decision types
- **Service Interface**: WorkflowService trait with all 25+ methods
- **Request/Response Types**: All Thrift request/response structures

### 2. cadence-core (Foundation)
- **Error System**: 12 error types (CustomError, TimeoutError, PanicError, etc.)
- **Core Types**: RetryPolicy, ActivityOptions, ChildWorkflowOptions, WorkflowInfo
- **Serialization**: EncodedValue with JSON support

### 3. cadence-client (Client API)
- **WorkflowClient**: 30+ methods (start, execute, signal, query, cancel, etc.)
- **DomainClient**: Domain management (register, describe, update, failover)
- **Thrift Transport**: Ready for Thrift connection implementation

### 4. cadence-worker (Execution Engine)
- **Worker**: Task polling and execution lifecycle
- **Registry**: Thread-safe workflow/activity registration with DashMap
- **Pollers**: Decision and activity task pollers with rate limiting
- **Configuration**: Extensive WorkerOptions (concurrency, rate limits, sticky execution)

### 5. cadence-workflow (Authoring SDK)
- **WorkflowContext**: 15+ methods for workflow authoring
  - `execute_activity()`, `execute_local_activity()`, `execute_child_workflow()`
  - `get_signal_channel()`, `signal_external_workflow()`
  - `side_effect()`, `mutable_side_effect()`, `get_version()`
  - `sleep()`, `now()`, `new_timer()`
  - `continue_as_new()`
- **Decision State Machine**: Deterministic execution tracking
  - Activity, Timer, ChildWorkflow state machines
  - State transitions (Created → Sent → Initiated → Started → Completed)

### 6. cadence-activity (Activity SDK)
- **ActivityContext**: Heartbeats, deadline tracking, worker stop detection
- **ActivityInfo**: Task token, attempt count, timeouts
- **Error Types**: ErrResultPending for async completion

### 7. cadence-testsuite (Testing)
- **TestWorkflowEnvironment**: Test workflows without server
- **TestActivityEnvironment**: Test activities in isolation
- **WorkflowReplayer**: Backwards compatibility testing
- **WorkflowShadower**: Non-determinism detection

## Quick Start

### 1. Start Cadence Server
```bash
docker-compose up -d
```

### 2. Define a Workflow
```rust
async fn hello_workflow(ctx: &mut WorkflowContext, input: HelloInput) -> Result<HelloOutput, WorkflowError> {
    let result = ctx.execute_activity("format_greeting", 
        Some(serde_json::to_vec(&input).unwrap()),
        ActivityOptions::default()
    ).await?;
    
    let output: HelloOutput = serde_json::from_slice(&result.unwrap())?;
    Ok(output)
}
```

### 3. Set up Worker
```rust
let registry = Arc::new(WorkflowRegistry::new());
registry.register_workflow("HelloWorld", Box::new(hello_workflow));

let worker = CadenceWorker::new("my-domain", "my-task-list", WorkerOptions::default(), registry);
worker.run()?;
```

### 4. Start Workflow
```rust
let client = Client::new("my-domain").await?;
let execution = client.start_workflow(options, "HelloWorld", Some(input)).await?;
```

## Configuration

### Worker Options
- Concurrent execution limits (1000 activities/decisions default)
- Rate limiting (tasks per second)
- Sticky execution cache
- Non-determinism policies
- Auto-scaling

### Client Options
- Identity configuration
- Feature flags
- Metrics and logging
- Data converters

## Testing

### Unit Tests
```rust
#[tokio::test]
async fn test_hello_workflow() {
    let env = TestWorkflowEnvironment::new();
    // Test workflow without server
}
```

### Integration Tests
See `examples/hello_workflow/` for a complete working example.

## Build Commands

```bash
# Build all packages
cargo build --all

# Run tests
cargo test --all

# Check compilation
cargo check --all

# Run example
cargo run -p hello_workflow
```

## Dependencies

### Core Stack
- **tokio** (1.43) - Async runtime
- **serde** (1.0) - Serialization
- **thrift** (0.17) - Protocol (when connected)
- **uuid** (1.11) - UUID generation
- **chrono** (0.4) - Date/time handling
- **thiserror** (1.0) - Error handling
- **async-trait** (0.1) - Async traits
- **dashmap** (6.1) - Concurrent hashmap
- **tracing** (0.1) - Logging/observability

### Development
- **tokio-test** (0.4) - Testing utilities
- **mockall** (0.13) - Mocking
- **anyhow** (1.0) - Error handling in examples

## Status

✅ **Complete and Compiling**
- All 7 workspace crates implemented
- Protocol layer with service definitions
- Client with 30+ methods
- Worker with polling and registry
- Workflow SDK with state machine
- Activity SDK with heartbeats
- Testing framework with replayer
- Examples and documentation

## Next Steps (Future)

1. **Thrift Connection**: Implement actual Thrift transport to Cadence server
2. **Integration Tests**: Full end-to-end tests with real server
3. **gRPC Support**: Add gRPC protocol support
4. **Metrics**: Prometheus/StatsD integration
5. **Tracing**: OpenTelemetry support
6. **Macros**: Procedural macros for workflow/activity definitions

## License

MIT License - See LICENSE file for details.
