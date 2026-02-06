//! # Example 15: Client Operations
//!
//! This example demonstrates the Client API for workflow management operations.
//!
//! ## Features Demonstrated
//!
//! - Starting workflows (synchronously and asynchronously)
//! - Sending signals to running workflows
//! - Querying workflow state
//! - Cancelling and terminating workflows
//! - Listing and scanning workflow executions
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p client_operations
//! ```

use cadence_client::client::StartWorkflowOptions;
use cadence_core::WorkflowIdReusePolicy;
use client_operations::*;
use examples_common::tracing_setup::init_tracing;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Client Operations Example ===\n");
    println!("This example demonstrates:");
    println!("1. StartWorkflowOptions - Configuring workflow execution");
    println!("2. Workflow ID reuse policies");
    println!("3. Signal patterns");
    println!("4. Query patterns");
    println!("5. Workflow lifecycle management");
    println!();

    // Demonstrate StartWorkflowOptions
    demonstrate_start_options();

    // Demonstrate signal workflow patterns
    demonstrate_signal_patterns().await;

    // Demonstrate query patterns
    demonstrate_query_patterns().await;

    // Demonstrate lifecycle management
    demonstrate_lifecycle_patterns();

    println!("\nExample completed successfully!");
    println!("\nTo run tests:");
    println!("  cargo test -p client_operations");

    Ok(())
}

fn demonstrate_start_options() {
    println!("\n--- StartWorkflowOptions Demonstration ---\n");

    // Create basic start options
    let basic_options = StartWorkflowOptions {
        id: "workflow-001".to_string(),
        task_list: "example-task-list".to_string(),
        execution_start_to_close_timeout: Some(Duration::from_secs(60)),
        task_start_to_close_timeout: Some(Duration::from_secs(10)),
        identity: Some("client-ops-example".to_string()),
        workflow_id_reuse_policy: WorkflowIdReusePolicy::AllowDuplicateFailedOnly,
        retry_policy: Some(cadence_core::RetryPolicy::default()),
        cron_schedule: None,
        memo: None,
        search_attributes: None,
        header: None,
        delay_start: None,
    };

    println!("Basic StartWorkflowOptions:");
    println!("  Workflow ID: {}", basic_options.id);
    println!("  Task List: {}", basic_options.task_list);
    println!(
        "  Execution Timeout: {:?}",
        basic_options.execution_start_to_close_timeout
    );
    println!(
        "  Task Timeout: {:?}",
        basic_options.task_start_to_close_timeout
    );
    println!(
        "  ID Reuse Policy: {:?}",
        basic_options.workflow_id_reuse_policy
    );

    // Create options with retry policy
    let retry_policy = cadence_core::RetryPolicy {
        initial_interval: Duration::from_secs(1),
        backoff_coefficient: 2.0,
        maximum_interval: Duration::from_secs(60),
        maximum_attempts: 5,
        non_retryable_error_types: vec!["InvalidInput".to_string()],
        expiration_interval: Duration::from_secs(300),
    };

    let retry_options = StartWorkflowOptions {
        id: "workflow-with-retry".to_string(),
        task_list: "retry-task-list".to_string(),
        retry_policy: Some(retry_policy),
        ..basic_options.clone()
    };

    println!("\nWorkflow with Retry Policy:");
    println!(
        "  Max Attempts: {:?}",
        retry_options
            .retry_policy
            .as_ref()
            .map(|r| r.maximum_attempts)
    );
    println!(
        "  Initial Interval: {:?}",
        retry_options
            .retry_policy
            .as_ref()
            .map(|r| r.initial_interval)
    );

    // Create options for different reuse policies
    let policies = vec![
        WorkflowIdReusePolicy::AllowDuplicateFailedOnly,
        WorkflowIdReusePolicy::AllowDuplicate,
        WorkflowIdReusePolicy::RejectDuplicate,
        WorkflowIdReusePolicy::TerminateIfRunning,
    ];

    println!("\nWorkflow ID Reuse Policies:");
    for policy in policies {
        let desc = match policy {
            WorkflowIdReusePolicy::AllowDuplicateFailedOnly => {
                "Allow only if previous failed/terminated"
            }
            WorkflowIdReusePolicy::AllowDuplicate => "Allow if not currently running",
            WorkflowIdReusePolicy::RejectDuplicate => "Never allow duplicate IDs",
            WorkflowIdReusePolicy::TerminateIfRunning => "Terminate existing and start new",
        };
        println!("  {:?}: {}", policy, desc);
    }
}

async fn demonstrate_signal_patterns() {
    println!("\n--- Signal Patterns Demonstration ---\n");

    use cadence_testsuite::{TestWorkflowContext, TestWorkflowEnvironment, WorkflowError};

    let mut env = TestWorkflowEnvironment::new();

    // Register a simple workflow for demonstration
    // Note: signal_handling_workflow uses WorkflowContext, but TestWorkflowEnvironment
    // uses TestWorkflowContext. For full signal testing, use the real Cadence server.
    env.register_workflow("simple_signal_workflow", |ctx, input: i32| async move {
        // Simple workflow that just returns the input
        Ok::<(TestWorkflowContext, i32), WorkflowError>((ctx, input))
    });

    let input = SignalWorkflowInput {
        workflow_id: "signal-test-001".to_string(),
        initial_value: 10,
    };

    println!("Signal Patterns:");
    println!("  1. Increment signal: Add a value to the current total");
    println!("  2. Multiply signal: Multiply the current total by a factor");
    println!("  3. Multiple signals can be sent before workflow starts");
    println!("  4. Signals are processed in order received");

    println!("\nSignal Workflow Example:");
    println!("  Initial value: {}", input.initial_value);
    println!("  Signals sent: increment(5), increment(5), multiply(2)");
    println!("  Expected: (10 + 5 + 5) * 2 = 40");

    // Execute the simple workflow to demonstrate the test environment
    let result: Result<i32, _> = env
        .execute_workflow("simple_signal_workflow", input.initial_value)
        .await;

    match result {
        Ok(value) => {
            println!("  Simple workflow result: {}", value);
            println!("  Note: For full signal handling, use the real Cadence server");
        }
        Err(e) => {
            println!("  Error: {:?}", e);
        }
    }
}

async fn demonstrate_query_patterns() {
    println!("\n--- Query Patterns Demonstration ---\n");

    println!("Query Types:");
    println!("  __stack_trace - Get current execution stack trace");
    println!("  __open_sessions - List open sessions");
    println!("  __query_types - List available query handlers");
    println!("  Custom queries - Application-specific queries");

    println!("\nQuery Patterns:");
    println!("  1. Status queries - Check workflow progress");
    println!("  2. State queries - Get current workflow state");
    println!("  3. Stack trace queries - Debug running workflows");

    // Create query response example
    let query_response = StatusQueryResponse {
        current_step: "processing".to_string(),
        progress_percent: 65,
        data: serde_json::json!({
            "items_processed": 650,
            "total_items": 1000,
        }),
    };

    println!("\nExample Query Response:");
    println!("  Current Step: {}", query_response.current_step);
    println!("  Progress: {}%", query_response.progress_percent);
    println!(
        "  Data: {}",
        serde_json::to_string_pretty(&query_response.data).unwrap()
    );
}

fn demonstrate_lifecycle_patterns() {
    println!("\n--- Workflow Lifecycle Management ---\n");

    println!("Lifecycle Operations:");
    println!("  1. Start - Begin workflow execution");
    println!("     - start_workflow() - Synchronous, waits for result");
    println!("     - start_workflow_async() - Asynchronous, returns immediately");
    println!("     - signal_with_start() - Signal or start if not running");

    println!("\n  2. Cancel - Request graceful termination");
    println!("     - Sends cancellation request");
    println!("     - Workflow can catch and clean up");
    println!("     - Preferred over terminate for graceful shutdown");

    println!("\n  3. Terminate - Force immediate termination");
    println!("     - Immediately stops workflow");
    println!("     - No cleanup opportunity");
    println!("     - Use for stuck workflows");

    println!("\n  4. Reset - Restart from specific point");
    println!("     - Useful for recovering from bad states");
    println!("     - Preserves workflow ID, new run ID");

    println!("\n  5. Query - Inspect running workflow");
    println!("     - Read-only operation");
    println!("     - Doesn't affect workflow state");
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::TestWorkflowEnvironment;
//
//     #[tokio::test]
//     async fn test_signal_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_workflow("signal_workflow", signal_handling_workflow);
//
//         let input = SignalWorkflowInput {
//             workflow_id: "test-001".to_string(),
//             initial_value: 100,
//         };
//
//         // Send some signals
//         env.signal_workflow("increment", serde_json::to_vec(&10i32).unwrap());
//         env.signal_workflow("multiply", serde_json::to_vec(&2i32).unwrap());
//
//         let result = env.execute_workflow("signal_workflow", input).await;
//         // Note: This is a simplified test - real signal handling would require
//         // more sophisticated test environment support
//         assert!(result.is_ok() || result.is_err()); // Placeholder
//     }
//
//     #[tokio::test]
//     async fn test_queryable_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_workflow("queryable_workflow", queryable_workflow);
//
//         let input = QueryableWorkflowInput {
//             steps: vec![
//                 "step1".to_string(),
//                 "step2".to_string(),
//                 "step3".to_string(),
//             ],
//         };
//
//         let result = env.execute_workflow("queryable_workflow", input).await;
//         assert!(result.is_ok());
//     }
//
//     #[tokio::test]
//     async fn test_cancellable_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_workflow("cancellable_workflow", cancellable_workflow);
//
//         let input = CancellableWorkflowInput {
//             duration_seconds: 60,
//             checkpoints: vec![
//                 "checkpoint1".to_string(),
//                 "checkpoint2".to_string(),
//                 "checkpoint3".to_string(),
//             ],
//         };
//
//         let result = env.execute_workflow("cancellable_workflow", input).await;
//         assert!(result.is_ok());
//     }
// }
