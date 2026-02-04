//! # Example 26: Channels and Spawn
//!
//! This example demonstrates parallel workflow execution using channels and spawn.
//!
//! ## Features Demonstrated
//!
//! - Creating channels for communication between spawned tasks
//! - Using ctx.spawn() to execute tasks in parallel
//! - Split-merge (fan-out/fan-in) pattern
//! - Collecting results from parallel execution
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p channels_and_spawn
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p channels_and_spawn
//! ```

use channels_and_spawn::{Job, SplitMergeInput};
use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    init_tracing();

    println!("\n=== Cadence Rust Client - Channels and Spawn Example ===\n");
    println!("This example demonstrates:");
    println!("1. Using ctx.spawn() for parallel task execution");
    println!("2. Using channels for communication between tasks");
    println!("3. Split-merge (fan-out/fan-in) pattern");
    println!("4. Deterministic parallel execution for replay safety");
    println!();

    // Example input
    let input = SplitMergeInput {
        jobs: vec![
            Job {
                id: 1,
                data: "First job".to_string(),
            },
            Job {
                id: 2,
                data: "Second job".to_string(),
            },
            Job {
                id: 3,
                data: "Third job".to_string(),
            },
        ],
    };

    let serialized = serde_json::to_vec(&input)?;
    println!("Example input serialized: {} bytes", serialized.len());

    println!("\nKey Concepts:");
    println!("• Channels are ephemeral (not recorded to history)");
    println!("• Tasks execute cooperatively, not in OS-level parallel");
    println!("• Deterministic execution order ensures replay safety");
    println!("• All spawned tasks blocked = workflow yields to worker");

    println!("\nTo run the full workflow test:");
    println!("  cargo test -p channels_and_spawn");

    println!("\nExample completed successfully!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadence_testsuite::TestWorkflowEnvironment;
    use channels_and_spawn::{
        fast_process, parallel_workflow, process_job, split_merge_workflow, JobResult,
    };

    #[tokio::test]
    async fn test_split_merge_workflow() {
        init_tracing();

        let mut env = TestWorkflowEnvironment::new();

        // Register activities
        env.register_activity("process_job", process_job);
        env.register_activity("fast_process", fast_process);

        // Register workflow
        env.register_workflow("split_merge_workflow", split_merge_workflow);

        // Create input
        let input = SplitMergeInput {
            jobs: vec![
                Job {
                    id: 1,
                    data: "Job 1".to_string(),
                },
                Job {
                    id: 2,
                    data: "Job 2".to_string(),
                },
                Job {
                    id: 3,
                    data: "Job 3".to_string(),
                },
            ],
        };

        // Execute workflow
        let result = env
            .execute_workflow("split_merge_workflow", input)
            .await
            .expect("Workflow should complete successfully");

        // Verify results
        assert_eq!(result.results.len(), 3);
        assert!(result
            .results
            .iter()
            .any(|r| r.job_id == 1 && r.result.contains("Job 1")));
        assert!(result
            .results
            .iter()
            .any(|r| r.job_id == 2 && r.result.contains("Job 2")));
        assert!(result
            .results
            .iter()
            .any(|r| r.job_id == 3 && r.result.contains("Job 3")));

        println!("✓ Split-merge workflow test passed");
    }

    #[tokio::test]
    async fn test_parallel_workflow() {
        init_tracing();

        let mut env = TestWorkflowEnvironment::new();

        // Register activities
        env.register_activity("fast_process", fast_process);

        // Register workflow
        env.register_workflow("parallel_workflow", parallel_workflow);

        // Execute workflow
        let input = vec![1, 2, 3, 4, 5];
        let result = env
            .execute_workflow("parallel_workflow", input)
            .await
            .expect("Workflow should complete successfully");

        // Verify results (should be doubled)
        assert_eq!(result, vec![2, 4, 6, 8, 10]);

        println!("✓ Parallel workflow test passed");
    }
}
