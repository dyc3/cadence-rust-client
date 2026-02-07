# Cadence Rust Client

A Rust client library for [Cadence](https://cadenceworkflow.io/), Uber's distributed, scalable, durable, and highly available orchestration engine.

This is a port of the official [Cadence Go client](https://github.com/uber-go/uber_cadence_client) to Rust, providing idiomatic Rust APIs for workflow orchestration.

## Overview

The Cadence Rust Client provides:

- **Workflow Client** - Start, query, signal, and manage workflow executions
- **Domain Client** - Manage Cadence domains (register, describe, update, failover)
- **Worker** - Host and execute workflow and activity implementations
- **Workflow SDK** - Author workflows with deterministic execution guarantees
- **Activity SDK** - Implement activities with heartbeats and cancellation support
- **Testing Framework** - Unit test workflows and activities without a real server

## Architecture

The project is organized as a Cargo workspace with the following crates:

| Crate | Description |
|-------|-------------|
| `uber_cadence_proto` | Protocol definitions (Thrift/Protobuf) and generated types |
| `uber_cadence_core` | Core types, error handling, and serialization |
| `uber_cadence_client` | Client implementation for workflow operations |
| `uber_cadence_worker` | Worker for hosting workflow and activity executions |
| `uber_cadence_workflow` | Workflow authoring SDK with deterministic execution |
| `uber_cadence_activity` | Activity authoring SDK |
| `uber_cadence_testsuite` | Testing utilities and workflow replayer |

## Quick Start

### Setting up a Worker

```rust
use uber_cadence_worker::{CadenceWorker, WorkerOptions, WorkflowRegistry};
use uber_cadence_workflow::WorkflowContext;
use std::sync::Arc;

// Define a simple workflow
fn hello_world_workflow(ctx: &mut WorkflowContext) -> Result<Vec<u8>, WorkflowError> {
    let name = String::from_utf8(ctx.get_input().unwrap_or_default())
        .unwrap_or_else(|_| "World".to_string());
    
    Ok(format!("Hello, {}!", name).into_bytes())
}

// Define an activity
fn greet_activity(ctx: &ActivityContext) -> Result<Vec<u8>, ActivityError> {
    let name = String::from_utf8(ctx.get_input().unwrap_or_default())
        .unwrap_or_else(|_| "World".to_string());
    
    Ok(format!("Greetings, {}!", name).into_bytes())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a registry and register workflows/activities
    let registry = Arc::new(WorkflowRegistry::new());
    registry.register_workflow("HelloWorld", Box::new(hello_world_workflow));
    registry.register_activity("Greet", Box::new(greet_activity));
    
    // Configure worker options
    let options = WorkerOptions {
        max_concurrent_activity_execution_size: 100,
        max_concurrent_decision_task_execution_size: 100,
        identity: "my-worker".to_string(),
        ..Default::default()
    };
    
    // Create and start the worker
    let worker = CadenceWorker::new("my-domain", "my-task-list", options, registry);
    worker.run()?;
    
    Ok(())
}
```

### Macro-based Workflows and Activities

```rust
use uber_cadence_activity::{activity, ActivityContext};
use uber_cadence_worker::{CadenceWorker, WorkerOptions, WorkflowRegistry};
use uber_cadence_workflow::{call_activity, workflow, WorkflowContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
struct WelcomeInput {
    email: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmailRequest {
    to: String,
    subject: String,
    body: String,
}

#[activity(name = "send_email")]
async fn send_email(_ctx: &ActivityContext, input: EmailRequest) -> Result<(), uber_cadence_worker::ActivityError> {
    println!("Sending to {}", input.to);
    Ok(())
}

#[workflow(name = "welcome_flow")]
async fn welcome_flow(ctx: WorkflowContext, input: WelcomeInput) -> Result<(), uber_cadence_worker::WorkflowError> {
    let options = uber_cadence_core::ActivityOptions {
        schedule_to_close_timeout: Duration::from_secs(30),
        schedule_to_start_timeout: Duration::from_secs(30),
        start_to_close_timeout: Duration::from_secs(30),
        heartbeat_timeout: Duration::from_secs(0),
        ..Default::default()
    };

    let request = EmailRequest {
        to: input.email,
        subject: "Welcome".to_string(),
        body: format!("Hi {}, welcome!", input.name),
    };

    let _: () = call_activity!(ctx, send_email, request, options).await?;
    Ok(())
}

fn register_all(registry: &dyn uber_cadence_worker::Registry) {
    welcome_flow_cadence::register(registry);
    send_email_cadence::register(registry);
}

#[tokio::main]
async fn main() -> Result<(), uber_cadence_core::CadenceError> {
    let registry = Arc::new(WorkflowRegistry::new());
    register_all(registry.as_ref());

    let worker = CadenceWorker::new(
        "my-domain",
        "my-task-list",
        WorkerOptions::default(),
        registry,
    );
    worker.start()?;
    Ok(())
}
```

### Starting a Workflow

```rust
use uber_cadence_client::{Client, StartWorkflowOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = Client::new("my-domain", client_options).await?;
    
    // Start a workflow
    let options = StartWorkflowOptions {
        id: "my-workflow-123".to_string(),
        task_list: "my-task-list".to_string(),
        ..Default::default()
    };
    
    let execution = client
        .start_workflow(options, "HelloWorld", Some(b"Alice".to_vec()))
        .await?;
    
    println!("Started workflow: {:?}", execution);
    
    // Query the workflow
    let result = client
        .query_workflow("my-workflow-123", None, "__stack_trace", None)
        .await?;
    
    Ok(())
}
```

### Testing Workflows

```rust
use uber_cadence_testsuite::{TestWorkflowEnvironment, TestActivityEnvironment};

#[tokio::test]
async fn test_hello_world_workflow() {
    let mut env = TestWorkflowEnvironment::new();
    
    // Register workflow
    env.register_workflow("HelloWorld", Box::new(hello_world_workflow));
    
    // Execute and verify
    let result = env.execute_workflow(|ctx| {
        hello_world_workflow(ctx)
    }).await;
    
    assert_eq!(result.unwrap(), b"Hello, World!");
}
```

## Features

### Core Workflow Features

- ✅ Workflow execution with deterministic guarantees
- ✅ Activity scheduling with retry policies
- ✅ Child workflow execution
- ✅ Signals and queries
- ✅ Timers and delays
- ✅ Side effects and mutable side effects
- ✅ Versioning for backwards compatibility
- ✅ Search attributes
- ✅ Cancellation support

### Worker Features

- ✅ Concurrent task execution
- ✅ Rate limiting
- ✅ Sticky execution for performance
- ✅ Auto-scaling of pollers
- ✅ Heartbeat support for long-running activities
- ✅ Session management
- ✅ Interceptor chain for cross-cutting concerns

### Client Features

- ✅ Start, execute, and get workflows
- ✅ Signal and query workflows
- ✅ Cancel and terminate workflows
- ✅ List and scan workflows
- ✅ Domain management
- ✅ Async activity completion
- ✅ History retrieval

## Configuration

### Worker Options

```rust
use uber_cadence_worker::WorkerOptions;
use std::time::Duration;

let options = WorkerOptions {
    // Concurrency limits
    max_concurrent_activity_execution_size: 1000,
    max_concurrent_decision_task_execution_size: 1000,
    
    // Rate limiting (tasks per second)
    worker_activities_per_second: 100_000.0,
    worker_decision_tasks_per_second: 100_000.0,
    
    // Poller configuration
    max_concurrent_decision_task_pollers: 2,
    max_concurrent_activity_task_pollers: 2,
    
    // Sticky execution (caches workflow state)
    disable_sticky_execution: false,
    sticky_schedule_to_start_timeout: Duration::from_secs(5),
    
    // Non-determinism handling
    non_deterministic_workflow_policy: NonDeterministicWorkflowPolicy::BlockWorkflow,
    
    ..Default::default()
};
```

### Client Options

```rust
use uber_cadence_client::ClientOptions;

let options = ClientOptions {
    identity: "my-client".to_string(),
    feature_flags: FeatureFlags {
        enable_execution_cache: true,
        enable_async_workflow_consistency: false,
    },
    ..Default::default()
};
```

## Error Handling

The client uses a comprehensive error system:

```rust
use uber_cadence_core::{CadenceError, CustomError, TimeoutError};

match result {
    Err(CadenceError::Custom(e)) => {
        println!("Workflow error: {}", e.reason());
    }
    Err(CadenceError::Timeout(e)) => {
        println!("Timeout: {:?}", e.timeout_type());
    }
    Err(CadenceError::Canceled(_)) => {
        println!("Workflow was canceled");
    }
    _ => {}
}
```

## Architecture Highlights

### Deterministic Execution

Workflows in Cadence must be deterministic - they must produce the same results when replayed. The Rust client achieves this through:

1. **Event Sourcing** - All decisions (activities, timers, etc.) are recorded in the workflow history
2. **Replay-Based Execution** - Workers replay history events to reconstruct workflow state
3. **Side Effect Wrappers** - Non-deterministic operations (random, time, I/O) are wrapped and cached
4. **Decision State Machine** - Tracks the state of each decision through its lifecycle

### Sticky Execution

For performance, workflow state is cached on workers:

- Workers maintain an LRU cache of workflow executions (default 10K entries)
- Subsequent decision tasks for the same workflow are dispatched to the same worker
- This avoids replaying the entire history for every decision task

### Task Polling

Workers use long-polling to receive tasks:

- Decision task pollers poll the decision task list
- Activity task pollers poll the activity task list
- Supports both normal and sticky task lists
- Rate limiting prevents overwhelming the server

## License

MIT License - See [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Acknowledgments

This project is a port of the [Cadence Go Client](https://github.com/uber-go/uber_cadence_client) by Uber.
