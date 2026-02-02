# Integration Tests

This directory contains integration tests for both the Thrift and gRPC Cadence client implementations.

## Prerequisites

To run these tests, you need a running Cadence server instance. The easiest way to set this up is using Docker Compose.

## Running the Tests

### 1. Start Cadence Server

```bash
docker compose up -d
```

Wait for the services to be fully ready (this can take 2-3 minutes):

```bash
# Check if services are running
docker compose ps

# Check Cadence logs
docker compose logs -f cadence
```

### 2. Run Integration Tests

The tests are marked with `#[ignore]` by default to prevent failures in CI environments where Cadence might not be available.

#### gRPC Integration Tests

Run all gRPC integration tests (sequentially to avoid domain cache issues):

```bash
cargo test --test grpc_integration -- --ignored --test-threads=1
```

Run a specific gRPC test:

```bash
cargo test --test grpc_integration test_grpc_connection -- --ignored --nocapture
```

**Important:** The gRPC tests must be run with `--test-threads=1` to avoid overwhelming the Cadence domain cache. Each test registers a unique domain and waits 1500ms for cache propagation before starting workflows.

#### Run All Integration Tests

```bash
cargo test --tests -- --ignored
```

### 3. Stop Cadence Server

```bash
docker-compose down
```

## Test Coverage

### Thrift Integration Tests

The Thrift integration tests cover:

- **Connection Testing**: Verify TCP connection to Cadence server on port 7933
- **Domain Management**: Register, describe, and update domains
- **Workflow Execution**: Start workflows, signal workflows, query workflows
- **Decision Tasks**: Poll for decision tasks
- **Activity Tasks**: Poll for activity tasks, record heartbeats
- **Workflow History**: Retrieve execution history
- **Listing Operations**: List open and closed workflow executions

### gRPC Integration Tests

The gRPC integration tests (`grpc_integration.rs`) cover:

- **Connection Testing**: Verify gRPC connection to Cadence server on port 7833
- **Domain Management**: Register and describe domains via gRPC
- **Workflow Execution**: Start workflows via gRPC
- **Workflow History**: Retrieve execution history via gRPC
- **Workflow Signals**: Send signals to running workflows
- **Workflow Queries**: Query workflow state (expects error without worker)
- **Listing Operations**: List open workflow executions with time filters

**All 7 gRPC tests are currently passing ✅**

#### Implementation Details

The gRPC tests verify the critical fix for Cadence's YARPC gRPC adapter:
- Uses standard `grpc-timeout` header (e.g., "60S") instead of custom `context-ttl-ms`
- Includes proper Cadence client identification headers
- Each test uses UUID-based unique domain/workflow names to avoid collisions
- 1500ms delay after domain registration allows for distributed cache propagation

### Port Differences

- **Thrift**: Uses port **7933** (TChannel protocol)
- **gRPC**: Uses port **7833** (gRPC protocol)

## Troubleshooting

### Connection Errors

If you see "Transport error: not open" or "missing TTL" errors:
1. Ensure Cadence server is fully started (check logs)
2. Wait longer for initialization (Cassandra + Cadence can take 3-5 minutes)
3. Verify ports are accessible:
   - gRPC: `telnet localhost 7833`
   - Thrift: `telnet localhost 7933`

### Domain Cache Errors

If you see "DefaultDomainCache encounter case where domain exists but cannot be loaded":
1. Run tests sequentially with `--test-threads=1`
2. The tests already include a 1500ms delay after domain registration
3. If issues persist, you may need to increase the delay in `register_domain_and_wait()` helper

### CQL Version Mismatch

If Cadence logs show CQL version errors with Cassandra:
- This is a known issue with certain Cassandra/Cadence version combinations
- Try using Cassandra 3.11.4 or adjusting the CQL version in the Cadence startup script

### Timeout Issues

If tests timeout:
- Increase the timeout in `ClientConfig` (default is 30 seconds)
- Check network connectivity
- Verify Cadence is not under heavy load

## Environment Variables

You can customize the test configuration using environment variables:

- `CADENCE_HOST`: Cadence server hostname (default: `localhost`)
- `CADENCE_PORT`: Cadence server port (default: `7833`)

Example:

```bash
CADENCE_HOST=my-cadence-server CADENCE_PORT=7833 cargo test --test grpc_integration -- --ignored
```

## Test Results

### Successful Tests

✅ `test_thrift_connection` - Verifies basic TCP connection to Cadence server

### Tests Requiring Running Cadence Server

The following tests require a fully initialized Cadence server:

**Thrift Tests:**
- `test_register_domain`
- `test_describe_domain`
- `test_start_workflow_execution`
- `test_signal_workflow_execution`
- `test_query_workflow`
- `test_poll_for_decision_task_timeout`
- `test_list_open_workflow_executions`
- `test_list_closed_workflow_executions`
- `test_get_workflow_execution_history`

**gRPC Tests (all passing ✅):**
- `test_grpc_connection`
- `test_register_and_describe_domain`
- `test_start_workflow_execution`
- `test_get_workflow_execution_history`
- `test_signal_workflow_execution`
- `test_query_workflow`
- `test_list_open_workflow_executions`

## CI/CD Integration

For CI/CD pipelines, you can:

1. Use a pre-existing Cadence instance
2. Start Cadence in the pipeline with health checks
3. Skip integration tests by not passing the `--ignored` flag

Example GitHub Actions workflow:

```yaml
- name: Start Cadence
  run: docker-compose up -d

- name: Wait for Cadence
  run: sleep 60

- name: Run Integration Tests
  run: cargo test --test grpc_integration -- --ignored --test-threads=1
```

## Key Accomplishments

### gRPC Client Implementation ✅

The gRPC client now successfully communicates with Cadence server using the correct headers:

1. **Fixed gRPC Timeout Header**: Changed from custom `context-ttl-ms` to standard `grpc-timeout` header with proper format (e.g., "60S")
2. **Added Cadence Client Headers**: Includes `cadence-client-name`, `cadence-client-library-version`, `cadence-client-feature-version`, and `cadence-caller-type`
3. **Added YARPC Headers**: Includes `rpc-service`, `rpc-caller`, and `rpc-encoding` for compatibility with Cadence's YARPC gRPC adapter

### All 7 gRPC Tests Passing ✅

- Basic connection and domain operations
- Workflow lifecycle (start, signal, query)
- History retrieval with proper event parsing
- Listing open workflows with time filters
- Proper error handling for expected failures (queries without workers)

The test suite uses UUID-based domain names and includes proper cache propagation delays to ensure reliability.
