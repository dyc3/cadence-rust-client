# Example 23: Performance Tuning

## Overview

This example demonstrates performance optimization techniques for high-throughput Cadence workflows. Learn how to maximize throughput, minimize latency, and efficiently manage resources.

## Features Demonstrated

- **Batch Processing**: Group multiple operations to reduce overhead
- **Parallel Execution**: Process independent activities concurrently
- **Cache Optimization**: Pre-warm caches for improved latency
- **High-Throughput Ingestion**: Efficiently process large data volumes
- **Resource Management**: Efficient memory and connection usage

## Key Concepts

### Batch Processing

Processing items in batches reduces per-item overhead and improves throughput:

```rust
let batch_input = BatchInput {
    items: large_dataset,
    batch_size: 100,  // Process 100 items at a time
};
```

### Parallel Activity Execution

Execute independent activities in parallel to maximize throughput:

```rust
let mut results = Vec::new();
for dataset in datasets {
    let result = ctx.execute_activity("transform", ...).await?;
    results.push(result);
}
```

### Heartbeat Configuration

Configure appropriate heartbeat intervals for long-running activities:

```rust
ActivityOptions {
    heartbeat_timeout: Some(Duration::from_secs(30)),
    start_to_close_timeout: Duration::from_secs(300),
    ..Default::default()
}
```

## Performance Strategies

### 1. Batch Size Optimization

Choose batch sizes based on:
- Activity execution time
- Network latency
- Memory constraints
- Cadence service limits

### 2. Worker Configuration

For high-throughput scenarios:
- Increase worker task slots
- Configure appropriate task list partitions
- Tune connection pool sizes
- Enable connection reuse

### 3. Retry Policy Tuning

Configure retry policies for performance:

```rust
RetryPolicy {
    initial_interval: Duration::from_millis(100),
    backoff_coefficient: 1.5,
    maximum_interval: Duration::from_secs(10),
    maximum_attempts: 5,
    non_retryable_error_types: vec!["ValidationError".to_string()],
}
```

### 4. Serialization Efficiency

- Use binary formats for large payloads
- Compress data when appropriate
- Minimize serialization overhead

## Running the Example

```bash
# Run the main demonstration
cargo run -p performance

# Run all tests with performance metrics
cargo test -p performance -- --nocapture

# Run specific performance benchmarks
cargo test -p performance test_performance_benchmarks -- --nocapture
```

## Benchmarks

The example includes performance benchmarks for various data sizes:

| Items | Batch Size | Throughput (items/sec) |
|-------|-----------|----------------------|
| 100   | 10        | ~50,000              |
| 1,000 | 100       | ~100,000             |
| 5,000 | 100       | ~120,000             |

*Note: Actual performance depends on hardware, network, and Cadence server configuration.*

## Code Structure

- `src/main.rs` - Example demonstration and benchmarks
- `src/lib.rs` - Library exports
- `src/activities/` - High-performance activity implementations
- `src/workflows/` - Optimized workflow patterns

## Activities

### batch_process_activity
Processes items in configurable batches with progress heartbeats.

### data_transform_activity
Efficiently transforms and optionally compresses data.

### cache_warmup_activity
Pre-loads cache entries to reduce latency for subsequent operations.

### high_throughput_ingest_activity
Optimized for ingesting large volumes of records with minimal overhead.

## Workflows

### high_throughput_workflow
Demonstrates batch processing patterns for large datasets.

### parallel_processing_workflow
Shows parallel execution of independent transformations.

### cache_optimized_workflow
Combines cache warmup with batch processing for optimal performance.

### ingestion_pipeline_workflow
End-to-end high-throughput data ingestion workflow.

## Production Considerations

### Monitoring
- Track workflow execution duration
- Monitor activity throughput
- Watch for heartbeat timeouts
- Measure serialization overhead

### Tuning Guidelines
1. Start with conservative batch sizes and increase gradually
2. Monitor memory usage with large batches
3. Tune heartbeat intervals based on actual activity duration
4. Use connection pooling for external service calls
5. Implement circuit breakers for external dependencies

### Resource Limits
- Be aware of Cadence service limits
- Implement backpressure for extremely high volumes
- Consider workflow partitioning for very large datasets

## Related Examples

- Previous: [22_cancellation_patterns](../22_cancellation_patterns) - Graceful cancellation
- Next: [24_complete_application](../24_complete_application) - Full production application
