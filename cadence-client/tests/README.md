# Thrift Integration Tests

This directory contains integration tests for the Thrift-based Cadence client implementation.

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

Run all integration tests:

```bash
cargo test --test thrift_integration -- --ignored
```

Run a specific test:

```bash
cargo test --test thrift_integration test_thrift_connection -- --ignored --nocapture
```

### 3. Stop Cadence Server

```bash
docker-compose down
```

## Test Coverage

The integration tests cover:

- **Connection Testing**: Verify TCP connection to Cadence server on port 7933
- **Domain Management**: Register, describe, and update domains
- **Workflow Execution**: Start workflows, signal workflows, query workflows
- **Decision Tasks**: Poll for decision tasks
- **Activity Tasks**: Poll for activity tasks, record heartbeats
- **Workflow History**: Retrieve execution history
- **Listing Operations**: List open and closed workflow executions

## Troubleshooting

### Connection Errors

If you see "Transport error: not open" errors:
1. Ensure Cadence server is fully started (check logs)
2. Wait longer for initialization (Cassandra + Cadence can take 3-5 minutes)
3. Verify port 7933 is accessible: `telnet localhost 7933`

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
- `CADENCE_PORT`: Cadence server port (default: `7933`)

Example:

```bash
CADENCE_HOST=my-cadence-server CADENCE_PORT=7933 cargo test --test thrift_integration -- --ignored
```

## Test Results

### Successful Tests

✅ `test_thrift_connection` - Verifies basic TCP connection to Cadence server

### Tests Requiring Running Cadence Server

The following tests require a fully initialized Cadence server:
- `test_register_domain`
- `test_describe_domain`
- `test_start_workflow_execution`
- `test_signal_workflow_execution`
- `test_query_workflow`
- `test_poll_for_decision_task_timeout`
- `test_list_open_workflow_executions`
- `test_list_closed_workflow_executions`
- `test_get_workflow_execution_history`

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
  run: cargo test --test thrift_integration -- --ignored
```
