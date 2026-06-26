# Cadence Rust Client — Go-Client Parity Plan

Status: **active** · Authored 2026-06-26 · Baseline: **v0.3.0**

This is the plan to bring `cadence-rust-client` "up to par" with the Uber Cadence Go
client (`cadence-go-client/`, the source of truth per `AGENTS.md`).

It is grounded in a four-subsystem gap analysis (client API, workflow authoring,
worker/activity, core/testsuite/observability) performed against the Go source.

---

## 1. What "parity" means here

**Behavioral / idiomatic parity**, not 1:1 API parity. We match the Go client's
*capabilities and wire-protocol behavior*, expressed in idiomatic Rust. See
[ADR-0001](adr/0001-idiomatic-parity.md).

- `async`/`await` + tokio subsume `Selector`, `WaitGroup`, `workflow.Go`, `Await`,
  and the `WithActivityOptions(ctx,…)` builder pattern — these are **non-goals**.
- The determinism guarantee therefore rests on our `WorkflowDispatcher`, making
  **determinism a first-class, replay-gated workstream** (Workstream A).

### Cross-cutting decisions

| Decision | Choice | Reference |
|---|---|---|
| Parity model | Idiomatic/behavioral, not 1:1 | [ADR-0001](adr/0001-idiomatic-parity.md) |
| Transport / codec | gRPC + JSON only; **Thrift out** (all milestones) | [ADR-0002](adr/0002-grpc-json-only.md) |
| Determinism | First-class workstream + replay CI gate | §A |
| Observability | `tracing` + `metrics` facade now; OTel/OTLP seam in M2 | §3 |
| Extensibility seam | `ContextPropagator` only in M1; around-execution hooks deferred | §G, §6 |
| API stability | Break freely pre-1.0; lock SemVer at 1.0 = "parity reached" | — |
| Deferrals | Every deferral/tech-debt item is tracked (§6), never implicit | — |

---

## 2. Gap map (v0.3.0 vs Go)

Condensed from the subsystem analysis. ✅ done · 🟡 partial/stubbed · ❌ missing.

**Solid (✅):** error taxonomy (full parity); core execution — activities, child
workflows, timers, signals, queries, side effects, versioning, local activities,
heartbeating, sticky cache, non-determinism detection; ~86% of client methods present
by signature.

**Partial (🟡):**
- DomainClient — all 4 methods stubbed, no service client wired (`domain.rs:162,180–195`)
- Visibility — `ListWorkflow` delegates to open-only; `ListArchivedWorkflow` missing
  (`client.rs:1226`); `QueryWorkflow` ignores `query_rejected` (`client.rs:1324`)
- `upsert_search_attributes` — no-op stub (`crabdance_workflow/src/context.rs:782`)
- Testsuite — execute/register only; no mock expectations, listeners, timer control
- DataConverter — JSON only (intended; Thrift is a non-goal)
- Autoscaler / session worker / poller rate-limiting — structs exist, not wired
- Sticky cache — no eviction/expiration policy

**Missing (❌):** native observability (metrics/tracing are no-op skeletons);
interceptor/extensibility seam; external/async activity completion path; shadow
worker / replayer; assorted workflow accessors (public `IsReplaying`, cron
`LastCompletionResult`, history-size accessors); ~19 `WorkerOptions` fields; several
`StartWorkflowOptions` fields; `WorkflowRun::GetID`, `get_with_timeout`
(`client.rs:1610`).

---

## 3. Observability approach (rationale)

Go uses zap (logs) + tally (metrics) + opentracing, delivered through its interceptor
chain. We instead instrument **natively and idiomatically**:

- **Logs + spans:** the `tracing` crate (already partially used). Drop the unused
  `Logger` trait stub.
- **Metrics:** the `metrics` facade crate (`counter!`/`histogram!`/`gauge!`).
  Drop `NoopMetricsScope`. Emit parity metrics (poller starts, task latency, panic
  counter, concurrent-task quota, …). Applications install their own exporter
  (e.g. `metrics-exporter-prometheus`).
- **Replay-aware emission:** suppress (or tag) metric/log emission while the dispatcher
  is replaying history — the analogue of Go's `EnableLoggingInReplay`.
- **OTel seam (M2):** add `tracing-opentelemetry` → OTLP and W3C `traceparent`
  propagation, riding on the `ContextPropagator` (§G). Deferred so we don't pay the
  maturing otel-metrics tax now.

Rejected: keeping Go's injected `MetricsScope`/`Logger` trait model (least idiomatic,
forces users to learn our abstraction instead of the ecosystem's) and all-in
OpenTelemetry now (heaviest deps, immature metrics API).

---

## 4. Milestone 1 — "usable in production"

Production-blockers first, as 18 independently-grabbable vertical slices. Numbers match
the published GitHub issues.

### Enabling seam (already present)

The real `WorkflowContext` is built around `Arc<dyn CommandSink>` (`context.rs:133`);
every operation (`execute_activity`, `new_timer`, `execute_child_workflow`, signals,
markers) submits through `command_sink.submit(...)`. This is the Rust analogue of Go's
`workflowEnvironment` interface: production supplies a server-backed sink, and tests /
replay supply alternative sinks. The same **driver loop + pluggable resolver** powers
both the replay harness (resolve commands from recorded history) and the testsuite
(resolve by running or mocking activities). The current `TestWorkflowContext` mimic
bypasses this seam and is retired by slice 18.

### Determinism & replay foundation
1. **In-memory workflow driver loop** — drive the real `WorkflowContext`/dispatcher to
   "all tasks blocked" via a pluggable command resolver; auto-fire the earliest timer on
   a mock clock when blocked (mirrors Go's `startMainLoop`/`autoFireNextTimer`). Shared
   foundation for slices 2 and 18. *(no blockers)*
2. **Replay harness + golden fixture** — replay resolver matches recorded history; assert
   the emitted command sequence. Port fixtures from Go's `replaytest`. *(blocked by 1)*
3. **Fix `get_version` leak + determinism-leak audit** — route the version marker through
   the sink/command pipeline instead of the detached `tokio::spawn` (`context.rs:735`);
   sweep the workflow crate for other detached spawns, wall-clock reads, and `rand` on
   workflow paths. *(blocked by 2)*
4. **Replay CI gate + non-determinism detection parity** — replay suite as a required CI
   check; mismatch → `NonDeterministicError` with full `ReplayContext`. *(blocked by 2)*

### DomainClient
5. **register + describe end-to-end** — wire the domain gRPC service client
   (`domain.rs:162`), implement with conversions, integration test. *(no blockers)*
6. **update + failover** — remaining two methods + integration test. *(blocked by 5)*

### Visibility & search attributes
7. **list / list-archived workflows** — real visibility-backed `list_workflows` (not the
   open-only delegate) + `list_archived_workflows` (`client.rs:1226`). *(no blockers)*
8. **scan / count + `query_rejected` handling** (`client.rs:1324`). *(no blockers)*
9. **`upsert_search_attributes` through the deterministic command pipeline** — emit the
   decision (`context.rs:782` no-op today); verify `get_search_attributes`; replay
   fixture. *(blocked by 2)*

### Observability (native)
10. **`tracing` logs+spans + replay-aware emission guard** — remove the `Logger` stub.
    *(no blockers)*
11. **`metrics` facade + parity metrics + Prometheus example** — remove
    `NoopMetricsScope`; emit poller/task/panic/quota metrics; example exporter + docs.
    *(blocked by 10)*

### Async / external activity completion
12. **External activity completion end-to-end** — worker honors "result pending" and does
    not auto-respond; `complete_activity`/`complete_activity_by_id` verified by task
    token; integration test. *(no blockers)*

### API-completeness polish (break freely)
13. **`StartWorkflowOptions` missing fields** — `JitterStart`, `FirstRunAt`,
    `CronOverlapPolicy`, `ActiveClusterSelectionPolicy`. *(no blockers)*
14. **`CancelWorkflow` cancel-options + `WorkflowRun`** — `WithCancelReason` parity; add
    `get_id()`; implement `get_with_timeout` (`client.rs:1610`); reconcile `get()`.
    *(no blockers)*
15. **Workflow accessors + utility helpers** — public `is_replaying()`; cron
    `has/get_last_completion_result`; `get_total_history_bytes()`/`get_history_count()`;
    `NewValue`/`NewValues` equivalents; `is_workflow_error`. *(no blockers)*

### Versioning
16. **Workflow versioning correctness end-to-end** — marker recorded through the
    deterministic command pipeline; replay returns the recorded version (golden fixture);
    range validation; `ExecuteWithVersion`/`ExecuteWithMinVersion`. *(blocked by 3, 2)*

### Context propagation
17. **`ContextPropagator` seam + header propagation + W3C scaffold** — `inject`/`extract`
    over Cadence `Header`s across the workflow→activity/child boundary; no-op default + a
    `traceparent` propagator scaffold (OTLP export is M2 / TD-2). Around-execution
    interceptor hooks are deferred — see §6 (TD-1). *(no blockers)*

### Testsuite
18. **Unit-testing baseline (real context, Go-parity core)** — a test resolver behind
    `CommandSink` that runs real **or** mocked activities (`on_activity` by value/fn),
    auto-fires timers on the mock clock, and supports signals, queries, child workflows,
    cancellation, `get_version`, and side effects in-test; real assertions; retire the
    `TestWorkflowContext` mimic. Fluent cardinality + lifecycle listeners are M2 (TD-4).
    *(blocked by 1)*

---

## 5. Milestone 2 — "parity polish"

- **Interceptor around-execution hooks** — lean Rust hooks (around activity / child /
  workflow execution) layered on the M1 `ContextPropagator`. (Deferred from §G.)
- **OTel/OTLP export** — `tracing-opentelemetry` + W3C propagation over Cadence headers.
- **Sessions** — `create/recreate/complete_session`, `get_session_info`, sticky
  task-list routing, session-worker validation.
- **Testsuite mocking** — mock expectations (`on_activity`, cardinality), lifecycle
  listeners, timer/clock control. Likely built on `mockall`.
- **Shadow worker / replayer** — wire the existing `ShadowOptions`/`ReplayOptions` into
  a runnable replayer + live shadowing.
- **Autoscaler & poller rate-limit integration** — wire `AutoScalerOptions` and
  `RateLimitedPoller` into the poller manager.

---

## 6. Deferred work / tech-debt register

Per project policy, nothing is deferred silently. Each item below must become a
**labeled tracking issue** so it can be resolved later.

| # | Item | Deferred from | Why | Resolve in |
|---|---|---|---|---|
| TD-1 | Around-execution interceptor hooks (Go's 21-method chain) | §G | Native observability removes the main need; `ContextPropagator` covers propagation. Start minimal, grow if demanded. | M2 |
| TD-2 | OTel/OTLP export + W3C propagation | §3, §G | Avoid maturing otel-metrics tax now; rides on `ContextPropagator`. | M2 |
| TD-3 | Sessions (create/recreate/complete/info) | §5 | Not a first-production blocker; needs sticky routing. | M2 |
| TD-4 | Testsuite mocking richness — fluent cardinality (`.once()`/`.times()`), the 10 lifecycle listeners, `register_delayed_callback`, sessions-in-test, `set_last_completion_result`/heartbeat-retry simulation | slice 18 | The Go-parity testing **baseline** (real-context engine, mock clock, real/mock activities, signals/queries/child/cancel/`get_version`) ships in M1 slice 18; this is the convenience layering on top. | M2 |
| TD-5 | Shadow worker / replayer | §5 | Pre-deploy safety tool, not runtime-blocking. | M2 |
| TD-6 | Autoscaler / poller rate-limit wiring | §5 | Structs exist; fixed pools work for now. | M2 |
| TD-7 | Sticky cache eviction/expiration policy | §2 | Cache works; bounded growth needs a policy. | M2 |
| TD-8 | Remaining `WorkerOptions` fields (DisableWorkflow/ActivityWorker, IsolationGroup, FeatureFlags, BugPorts, WorkerStats, …) | §2 | Triage which actually matter; many are Go-specific. | M2 (triage) |
| TD-9 | Thrift codec | ADR-0002 | Permanent non-goal; recorded to prevent re-litigation. | Won't do |

---

## 7. Definition of done

- **M1 done** = a real workflow can be authored, deployed, operated, and observed in
  production: deterministic replay is CI-gated, domains/visibility work, metrics/traces
  flow, activities can complete out-of-band, and the public API has no `unimplemented!`
  on a documented path.
- **1.0 / "parity reached"** = M1 + M2 complete, the deferred register is burned down
  (except TD-9), and the public API is stabilized under SemVer.

---

## 8. References

- Gap analysis: this document §2 (client API, workflow authoring, worker/activity,
  core/testsuite/observability).
- Go source of truth: `cadence-go-client/`.
- Domain glossary: [`CONTEXT.md`](../CONTEXT.md).
- Decisions: [`docs/adr/`](adr/).
