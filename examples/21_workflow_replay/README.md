# Example 21: Workflow Replay

## Overview

This example demonstrates workflow replay for debugging and backwards compatibility testing.

## Features Demonstrated

- **Workflow Replay**: Replaying workflow history to reproduce issues
- **Non-determinism Detection**: Identifying code changes that break determinism
- **History Analysis**: Understanding workflow execution flow

## Key Concepts

### Workflow History

Workflow history contains all events that occurred during workflow execution.

### Replay

Replay allows you to test if workflow code changes are compatible with existing histories.

## Running the Example

```bash
cargo run -p workflow_replay
```

## Related Examples

- Previous: [20_testing_basics](../20_testing_basics)
- Next: [22_data_converter](../22_data_converter)
