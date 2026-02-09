# Cadence Rust Client - Architecture

## Overview

This is a complete Rust port of the [Uber Cadence Go client](https://github.com/uber-go/crabdance_client), providing workflow orchestration capabilities with idiomatic Rust APIs. The implementation follows the Go client's architecture while adapting to Rust's type system and async ecosystem.

## High-Level Architecture

The Cadence Rust Client is organized as a Cargo workspace with seven main crates, each with distinct responsibilities:

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Application                         │
└────────────┬──────────────────────────────────┬─────────────────┘
             │                                   │
             ▼                                   ▼
    ┌────────────────┐                  ┌────────────────┐
    │ crabdance_client │                  │ crabdance_worker │
    └────────┬───────┘                  └────────┬───────┘
             │                                   │
             │    ┌──────────────────────┐       │
             └───►│   crabdance_workflow   │◄──────┘
                  └──────────┬───────────┘
                             │
             ┌───────────────┼───────────────┐
             │               │               │
             ▼               ▼               ▼
    ┌────────────────┐ ┌─────────────┐ ┌────────────────┐
    │ crabdance_core   │ │crabdance_proto│ │crabdance_activity│
    └────────────────┘ └─────────────┘ └────────────────┘
                             │
                             ▼
                    ┌────────────────┐
                    │ Cadence Server │
                    │   (gRPC API)   │
                    └────────────────┘
```

## Crate Responsibilities

### `crabdance_proto`

**Purpose**: Protocol layer for Cadence server communication

**Key Modules**:
- `generated/` - Auto-generated gRPC/Protobuf code (from `.proto` files)
- `shared.rs` - Hand-written wrapper types around generated types
- `workflow_service.rs` - Workflow service client and request/response types
- `conversions/` - Type conversions between API types and protobuf types

**Dependencies**: `tonic`, `prost`, `prost-types`

**Notes**: This crate bridges the gap between the raw protobuf definitions and idiomatic Rust types. The `generated` module is auto-generated during build via `build.rs` using `tonic-build`.

### `crabdance_core`

**Purpose**: Foundational types, error handling, and serialization

**Key Modules**:
- `error.rs` - Error types (`CadenceError`, `CustomError`, `TimeoutError`, etc.)
- `types.rs` - Core domain types (workflow execution, options, retry policies, etc.)
- `encoded.rs` - Data serialization/deserialization framework (`DataConverter`, `Payload`)

**Key Types**:
- `CadenceResult<T>` - Standard result type used throughout the codebase
- `WorkflowExecution` - Identifies a workflow instance (workflow_id + run_id)
- `RetryPolicy` - Configures retry behavior for activities/workflows
- `DataConverter` - Pluggable serialization (JSON, binary, etc.)

**Dependencies**: `serde`, `serde_json`, `thiserror`

**Notes**: This is the foundation layer - no dependencies on other Cadence crates.

### `crabdance_client`

**Purpose**: Client API for starting and managing workflows

**Key Modules**:
- `client.rs` - Main `Client` struct for workflow operations
- `domain.rs` - Domain management client (`DomainClient`)
- `grpc.rs` - gRPC transport layer, connection management, retries
- `auth/` - Authentication providers
  - `provider.rs` - `AuthProvider` trait
  - `jwt.rs` - JWT authentication implementation
  - `interceptor.rs` - gRPC interceptor for auth headers

**Key Operations**:
- Start, signal, query, cancel, terminate workflows
- Get workflow history and execution details
- List and scan workflows
- Domain registration, update, describe, deprecate, failover

**Dependencies**: `crabdance_proto`, `crabdance_core`, `tonic`, `tokio`

**Notes**: Implements connection pooling, automatic retries, and error mapping from gRPC to `CadenceError`.

### `crabdance_worker`

**Purpose**: Worker infrastructure for executing workflows and activities

**Key Modules**:
- `worker.rs` - Main `Worker` struct, orchestrates pollers and executors
- `registry.rs` - Workflow and activity function registration
- `pollers.rs` - Long-poll task queues for decision and activity tasks
- `executor/` - Task execution engines
  - `workflow.rs` - Workflow execution engine (replay-based)
  - `local_activity.rs` - Local activity executor
  - `replay.rs` - Replay logic for determinism
  - `cache.rs` - Workflow state cache (sticky execution)
- `handlers/` - Task processing logic
  - `decision.rs` - Decision task handler
  - `activity.rs` - Activity task handler
- `heartbeat.rs` - Activity heartbeat management
- `local_activity_queue.rs` - In-memory queue for local activities
- `replay_verifier.rs` - Replay verification for testing

**Key Concepts**:
- **Sticky Execution**: Workers cache workflow state for fast resumption
- **Pollers**: Long-poll Cadence server for decision/activity tasks
- **Registry**: Function registration maps workflow/activity names to implementations
- **Replay**: Workflows are re-executed from history to reconstruct state

**Dependencies**: `crabdance_client`, `crabdance_workflow`, `crabdance_activity`, `crabdance_core`, `tokio`

**Notes**: This is the most complex crate, handling concurrent task execution, state caching, and deterministic replay.

### `crabdance_workflow`

**Purpose**: SDK for authoring workflows

**Key Modules**:
- `context.rs` - `WorkflowContext` - API surface for workflow code
- `commands.rs` - Workflow commands (decisions to send to server)
- `state_machine.rs` - Decision state machine tracking decision lifecycle
- `dispatcher.rs` - Event dispatcher for replaying history
- `channel.rs` - Workflow channels for signal handling and inter-coroutine communication
- `future.rs` - Workflow futures for async operations
- `local_activity.rs` - Local activity execution (no server round-trip)
- `side_effect_serialization.rs` - Deterministic handling of non-deterministic operations

**Key Types**:
- `WorkflowContext` - Main API for workflow authors
  - Schedule activities (`schedule_activity`, `schedule_local_activity`)
  - Start child workflows (`execute_child_workflow`)
  - Timers (`sleep`, `new_timer`)
  - Signals/queries (`await_signal`, `set_query_handler`)
  - Side effects (`side_effect`, `mutable_side_effect`)
  - Versioning (`get_version`)

**Dependencies**: `crabdance_core`, `crabdance_proto`, `tokio`, `futures`

**Notes**: Workflows must be deterministic - the context ensures this by wrapping all non-deterministic operations.

### `crabdance_activity`

**Purpose**: SDK for authoring activities

**Key Modules**:
- `context.rs` - `ActivityContext` - API surface for activity code
  - Access input, task token, workflow info
  - Record heartbeats
  - Handle cancellation

**Dependencies**: `crabdance_core`, `tokio`

**Notes**: Activities can be non-deterministic and perform side effects (database calls, API requests, etc.).

### `crabdance_testsuite`

**Purpose**: Testing framework for workflows and activities

**Key Modules**:
- `suite.rs` - Test environment for unit testing workflows
  - `TestWorkflowEnvironment` - In-memory workflow execution
  - `TestActivityEnvironment` - Activity testing helpers
  - Mock implementations of workflow context

**Dependencies**: `crabdance_workflow`, `crabdance_worker`, `crabdance_core`

**Notes**: Allows testing workflows without a running Cadence server using replay verification.

## Key Design Patterns

### 1. Deterministic Execution via Replay

Workflows must be deterministic to enable reliable recovery and scaling. The replay mechanism:

1. **History Events**: All workflow decisions are recorded as events
2. **Replay**: On resumption, workers replay history to reconstruct state
3. **Decision Matching**: New decisions must match historical decisions
4. **Side Effect Wrapping**: Non-deterministic ops (time, random, I/O) are wrapped

**Implementation**:
- `crabdance_workflow/state_machine.rs` tracks decision states
- `crabdance_worker/executor/replay.rs` handles replay logic
- `WorkflowContext` enforces determinism by caching side effects

### 2. Sticky Execution for Performance

**Problem**: Replaying full history on every decision task is expensive

**Solution**: Cache workflow state on workers
- `crabdance_worker/executor/cache.rs` implements LRU cache (default 10K entries)
- Cadence server dispatches subsequent tasks to the same worker
- Only incremental history replay needed

**Trade-offs**: Requires memory, but drastically reduces CPU and latency

### 3. Long Polling Architecture

Workers use long-polling to receive tasks:

**Decision Task Flow**:
1. Worker polls decision task list (60s timeout)
2. Server returns task when workflow needs decision
3. Worker executes workflow, generates decisions
4. Worker sends decisions back, immediately polls again

**Activity Task Flow**: Similar to decision tasks but for activities

**Implementation**: `crabdance_worker/pollers.rs`

### 4. Type-Safe Error Handling

**Strategy**: Use `thiserror` for error definitions, `CadenceError` as unified error type

**Error Categories** (`crabdance_core/error.rs`):
- `Custom` - User-defined workflow errors
- `Timeout` - Various timeout types (start-to-close, heartbeat, etc.)
- `Canceled` - Workflow/activity cancellation
- `Terminated` - Workflow termination
- `Panic` - Workflow panic
- Transport errors (gRPC, serialization)

**Usage**: `CadenceResult<T>` is used throughout, enabling `?` operator

### 5. Pluggable Serialization

**Abstraction**: `DataConverter` trait in `crabdance_core/encoded.rs`

**Default**: JSON serialization via `serde_json`

**Extensibility**: Users can implement custom serializers (protobuf, MessagePack, etc.)

### 6. Authentication

**Strategy**: Pluggable auth via `AuthProvider` trait

**Implementations** (`crabdance_client/auth/`):
- `JwtAuthProvider` - JWT-based auth with automatic token refresh
- `AuthInterceptor` - gRPC interceptor to inject auth headers

## Data Flow Examples

### Starting a Workflow

```
User Code                 Client                 gRPC Layer            Server
   |                        |                        |                   |
   |--start_workflow()----->|                        |                   |
   |                        |--encode input--------->|                   |
   |                        |--StartWorkflowRequest->|------------------>|
   |                        |                        |<--Response--------|
   |<--WorkflowExecution----|                        |                   |
```

### Processing a Decision Task

```
Worker               Poller            Executor          Workflow Code
  |                    |                  |                    |
  |--poll_decision---->|----------------->|                    |
  |                    |<--DecisionTask---|                    |
  |                    |                  |--replay_history--->|
  |                    |                  |<--new_decisions----|
  |                    |--RespondDecision---------------------->|
```

### Executing an Activity

```
Workflow            State Machine       Server           Activity Worker
   |                     |                 |                    |
   |--schedule_activity->|                 |                    |
   |                     |--decision------>|                    |
   |                     |                 |--activity_task---->|
   |                     |                 |<--result-----------|
   |<--future_resolves---|<--event---------|                    |
```

## Concurrency Model

### Async Runtime: Tokio

All async operations use Tokio:
- `tokio::spawn` for task execution
- `tokio::sync` primitives (Mutex, RwLock, channels)
- `async-trait` for async trait methods

### Worker Concurrency

**Decision Tasks**: Configurable concurrent execution
- `max_concurrent_decision_task_execution_size` (default: 100)
- `max_concurrent_decision_task_pollers` (default: 2)

**Activity Tasks**: Separate concurrency controls
- `max_concurrent_activity_execution_size` (default: 100)
- `max_concurrent_activity_task_pollers` (default: 2)

**Rate Limiting**: Token bucket algorithm
- `worker_activities_per_second`
- `worker_decision_tasks_per_second`

### Thread Safety

Shared state uses:
- `Arc<T>` for shared ownership
- `RwLock<T>` / `Mutex<T>` for interior mutability
- `DashMap<K, V>` for concurrent hash maps

## Testing Strategy

### Unit Tests

**Location**: `<crate>/tests/` subdirectories

**Approach**: Test individual modules with mocked dependencies

**Tools**: `mockall` for mocking

### Integration Tests

**Location**: `<crate>/tests/` at crate root

**Requirements**: Running Cadence server (via Docker Compose)

**Marking**: Tests marked with `#[ignore]` (run explicitly)

**Execution**: `cargo test --test <test_name> -- --ignored`

### Workflow Testing

**Tool**: `crabdance_testsuite`

**Approach**: Replay-based testing without server
- Execute workflow with mock context
- Verify decisions produced
- Test with various history scenarios

## Performance Considerations

### Sticky Execution Cache

- **Size**: Default 10K workflow executions
- **Eviction**: LRU policy
- **Impact**: 10-100x latency reduction for hot workflows (unverified)

### gRPC Connection Pooling

- Reuse connections to Cadence server
- Configurable connection limits

### Serialization

- JSON is default (readable, debuggable)
- Binary formats (protobuf, MessagePack) available for performance

### Local Activities

- Execute within decision task (no server round-trip)
- Suitable for fast operations with no interactions with external services (pure calculation, <1s)
- Dramatically reduces latency

## Future Work / Limitations

### Not Yet Implemented

- Sessions (resource affinity for activities)
- Query consistency guarantees
- Workflow shadowing (side-by-side execution)
- Async activity completion (external completion)

### Known Limitations

- Workflow code must be Rust (no polyglot unlike Go client)
- No dynamic workflow registration (must be compile-time)
- Protobuf definitions must match server version

## References

- **Go Client**: [github.com/uber-go/crabdance_client](https://github.com/uber-go/crabdance_client) (reference implementation)
- **Cadence Docs**: [cadenceworkflow.io](https://cadenceworkflow.io)
- **Cadence Protocol**: `.proto` files in `proto/` directory
