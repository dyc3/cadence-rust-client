# Review Guide — Milestone 1 (Go-client parity)

This branch implements all 18 Milestone-1 slices from
[`docs/PARITY-PLAN.md`](PARITY-PLAN.md) §4 (GitHub issues #10–#27). Every slice is a
self-contained commit; reviewing commit-by-commit (bottom-up) is the easiest path. The
whole workspace builds, tests, and clippy-passes green; the deferred items (§6 TD-1…TD-9
plus three new follow-ups) remain open and tracked.

## How to verify the whole thing at once

```bash
cargo build --workspace
cargo test --workspace          # unit + in-process tests (server-backed ones are gated behind the `integration` feature)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
just replay-gate                # the determinism gate specifically (#13)
```

Integration tests that need live external services are gated behind the `integration`
Cargo feature (not `#[ignore]`): `docker compose up -d` then `just integration`.

## The architectural spine (read these first)

The whole determinism/test story rests on one new primitive and one existing seam:

- **`CommandSink`** (`crabdance_workflow/src/context.rs`) is the seam every workflow op
  goes through (`execute_activity`, `new_timer`, markers, …). Production supplies a
  server-backed sink (`ReplayCommandSink` in the worker); the new in-memory paths supply
  their own resolver behind the same seam.
- **`WorkflowDriver`** (`crabdance_workflow/src/driver.rs`, #10) drives the *real*
  `WorkflowContext`/dispatcher to "all-tasks-blocked", auto-firing the earliest timer on
  a mock clock. The replay harness (#11) and the test environment (#27) are both thin
  resolvers on top of it. **If you review one file, make it `driver.rs`.**

## Slice-by-slice

### Determinism & replay foundation

| # | Commit | What to look at | Key claim to check |
|---|---|---|---|
| #10 | `feat(workflow): in-memory workflow driver loop` | `crabdance_workflow/src/driver.rs` | Drives the real context; timers auto-fire deterministically; records the command sequence. Tests in-module. |
| #11 | `feat(testsuite): replay harness + golden fixture` | `crabdance_testsuite/src/replay.rs`, `fixtures/replay/order_workflow.json` | Re-running a workflow against recorded history must emit the **same schedulable-command sequence**; divergence → `ReplayError::NonDeterministic`. In replay, markers/versions/side-effects come from seeded caches and are *not* re-emitted. |
| #12 | `fix(workflow): route get_version marker through sink` | `context.rs` `get_version`, `submit_now`; worker marker IDs | **The headline determinism fix.** `get_version` no longer uses a detached `tokio::spawn` (which escaped the dispatcher); it records the marker synchronously and in order via `CommandSink::submit_now`. Worker marker IDs are now deterministic (no `SystemTime`). Audit note: the only other wall-clock uses in the workflow crate are test helpers / an overflow fallback. |
| #13 | `ci(replay): determinism replay gate` | `.github/workflows/ci.yml`, `justfile` | A dedicated required CI job runs the replay + driver determinism tests. |

### Client surface (implemented by sub-agents, integrated and re-verified)

| # | Commit | What to look at | Key claim |
|---|---|---|---|
| #14/#15 | `feat(client): DomainClient …` | `crabdance_client/src/domain.rs` | All four domain ops wired to the existing transport; `failover` uses the dedicated `FailoverDomain` RPC (Go parity). Retention converts to Cadence's whole-day wire format. |
| #16/#17/#22/#23 | `feat(client): visibility … start options … WorkflowRun` | `crabdance_client/src/client.rs`, `grpc.rs` | Real `list_workflows`/`list_archived_workflows`; `query_rejected` surfaced as an error; 4 new `StartWorkflowOptions` fields (cron-overlap defaults to `Skipped` per Go — note the behavior change from the previous hardcoded `0`); `cancel_workflow_with_options`, `WorkflowRun::get_id`. |

### Workflow authoring

| # | Commit | What to look at | Key claim |
|---|---|---|---|
| #18 | `feat(workflow): upsert_search_attributes …` | `context.rs`, `state_machine.rs`, worker handler | Was a no-op; now emits a real `UpsertWorkflowSearchAttributes` decision through the deterministic pipeline and updates `get_search_attributes`. Treated like a marker on replay (not re-emitted). |
| #24 | `feat(workflow): accessors + helpers` | `context.rs`, `future.rs` | `is_replaying`, cron `last_completion_result`, history count/bytes (bytes is best-effort serialized size — documented), `new_value`/`new_values`, `WorkflowError` predicates. |
| #25 | `feat(workflow): versioning correctness …` | `context.rs` `get_version_with_options` | `ExecuteWithVersion`/`ExecuteWithMinVersion` options; replays the recorded version; upserts `CadenceChangeVersion` SA (Go parity). Golden replay fixture in `replay.rs`. |

### Observability

| # | Commit | What to look at | Key claim |
|---|---|---|---|
| #19 | `feat(workflow): native tracing + replay guard` | `context.rs` `log_*`/`should_log`, `WorkerOptions.enable_logging_in_replay` | Dropped the `Logger` stub; logs go through `tracing`, suppressed during replay unless enabled (Go's `EnableLoggingInReplay`). |
| #20 | `feat(worker): native metrics + Prometheus docs` | `crabdance_worker/src/metrics.rs`, `docs/observability.md` | Dropped `MetricsScope` stub; emits parity metrics (decision/activity lifecycle + latency, panic, poller, quota) via the `metrics` facade at real boundaries. |

### Worker / extensibility / testing

| # | Commit | What to look at | Key claim |
|---|---|---|---|
| #21 | `feat(worker): async activity completion` | `future.rs` `ActivityError::ResultPending`, `handlers/activity.rs` | Worker no longer auto-responds when an activity returns `result_pending()`; completion arrives out-of-band via the existing `Client::complete_activity[_by_id]`. |
| #26 | `feat(core): ContextPropagator seam + W3C scaffold` | `crabdance_core/src/propagation.rs`, `context.rs` | The propagation seam + Noop/W3C propagators; the workflow carries/injects/extracts a `PropagationContext`. |
| #27 | `feat(testsuite): baseline on the real WorkflowContext` | `crabdance_testsuite/src/env.rs` | `WorkflowTestEnv` tests the **real** workflows you deploy (mocked/real activities, timers, signals, version gates, side effects, child workflows). The legacy mimic is superseded. |

## What is deliberately *not* done (tracked, not silent)

Per the project's explicit-deferral policy, every deferral is a labeled issue:

- **§6 register TD-1…TD-9** (#28–#35): M2 polish (interceptors, OTel/OTLP, sessions,
  testsuite mocking richness, shadow worker, autoscaler, sticky eviction, remaining
  `WorkerOptions`) and TD-9 Thrift (won't-do, ADR-0002). These M2 issues stay open by design.
- New follow-ups filed this pass:
  - **#38** — thread the 4 new start fields through `SignalWithStartWorkflow`.
  - **#39** — wire `ContextPropagator` headers onto the wire (needs a `Header` field on
    the started-event / schedule proto types; #26 shipped the programmatic seam).
  - **#40** — physically remove the legacy `TestWorkflowContext` mimic (#27 superseded it).

## Risk notes / things worth a careful look

1. **`submit_now` (context.rs)** drives a marker future with a single no-op poll. This is
   correct because marker submissions resolve immediately; a `Pending` here would mean a
   genuinely-async command was misrouted. Worth confirming you agree with that contract.
2. **`get_version` now upserts a search attribute** — this adds an
   `UpsertSearchAttributes` command between the version marker and any subsequent
   activity (see the regression test in `driver.rs`). Intended (Go parity), but it's a
   wire-behavior change.
3. **Cron-overlap default** changed from hardcoded `0` to `Skipped` (1) on start (#22) —
   matches Go, but it is a behavior change.
4. **`get_total_history_bytes`** is a best-effort serialized size, not the server figure
   (which the poll path doesn't surface). Documented inline.
5. The replay harness asserts **workflow-code determinism** (same history → same command
   sequence). The production replay path (worker `ReplayEngine` +
   `match_replay_with_history`) is the wire-level reconciler and is unchanged except for
   the deterministic marker-ID fix.
