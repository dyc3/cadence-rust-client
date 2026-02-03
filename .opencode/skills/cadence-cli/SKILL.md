---
name: cadence-cli
description: Operate Cadence workflow engine - manage domains, workflows, tasklists, clusters, and administrative operations
license: MIT
compatibility: opencode
metadata:
  audience: developers
  domain: workflow-engine
---

## What I do

I help users interact with the Cadence workflow engine through its CLI. I can:

- **Domain operations**: Register, describe, update, deprecate, and delete domains
- **Workflow operations**: Start, query, describe, terminate, reset, and manage workflow executions
- **Tasklist operations**: Describe and poll tasklists
- **Admin operations**: Perform administrative tasks like database operations, DLQ management, and async workflows
- **Cluster operations**: Manage cluster metadata and health

## Common Commands

### Domain Management
```bash
# Register a new domain
./cadence domain register --domain <name> --owner_email <email> --description <desc>

# Describe domain details
./cadence domain describe --domain <name>

# Update domain configuration
./cadence domain update --domain <name> --retention_days <days>

# List all domains
./cadence domain list
```

### Workflow Operations
```bash
# Start a workflow
./cadence workflow start --domain <name> --workflow_id <id> --workflow_type <type> --tasklist <tl> --execution_timeout <seconds>

# Query workflow history
./cadence workflow describe --domain <name> --workflow_id <id>

# Show workflow execution
./cadence workflow show --domain <name> --workflow_id <id>

# Terminate a workflow
./cadence workflow terminate --domain <name> --workflow_id <id> --reason <reason>

# Reset workflow to a previous state
./cadence workflow reset --domain <name> --workflow_id <id> --event_id <id> --reason <reason>

# Signal a workflow
./cadence workflow signal --domain <name> --workflow_id <id> --name <signal_name> --input <json>
```

### Tasklist Operations
```bash
# Describe tasklist
./cadence tasklist describe --domain <name> --tasklist <name>

# Poll for tasks
./cadence tasklist poll --domain <name> --tasklist <name>
```

### Cluster Operations
```bash
# Get cluster info
./cadence cluster describe

# Get cluster health
./cadence cluster health
```

### Admin Operations
```bash
# Show admin help
./cadence admin --help
```

## Global Options

Always available with any command:

- `--address, --ad`: Cadence frontend service host:port (env: `CADENCE_CLI_ADDRESS`)
- `--domain, --do`: Default workflow domain (env: `CADENCE_CLI_DOMAIN`)
- `--context_timeout, --ct`: RPC call timeout in seconds (default: 5, env: `CADENCE_CONTEXT_TIMEOUT`)
- `--jwt`: JWT for authorization (env: `CADENCE_CLI_JWT`)
- `--jwt-private-key, --jwt-pk`: Path to private key for JWT creation (env: `CADENCE_CLI_JWT_PRIVATE_KEY`)
- `--transport, -t`: Transport protocol - 'grpc' or 'tchannel' (env: `CADENCE_CLI_TRANSPORT_PROTOCOL`)
- `--tls_cert_path, --tcp`: Path to TLS certificate (env: `CADENCE_CLI_TLS_CERT_PATH`)

## When to use me

Use this skill when:
- Managing Cadence domains and their configuration
- Starting, monitoring, or debugging workflow executions
- Investigating tasklist activity
- Performing administrative operations on the Cadence cluster
- Troubleshooting workflow issues

## Best Practices

1. **Set environment variables** to avoid repeating `--domain` and `--address` flags:
   ```bash
   export CADENCE_CLI_ADDRESS=localhost:7933
   export CADENCE_CLI_DOMAIN=my-domain
   ```

2. **Use short aliases** for common commands:
   - `d` for `domain`
   - `wf` for `workflow`
   - `tl` for `tasklist`
   - `adm` for `admin`
   - `cl` for `cluster`

3. **Enable TLS** for production environments using `--tls_cert_path`

4. **Use JWT authentication** when authorization is enabled on the server

## Version Compatibility

- CLI Feature Version: 1.7.0
- Release Version: v1.3.7-prerelease30-dirty

Server is always backward compatible with older CLI versions, but won't accept commands from newer CLI versions than it supports.
