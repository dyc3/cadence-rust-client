//! # Example 23: Performance Tuning
//!
//! This example demonstrates performance optimization techniques for high-throughput Cadence workflows.
//!
//! ## Features Demonstrated
//!
//! - Batched activity execution for improved throughput
//! - Parallel activity processing
//! - Memory-efficient data structures
//! - Worker configuration for high performance
//! - Heartbeat patterns for long-running activities
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run -p performance
//! ```
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p performance
//! ```

use performance::*;
use examples_common::tracing_setup::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    println!("\n=== Cadence Rust Client - Performance Tuning Example ===\n");
    println!("This example demonstrates:");
    println!("1. Batched activity execution");
    println!("2. Parallel processing patterns");
    println!("3. Cache warmup strategies");
    println!("4. High-throughput ingestion");
    println!("5. Worker optimization techniques");
    println!();
    println!("Performance Optimization Strategies:");
    println!("  - Batch Processing: Group operations to reduce overhead");
    println!("  - Parallel Execution: Process independent activities concurrently");
    println!("  - Resource Pooling: Reuse connections and buffers");
    println!("  - Heartbeat Management: Efficient progress reporting");
    println!();
    println!("Run tests to see performance benchmarks:");
    println!("  cargo test -p performance -- --nocapture");

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cadence_testsuite::TestWorkflowEnvironment;
// 
//     fn generate_test_items(count: usize) -> Vec<String> {
//         (0..count)
//             .map(|i| format!("item_{:08}", i))
//             .collect()
//     }
// 
//     fn generate_test_datasets(count: usize, size: usize) -> Vec<Vec<u8>> {
//         (0..count)
//             .map(|i| format!("dataset_{}", i).repeat(size).into_bytes())
//             .collect()
//     }
// 
//     #[tokio::test]
//     async fn test_batch_processing() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("batch_process", batch_process_activity::<String>);
//         env.register_workflow("high_throughput", high_throughput_workflow);
// 
//         let items = generate_test_items(1000);
//         let start = std::time::Instant::now();
// 
//         let result = env
//             .execute_workflow("high_throughput", items)
//             .await
//             .expect("Workflow should complete");
// 
//         let duration = start.elapsed();
//         let throughput = result.processed_count as f64 / duration.as_secs_f64();
// 
//         assert_eq!(result.processed_count, 1000);
//         assert_eq!(result.failed_count, 0);
//         
//         println!("\nBatch Processing Performance:");
//         println!("  Items processed: {}", result.processed_count);
//         println!("  Duration: {:?}", duration);
//         println!("  Throughput: {:.0} items/sec", throughput);
//         println!("  Average per item: {:.2} microseconds", 
//             duration.as_micros() as f64 / result.processed_count as f64);
//     }
// 
//     #[tokio::test]
//     async fn test_parallel_processing() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("data_transform", data_transform_activity);
//         env.register_workflow("parallel_processing", parallel_processing_workflow);
// 
//         let datasets = generate_test_datasets(10, 100);
//         let start = std::time::Instant::now();
// 
//         let results = env
//             .execute_workflow("parallel_processing", datasets)
//             .await
//             .expect("Workflow should complete");
// 
//         let duration = start.elapsed();
// 
//         assert_eq!(results.len(), 10);
//         
//         let total_original: usize = results.iter().map(|r| r.original_size).sum();
//         let total_transformed: usize = results.iter().map(|r| r.transformed_size).sum();
//         
//         println!("\nParallel Processing Performance:");
//         println!("  Datasets processed: {}", results.len());
//         println!("  Total original size: {} bytes", total_original);
//         println!("  Total transformed size: {} bytes", total_transformed);
//         println!("  Compression ratio: {:.1}%", 
//             100.0 * (1.0 - total_transformed as f64 / total_original as f64));
//         println!("  Duration: {:?}", duration);
//     }
// 
//     #[tokio::test]
//     async fn test_cache_optimized_workflow() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("cache_warmup", cache_warmup_activity);
//         env.register_activity("batch_process", batch_process_activity::<String>);
//         env.register_workflow("cache_optimized", cache_optimized_workflow);
// 
//         let keys = generate_test_items(500);
//         let start = std::time::Instant::now();
// 
//         let (warmed, processed) = env
//             .execute_workflow("cache_optimized", keys)
//             .await
//             .expect("Workflow should complete");
// 
//         let duration = start.elapsed();
// 
//         assert_eq!(warmed, 500);
//         assert_eq!(processed, 500);
//         
//         println!("\nCache-Optimized Workflow Performance:");
//         println!("  Keys warmed: {}", warmed);
//         println!("  Items processed: {}", processed);
//         println!("  Total duration: {:?}", duration);
//     }
// 
//     #[tokio::test]
//     async fn test_high_throughput_ingestion() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("high_throughput_ingest", high_throughput_ingest_activity);
//         env.register_workflow("ingestion_pipeline", ingestion_pipeline_workflow);
// 
//         let records = generate_test_items(10000);
//         let start = std::time::Instant::now();
// 
//         let ingested = env
//             .execute_workflow("ingestion_pipeline", records)
//             .await
//             .expect("Workflow should complete");
// 
//         let duration = start.elapsed();
//         let throughput = ingested as f64 / duration.as_secs_f64();
// 
//         assert_eq!(ingested, 10000);
//         
//         println!("\nHigh-Throughput Ingestion Performance:");
//         println!("  Records ingested: {}", ingested);
//         println!("  Duration: {:?}", duration);
//         println!("  Throughput: {:.0} records/sec", throughput);
//         println!("  Average per record: {:.2} microseconds", 
//             duration.as_micros() as f64 / ingested as f64);
//     }
// 
//     #[tokio::test]
//     async fn test_individual_activities() {
//         let mut env = TestWorkflowEnvironment::new();
//         env.register_activity("batch_process", batch_process_activity::<String>);
//         env.register_activity("data_transform", data_transform_activity);
//         env.register_activity("cache_warmup", cache_warmup_activity);
//         env.register_activity("high_throughput_ingest", high_throughput_ingest_activity);
// 
//         // Test batch processing
//         let batch_input = BatchInput {
//             items: generate_test_items(100),
//             batch_size: 10,
//         };
//         
//         let result = env
//             .execute_activity("batch_process", batch_input)
//             .await
//             .expect("Batch process should succeed");
//         
//         let batch_result: BatchResult = serde_json::from_slice(&result)
//             .expect("Should parse result");
//         
//         assert_eq!(batch_result.processed_count, 100);
//         assert_eq!(batch_result.failed_count, 0);
// 
//         // Test data transform
//         let transform_input = TransformInput {
//             data: vec![0u8; 1000],
//             compression_level: 50,
//         };
//         
//         let result = env
//             .execute_activity("data_transform", transform_input)
//             .await
//             .expect("Transform should succeed");
//         
//         let transform_result: TransformOutput = serde_json::from_slice(&result)
//             .expect("Should parse result");
//         
//         assert_eq!(transform_result.original_size, 1000);
// 
//         // Test cache warmup
//         let warmup_input = CacheWarmupInput {
//             keys: generate_test_items(50),
//             preload_size: 100,
//         };
//         
//         let result = env
//             .execute_activity("cache_warmup", warmup_input)
//             .await
//             .expect("Cache warmup should succeed");
//         
//         let warmed: usize = serde_json::from_slice(&result)
//             .expect("Should parse result");
//         
//         assert_eq!(warmed, 50);
// 
//         // Test high-throughput ingest
//         let records = generate_test_items(1000);
//         
//         let result = env
//             .execute_activity("high_throughput_ingest", records)
//             .await
//             .expect("Ingest should succeed");
//         
//         let ingested: usize = serde_json::from_slice(&result)
//             .expect("Should parse result");
//         
//         assert_eq!(ingested, 1000);
//     }
// 
//     #[tokio::test]
//     async fn test_performance_benchmarks() {
//         println!("\n=== Performance Benchmarks ===\n");
//         
//         let sizes = vec![100, 1000, 5000];
//         
//         for size in sizes {
//             let mut env = TestWorkflowEnvironment::new();
//             env.register_activity("batch_process", batch_process_activity::<String>);
//             env.register_workflow("high_throughput", high_throughput_workflow);
// 
//             let items = generate_test_items(size);
//             let start = std::time::Instant::now();
// 
//             let result = env
//                 .execute_workflow("high_throughput", items)
//                 .await
//                 .expect("Workflow should complete");
// 
//             let duration = start.elapsed();
//             let throughput = result.processed_count as f64 / duration.as_secs_f64();
// 
//             println!("Size {:5}: {:6} items in {:8.2?} => {:8.0} items/sec",
//                 size, result.processed_count, duration, throughput);
//         }
//     }
// }
