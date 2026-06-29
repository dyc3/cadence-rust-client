# `WorkerOptions` parity triage (TD-8)

This is the field-by-field triage of the Go client's `worker.Options`
(`cadence-go-client/internal/worker.go`) against the Rust `WorkerOptions`
(`crabdance_worker::WorkerOptions`), per [issue #35](https://github.com/dyc3/cadence-rust-client/issues/35)
and [PARITY-PLAN.md](PARITY-PLAN.md) §6 TD-8.

Each Go field is classified as one of:

- **Implemented** — a Rust `WorkerOptions` field with equivalent behavior.
- **Mapped** — covered by a different Rust mechanism (a builder method, a
  different seam, or another option); no new field needed.
- **Non-goal** — intentionally not ported. Recorded here so it is not re-raised.
  Most non-goals are Go-specific (tally/zap/opentracing abstractions that Rust
  replaces with the native `metrics`/`tracing` facades, per PARITY-PLAN §3) or
  are flagged `Deprecated` in the Go source.

## Classification

| Go field | Class | Rust equivalent / rationale |
|---|---|---|
| `MaxConcurrentActivityExecutionSize` | Implemented | `max_concurrent_activity_execution_size` |
| `WorkerActivitiesPerSecond` | Implemented | `worker_activities_per_second` (per-worker poll pacing, wired in TD-6) |
| `MaxConcurrentLocalActivityExecutionSize` | Implemented | `max_concurrent_local_activity_execution_size` |
| `WorkerLocalActivitiesPerSecond` | Implemented | `worker_local_activities_per_second` |
| `TaskListActivitiesPerSecond` | **Implemented (new, TD-8)** | `task_list_activities_per_second` — sent to the server as `TaskListMetadata.max_tasks_per_second` on each activity poll |
| `MaxConcurrentActivityTaskPollers` | Implemented | `max_concurrent_activity_task_pollers` |
| `MaxConcurrentDecisionTaskExecutionSize` | Implemented | `max_concurrent_decision_task_execution_size` |
| `WorkerDecisionTasksPerSecond` | Implemented | `worker_decision_tasks_per_second` (wired in TD-6) |
| `MaxConcurrentDecisionTaskPollers` | Implemented | `max_concurrent_decision_task_pollers` |
| `AutoScalerOptions` | Implemented | `autoscaler` (TD-6) |
| `Identity` | Implemented | `identity` |
| `EnableLoggingInReplay` | Implemented | `enable_logging_in_replay` |
| `DisableWorkflowWorker` | **Implemented (new, TD-8)** | `disable_workflow_worker` — skips decision pollers |
| `DisableActivityWorker` | **Implemented (new, TD-8)** | `disable_activity_worker` — skips activity pollers + local-activity executor |
| `DisableStickyExecution` | Implemented | `disable_sticky_execution` |
| `StickyScheduleToStartTimeout` | Implemented | `sticky_schedule_to_start_timeout` |
| `NonDeterministicWorkflowPolicy` | Implemented | `non_deterministic_workflow_policy` |
| `WorkerStopTimeout` | Implemented | `worker_stop_timeout` |
| `EnableSessionWorker` | Implemented | `enable_session_worker` (behavior in TD-3) |
| `MaxConcurrentSessionExecutionSize` | Implemented | `max_concurrent_session_execution_size` (TD-3) |
| `DataConverter` | Mapped | `CadenceWorker::with_data_converter(...)` builder seam |
| `ContextPropagators` | Mapped | `WorkerOptions::context_propagators` (added in TD-2 with the propagation wiring) |
| `WorkflowInterceptorChainFactories` | Mapped | `WorkerOptions::interceptors` (added in TD-1, the lean around-execution `Interceptor` trait) |
| `BackgroundActivityContext` | Mapped | `CadenceWorker::with_resources(...)` — the Rust mechanism for injecting shared state into activities |
| `Authorization` | Mapped | Auth lives on the client transport (`crabdance_client::auth`); the worker uses the already-authenticated service handle |
| `ShadowOptions` / `EnableShadowWorker` | Mapped | Shadowing is a standalone replayer/shadower (TD-5), not a worker mode; `ShadowOptions` feeds that tool |
| `MinConcurrentActivityTaskPollers` | Non-goal | Deprecated in Go ("no effect, use AutoScalerOptions"); subsumed by `autoscaler.min_pollers` |
| `MinConcurrentDecisionTaskPollers` | Non-goal | Deprecated in Go; subsumed by `autoscaler.min_pollers` |
| `PollerAutoScalerCooldown` | Non-goal | Deprecated in Go; the Rust autoscaler derives its evaluation interval from `target_poll_duration` |
| `PollerAutoScalerTargetUtilization` | Non-goal | Deprecated in Go ("not used any more") |
| `PollerAutoScalerDryRun` | Non-goal | Deprecated in Go ("not used any more") |
| `MetricsScope` (tally) | Non-goal | Rust emits through the native `metrics` facade; no injected scope (PARITY-PLAN §3) |
| `Logger` (zap) | Non-goal | Rust emits through the native `tracing` facade; no injected logger (PARITY-PLAN §3) |
| `Tracer` (opentracing) | Non-goal | Rust uses `tracing` + OTel export (TD-2); no opentracing `Tracer` |
| `IsolationGroup` | Non-goal (for now) | Zone/failure-group routing has no Rust task-routing analogue yet; revisit only if isolation routing is requested |
| `FeatureFlags` | Non-goal | Server-side/Go-specific feature toggles; not part of the client behavioral contract |
| `WorkerBugPorts` | Non-goal | Explicitly deprecated in Go ("bugports are always deprecated and may be removed at any time") |
| `WorkerStats` | Non-goal | Go marks it "in development and very likely to change"; Rust exposes worker health via the `metrics` facade |

## Notes

- **Per-worker vs. per-task-list activity limits.** `WorkerActivitiesPerSecond`
  is a *client-side* limit enforced by the worker's poll pacing
  (`worker_activities_per_second`, TD-6). `TaskListActivitiesPerSecond` is a
  *server-side* limit for the whole task list, communicated to the server via
  `TaskListMetadata` on the activity poll request — these are independent and
  both are now honored.
- **Disable flags** make a worker single-purpose (workflow-only or
  activity-only). With both `true`, only background loops (e.g. session, if
  enabled) would run.
