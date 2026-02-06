//! # Example 17: Worker Configuration
//!
//! This example demonstrates worker setup and tuning options.
//!
//! ## Features Demonstrated
//!
//! - Worker options and configuration patterns
//! - Activity and workflow registration
//! - Rate limiting and concurrency controls
//! - Sticky execution configuration
//! - Worker identity and metrics
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p worker_configuration
//! ```

use cadence_worker::registry::WorkflowRegistry;
use examples_common::tracing_setup::init_tracing;
use worker_configuration::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Worker Configuration Example ===\n");
    println!("This example demonstrates:");
    println!("1. Worker options for different scenarios");
    println!("2. Activity and workflow registration");
    println!("3. Concurrency and rate limiting controls");
    println!("4. Sticky execution settings");
    println!("5. Worker identity configuration");
    println!();

    // Demonstrate different worker configurations
    demonstrate_worker_options();

    // Demonstrate registry setup
    demonstrate_registry();

    // Demonstrate worker setup
    demonstrate_worker_setup();

    println!("\nExample completed successfully!");
    println!("\nTo run tests:");
    println!("  cargo test -p worker_configuration");

    Ok(())
}

fn demonstrate_worker_options() {
    println!("\n--- Worker Options Demonstration ---\n");

    // High performance options
    let high_perf = create_high_performance_options();
    println!("High Performance Worker Configuration:");
    print_worker_config(&high_perf);

    // Development options
    let dev_options = create_development_options();
    println!("\nDevelopment Worker Configuration:");
    print_worker_config(&dev_options);

    // Limited resources options
    let limited = create_limited_resources_options();
    println!("\nResource-Limited Worker Configuration:");
    print_worker_config(&limited);

    println!("\nConfiguration Summary:");
    println!("  High Performance: For production with plenty of resources");
    println!("  Development: For debugging and testing (sticky disabled)");
    println!("  Resource-Limited: For memory-constrained environments");
}

fn demonstrate_registry() {
    println!("\n--- Registry Demonstration ---\n");

    let registry = WorkflowRegistry::new();

    println!("Registering workflows and activities...");
    register_workflows_and_activities(&registry);

    // Show registered items (conceptual - would need proper trait implementations)
    // let workflows = registry.get_registered_workflows();
    // let activities = registry.get_registered_activities();

    println!("\nRegistered Items:");
    println!("  Workflows: <requires trait wrappers>");
    // for wf in &workflows {
    //     println!("    - {}", wf.name);
    // }

    println!("  Activities: <requires trait wrappers>");
    // for act in &activities {
    //     println!("    - {}", act.name);
    // }

    println!("\nRegistry Patterns:");
    println!("  1. Register by name: registry.register_activity(\"name\", func)");
    println!("  2. Get by name: registry.get_activity(\"name\")");
    println!("  3. List all: registry.get_registered_activities()");
}

fn demonstrate_worker_setup() {
    println!("\n--- Worker Setup Demonstration ---\n");

    let options = create_development_options();
    setup_worker("example-domain", "example-task-list", options.clone());

    println!("Worker Setup Complete:");
    println!("  Domain: example-domain");
    println!("  Task List: example-task-list");
    println!("  Identity: {}", options.identity);

    println!("\nWorker Lifecycle:");
    println!("  1. Create: CadenceWorker::new(domain, task_list, options, registry)");
    println!("  2. Start: worker.start() - Non-blocking");
    println!("  3. Run: worker.run() - Blocking");
    println!("  4. Stop: worker.stop() - Graceful shutdown");

    println!("\nWorker Best Practices:");
    println!("  - Match task lists between client and worker");
    println!("  - Set appropriate concurrency limits");
    println!("  - Configure reasonable timeouts");
    println!("  - Use sticky execution for performance (prod)");
    println!("  - Disable sticky execution for debugging (dev)");
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::TestWorkflowEnvironment;
//
//     #[test]
//     fn test_high_performance_options() {
//         let opts = create_high_performance_options();
//
//         assert_eq!(opts.max_concurrent_activity_execution_size, 2000);
//         assert_eq!(opts.max_concurrent_decision_task_pollers, 4);
//         assert!(!opts.disable_sticky_execution);
//     }
//
//     #[test]
//     fn test_development_options() {
//         let opts = create_development_options();
//
//         assert_eq!(opts.max_concurrent_activity_execution_size, 100);
//         assert!(opts.disable_sticky_execution);
//     }
//
//     #[test]
//     fn test_limited_resources_options() {
//         let opts = create_limited_resources_options();
//
//         assert_eq!(opts.max_concurrent_activity_execution_size, 50);
//         assert_eq!(opts.worker_activities_per_second, 10.0);
//     }
//
//     #[test]
//     fn test_registry_registration() {
//         let registry = WorkflowRegistry::new();
//         register_workflows_and_activities(&registry);
//
//         let workflows = registry.get_registered_workflows();
//         let activities = registry.get_registered_activities();
//
//         assert_eq!(workflows.len(), 2);
//         assert_eq!(activities.len(), 2);
//
//         assert!(registry.get_workflow("data_processing").is_some());
//         assert!(registry.get_activity("compute").is_some());
//     }
//
//     #[tokio::test]
//     async fn test_data_processing_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("compute", compute_activity);
//         env.register_activity("process_data", process_data_activity);
//         env.register_workflow("data_processing", data_processing_workflow);
//
//         let input = DataProcessingWorkflowInput {
//             dataset_id: "dataset-001".to_string(),
//             compute_operations: vec!["sum".to_string(), "avg".to_string()],
//             batch_size: 100,
//         };
//
//         let result = env.execute_workflow("data_processing", input).await;
//         assert!(result.is_ok());
//     }
//
//     #[tokio::test]
//     async fn test_simple_compute_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//
//         env.register_activity("compute", compute_activity);
//         env.register_workflow("simple_compute", simple_compute_workflow);
//
//         let input = SimpleComputeInput {
//             values: vec![1.0, 2.0, 3.0, 4.0, 5.0],
//         };
//
//         let result = env.execute_workflow("simple_compute", input).await;
//         assert!(result.is_ok());
//
//         let output = result.unwrap();
//         assert_eq!(output.sum, 15.0);
//         assert_eq!(output.average, 3.0);
//         assert_eq!(output.min, 1.0);
//         assert_eq!(output.max, 5.0);
//     }
// }
