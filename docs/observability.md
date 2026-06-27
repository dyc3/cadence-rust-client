# Observability

The Rust client instruments itself **natively and idiomatically** rather than through
an injected logger/metrics abstraction (see [PARITY-PLAN.md](PARITY-PLAN.md) §3):

- **Logs & spans** use the [`tracing`](https://docs.rs/tracing) facade.
- **Metrics** use the [`metrics`](https://docs.rs/metrics) facade.

The SDK emits through these facades but does **not** bundle a subscriber or exporter —
your application installs the ones it wants. This is the analogue of the Go client's
zap + tally, expressed with the Rust ecosystem's standard facades.

## Logging

Workflow code should log through the context so emission honors the **replay-aware
guard** (the analogue of Go's `EnableLoggingInReplay`):

```rust
ctx.log_info("processing order");   // suppressed during replay by default
```

`ctx.should_log()` returns `false` during replay unless logging-in-replay is enabled via
`WorkerOptions { enable_logging_in_replay: true, .. }`. Worker- and activity-level logs
go through `tracing` directly.

Install any `tracing` subscriber, e.g.:

```rust
tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
```

## Metrics

The worker emits the following parity metrics through the `metrics` facade. Counters are
suffixed `_total`; histograms are in seconds. Names are exported as constants in
`crabdance_worker::metrics`.

| Metric | Type | Tags | Meaning |
|---|---|---|---|
| `cadence_decision_task_started_total` | counter | `task_list` | Decision tasks started |
| `cadence_decision_task_completed_total` | counter | `task_list` | Decision tasks completed |
| `cadence_decision_task_failed_total` | counter | `task_list` | Decision tasks failed |
| `cadence_decision_task_execution_latency_seconds` | histogram | `task_list` | Decision task wall-clock latency |
| `cadence_activity_task_started_total` | counter | `activity_type` | Activity tasks started |
| `cadence_activity_task_completed_total` | counter | `activity_type` | Activity tasks completed |
| `cadence_activity_task_failed_total` | counter | `activity_type` | Activity tasks failed |
| `cadence_activity_task_execution_latency_seconds` | histogram | `activity_type` | Activity task latency |
| `cadence_workflow_panic_total` | counter | `task_list` | Workflow code panics (the `Panic` error variant) — not ordinary workflow failures |
| `cadence_poller_start_total` | counter | `task_list` | Pollers started |
| `cadence_concurrent_task_quota` | gauge | `task_list` | Configured concurrent decision-task quota |

### Exporting to Prometheus

Add an exporter and install a global recorder before starting your worker:

```toml
# Cargo.toml
metrics-exporter-prometheus = "0.16"
```

```rust
use metrics_exporter_prometheus::PrometheusBuilder;

// Installs a global recorder and serves /metrics on 0.0.0.0:9000.
PrometheusBuilder::new()
    .install()
    .expect("install Prometheus recorder");

// ... start your worker; the metrics above are now scrapeable.
```

A sample Prometheus scrape config lives in [`prometheus/`](../prometheus/) and Grafana
dashboards in [`grafana/`](../grafana/).

## Interceptors

Around-execution interceptors (`crabdance_worker::interceptor`) wrap the
execution of activities and workflow decision tasks — the lean Rust analogue of
the Go client's `WorkflowInterceptor`, reduced to the one capability native
observability does not already provide: a generic **timing / fault-injection /
policy-gate** seam.

An `Interceptor` has two synchronous hooks:

- `before(ctx)` runs before the operation; returning `Err` **vetoes** it (the
  operation is skipped and fails with that error).
- `after(ctx, outcome)` runs after it completes, with the wall-clock `Outcome`.

Register interceptors on the worker; they fire `before` in order and `after` in
reverse (middleware nesting):

```rust
use std::sync::Arc;
use crabdance_worker::{WorkerOptions, TimingInterceptor};

let options = WorkerOptions {
    interceptors: vec![Arc::new(TimingInterceptor)],
    ..Default::default()
};
```

Each `InterceptorContext` carries the operation's `PropagationContext`, so an
interceptor composes with the configured `ContextPropagator`s (it can read the
propagated trace context / baggage for the boundary it wraps).

**Determinism.** Hooks are synchronous and run on the worker, not inside
replayed workflow code, so wall-clock work (timing, reading a feature flag) is
allowed. A workflow-boundary interceptor wraps a *decision-task execution*; it
must not mutate workflow state or emit commands, and a veto there fails the
decision task rather than the workflow logic.

> **Deferred (M2, TD-2):** OpenTelemetry/OTLP export and W3C `traceparent` propagation,
> riding on the `ContextPropagator` seam.
