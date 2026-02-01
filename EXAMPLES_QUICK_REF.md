# Cadence Examples Quick Reference

## Examples at a Glance

| # | Example | Features | Dependencies |
|---|---------|----------|--------------|
| 1 | hello_workflow | Basic workflow, activity, worker | None (exists) |
| 2 | activity_basics | Multiple activities, chaining | 1 |
| 3 | activity_advanced | Heartbeats, cancellation, deadlines | 2 |
| 4 | workflow_signals | Internal signals | 1 |
| 5 | workflow_external | External signals, cancellation | 4 |
| 6 | child_workflows | Child workflow execution | 2 |
| 7 | time_and_sleep | Timers, sleep, time manipulation | 4 |
| 8 | versioning | get_version, side_effect | 2 |
| 9 | continue_as_new | Continue-as-new pattern | 6 |
| 10 | error_handling | Custom errors, timeouts | 3, 6 |
| 11 | retry_policies | Retry configuration | 10 |
| 12 | query_operations | Query workflows | 1 |
| 13 | workflow_options | ID reuse, parent policies | 6 |
| 14 | search_attributes | Upsert search attributes | 1 |
| 15 | client_operations | Full client API | 5, 12 |
| 16 | domain_management | Domain registration | None |
| 17 | worker_configuration | Worker setup, sticky cache | 1 |
| 18 | local_activities | Local activity execution | 2 |
| 19 | workflow_patterns | Saga, fan-out/fan-in | 6, 10 |
| 20 | testing_basics | Test environment deep dive | 1 |
| 21 | workflow_replay | WorkflowReplayer | 20 |
| 22 | data_converter | Custom serialization | 1 |
| 23 | performance | Performance tuning | 17 |
| 24 | complete_application | Full integration | All |
| 25 | best_practices | Idiomatic patterns | All |

## Feature Coverage Matrix

### Activity Features
| Feature | Examples |
|---------|----------|
| Basic execution | 1, 2 |
| Heartbeats | 3 |
| get_heartbeat_details | 3 |
| Deadline tracking | 3 |
| Cancellation detection | 3 |
| ErrResultPending | 3 |
| Local activities | 18 |
| Retry policies | 11 |

### Workflow Features
| Feature | Examples |
|---------|----------|
| execute_activity | 1, 2, 6 |
| execute_local_activity | 18 |
| execute_child_workflow | 6, 9, 19 |
| Internal signals | 4, 7 |
| External signals | 5 |
| External cancellation | 5 |
| side_effect | 8 |
| mutable_side_effect | 8 |
| get_version | 8 |
| sleep/now | 7 |
| continue_as_new | 9 |
| upsert_search_attributes | 14 |

### Error Handling
| Feature | Examples |
|---------|----------|
| CustomError | 10 |
| CanceledError | 10 |
| TimeoutError | 10 |
| PanicError | 10 |
| ActivityTaskError | 10 |
| ChildWorkflowExecutionError | 10 |

### Client Operations
| Feature | Examples |
|---------|----------|
| start_workflow | 15 |
| execute_workflow | 15 |
| signal_workflow | 15 |
| signal_with_start | 15 |
| cancel | 15 |
| terminate | 15 |
| query | 15 |
| get_history | 15 |
| list_executions | 15 |

### Configuration
| Feature | Examples |
|---------|----------|
| RetryPolicy | 11 |
| ActivityOptions | 2, 3 |
| ChildWorkflowOptions | 6, 13 |
| WorkflowIdReusePolicy | 13 |
| ParentClosePolicy | 13 |
| DataConverter | 22 |

### Testing
| Feature | Examples |
|---------|----------|
| TestWorkflowEnvironment | 1, 20 |
| TestActivityEnvironment | 20 |
| WorkflowReplayer | 21 |
| Time manipulation | 7, 20 |

### Domain Management
| Feature | Examples |
|---------|----------|
| Domain registration | 16 |
| Domain description | 16 |
| Domain configuration | 16 |

## Recommended Learning Path

### Beginner Path (1-2 weeks)
1. hello_workflow
2. activity_basics
3. workflow_signals
4. testing_basics

### Intermediate Path (2-3 weeks)
5. activity_advanced
6. child_workflows
7. time_and_sleep
8. error_handling
9. query_operations

### Advanced Path (2-3 weeks)
10. versioning
11. continue_as_new
12. retry_policies
13. workflow_patterns
14. workflow_replay

### Production Path (1-2 weeks)
15. worker_configuration
16. performance
17. client_operations
18. complete_application
19. best_practices

## Quick Commands

```bash
# Run all example tests
cargo test --workspace --examples

# Run specific example
cargo run -p XX_example_name

# Test specific example
cargo test -p XX_example_name

# Check all examples compile
cargo check --workspace --examples

# Build all examples
cargo build --workspace --examples
```

## Shared Code Locations

- **Domain Types**: `examples-common/src/types/`
- **Mock Activities**: `examples-common/src/activities/mock_activities.rs`
- **Test Helpers**: `examples-common/src/test_helpers.rs`
- **Assertion Helpers**: `examples-common/src/assertions.rs`
