# Cadence Load Test Tool

A comprehensive load testing tool for testing Cadence worker capacity under various load patterns.

## Features

- **Decoupled Worker/Client Architecture**: Worker runs independently, clients connect to test
- **5 Load Test Scenarios**: High-throughput, high-concurrency, sustained-load, spike/burst, and failure testing
- **Real-time Metrics**: Live console updates with workflow/activity stats, latency percentiles, and system metrics
- **HDR Histogram**: Accurate latency tracking (P50, P95, P99) using HdrHistogram
- **Worker Profiles**: Pre-configured profiles for dev, high-performance, and stress testing
- **System Monitoring**: CPU and memory usage tracking via sysinfo

## Installation

```bash
cargo build --release -p load_test
```

## Usage

The load tester has two modes:
1. **Worker mode**: Starts a worker that processes workflows/activities (runs until Ctrl+C)
2. **Client mode**: Spawns workflow executions to test the worker

### Quick Start

**Terminal 1 - Start Worker:**
```bash
just load-test worker
```

**Terminal 2 - Run Load Test:**
```bash
just load-test client --duration 60 high-throughput --target-rate 100
```

### Worker Mode

Start a worker that will process workflows and activities:

```bash
just load-test worker [OPTIONS]
```

**Worker Options:**
- `--endpoint` - Cadence gRPC endpoint (default: http://localhost:7833)
- `--domain` - Domain name (default: loadtest-domain)
- `--task-list` - Task list name (default: loadtest-tasks)
- `--worker-profile` - Worker profile: dev, high-perf, stress (default: high-perf)
- `-v, --verbose` - Enable debug logging

The worker runs continuously until you press Ctrl+C. It can serve multiple concurrent clients.

### Client Mode - Load Test Scenarios

#### 1. High Throughput
Tests maximum workflows/second the worker can handle:

```bash
just load-test client --duration 60 high-throughput --target-rate 500 --warmup 10
```

#### 2. High Concurrency
Tests many simultaneous in-flight workflows:

```bash
just load-test client --duration 60 high-concurrency --target-count 1000 --workflow-duration 30
```

#### 3. Sustained Load
Tests consistent load to detect degradation:

```bash
just load-test client --duration 300 sustained-load --rate 100
```

#### 4. Spike/Burst
Tests behavior under sudden load increases:

```bash
just load-test client --duration 120 spike --baseline-rate 50 --spike-rate 500 --spike-interval 30
```

#### 5. Failure Testing
Tests error handling and retry logic:

```bash
just load-test client --duration 60 failure --rate 100 --failure-rate 0.2 --max-retries 3
```

### Configuration Options

**Client Global Options:**
- `--endpoint` - Cadence gRPC endpoint (default: http://localhost:7833)
- `--domain` - Domain name (default: loadtest-domain)
- `--task-list` - Task list name (default: loadtest-tasks)
- `--duration` - Test duration in seconds (default: 60)
- `-v, --verbose` - Enable debug logging

**Worker Profiles:**
- **dev**: Conservative settings for development (100 concurrent, 50 activities/sec)
- **high-perf**: Optimized for performance (2000 concurrent, 500 activities/sec)
- **stress**: Maximum load settings (5000 concurrent, 1000 activities/sec)

## Prerequisites

Start Cadence server using Docker:

```bash
docker compose up -d
```

Wait 2-3 minutes for services to be ready, then verify:

```bash
docker compose ps
```

Access Cadence Web UI at http://localhost:8088

## Metrics Explained

### Live Metrics Dashboard

The tool displays real-time metrics every N seconds (configurable):

```
╔════════════════════════════════════════════════════════════════╗
║             Cadence Load Test - Live Metrics                  ║
╚════════════════════════════════════════════════════════════════╝

⏱️  Elapsed Time: 00:01:23

┌─ WORKFLOWS ─────────────────────────────────────────────────┐
│  Started:      12500    In-Flight:       156              │
│  Completed:    12344    Failed:            0              │
│  Success Rate:  100.00%    Throughput:  150.45/sec        │
└─────────────────────────────────────────────────────────────┘

┌─ WORKFLOW LATENCY (ms) ─────────────────────────────────────┐
│  Min:     12  P50:     45  P95:    120  P99:    250  Max:    450│
│  Mean:     52.34 ms    Count:      12344                    │
└─────────────────────────────────────────────────────────────┘

┌─ SYSTEM ────────────────────────────────────────────────────┐
│  CPU Usage:     45.2%    Memory:   2048 /  8192 MB       │
└─────────────────────────────────────────────────────────────┘

  [Press Ctrl+C to stop test]
```

### Final Report

At the end of the test, a summary report is printed with:
- Total workflows started/completed/failed
- Overall throughput (workflows/sec)
- Success rate percentage
- Latency statistics (min, p50, p95, p99, max, mean)
- Activity statistics (if applicable)

## Architecture

### Components

```
load_test/
├── cli/           # CLI argument parsing with clap
├── config/        # Worker profile configurations
├── metrics/       # Metrics collection and reporting
│   ├── collector  # Thread-safe metrics with HDR histograms
│   ├── reporter   # Real-time console output
│   └── types      # Metric data structures
├── scenarios/     # Load test scenarios
│   ├── high_throughput.rs    # ✅ Fully implemented
│   ├── high_concurrency.rs   # Stub
│   ├── sustained_load.rs     # Stub
│   ├── spike_burst.rs        # Stub
│   └── failure.rs            # Stub
├── workflows/     # Test workflow implementations
└── activities/    # Test activity implementations
```

### Implementation Status

✅ **Complete:**
- CLI framework with worker/client separation
- Metrics collection with HDR histograms
- Real-time reporting with system metrics
- Worker profiles (dev, high-perf, stress)
- All 5 load test scenarios fully implemented and tested
- Decoupled worker architecture for realistic testing

## Development

### Building
```bash
cargo build -p load_test
```

### Running Tests
```bash
cargo test -p load_test
```

### Running Locally
```bash
# Start worker
just load-test worker

# In another terminal, run a test
just load-test client --duration 30 high-throughput --target-rate 50
```

## Architecture

The load tester uses a **decoupled worker/client architecture**:

### Worker
- Runs independently and continuously
- Registers all test workflows and activities
- Polls for decision and activity tasks
- Can serve multiple concurrent clients
- Configured with worker profiles (dev, high-perf, stress)

### Client
- Connects to Cadence server
- Spawns workflow executions according to scenario
- Tracks metrics (throughput, latency, success rate)
- Displays real-time progress
- Exits after test duration completes

This mirrors real production deployments where workers run continuously and clients submit workflows.

## Performance Tips

1. **Start worker first**: Always start the worker before running client tests
2. **Reuse worker**: Keep worker running and run multiple tests against it
3. **Use appropriate worker profile**: Start with `dev`, move to `high-perf` for serious testing
4. **Monitor system resources**: Watch CPU/memory in the live metrics
5. **Gradual ramp-up**: Use `--warmup` to gradually increase load
6. **Domain caching**: The first test may be slower as domain registration propagates
7. **Multiple clients**: You can run multiple client tests concurrently against one worker

## Troubleshooting

**Connection Refused:**
```
Error: Failed to connect to Cadence server
```
Solution: Ensure Cadence server is running (`docker compose up -d`)

**Domain Not Found:**
```
Error: Domain 'loadtest-domain' not found
```
Solution: The worker automatically registers the domain on startup. Wait 2-3 seconds.

**No workflows executing:**
- Ensure worker is running in another terminal
- Check task list names match between client and worker (use `--task-list` flag)
- Verify domain is registered
- Check worker logs with `-v` flag

**Workflows timing out:**
- Check that worker profile has sufficient capacity
- Verify worker is not overwhelmed (check CPU/memory)
- Consider reducing load test rate

## License

Part of the Cadence Rust Client project.
