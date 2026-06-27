# Review Guide — Milestone 2 (parity polish)

This branch implements all 8 Milestone-2 items from
[`docs/PARITY-PLAN.md`](PARITY-PLAN.md) §5/§6 — the deferred register TD-1…TD-8
([GitHub issues #28–#35](https://github.com/dyc3/cadence-rust-client/issues)).
Each TD is a self-contained commit; reviewing commit-by-commit is the easiest
path. The whole workspace builds, tests, and clippy-passes green.

## How to verify the whole thing at once

```bash
cargo build --workspace
cargo test --workspace                          # all unit/in-process tests
cargo clippy --workspace --all-features --all-targets   # CI's exact clippy (also builds the otel + integration targets)
cargo fmt --all -- --check
just replay-gate                                # determinism gate stays green
cargo build -p crabdance_worker --features otel # the optional OTLP feature compiles
```

Server-backed tests are gated behind the `integration` feature (`just integration`).

## Commit-by-commit

| # | Commit | What to look at | Key claim to check |
|---|---|---|---|
| TD-7 | `feat(worker): sticky workflow-cache idle expiration` | `crabdance_worker/src/executor/cache.rs` | LRU eviction already existed; this adds optional idle TTL (`WorkerOptions::sticky_cache_idle_ttl`) via a `Clock` seam. Tests: churn-bounds-size, LRU-not-FIFO, idle expiry, access refreshes deadline. |
| TD-6 | `feat(worker): wire poller rate limiting + auto-scaling` | `crabdance_worker/src/autoscaler.rs`, `pollers.rs`, `worker.rs` | The dead `RateLimitedPoller`/`AutoScalerOptions` are replaced by a real token-bucket `RateLimiter` (paces polls) + a resizable `PollerGate` + `PollerAutoScaler` (scales min↔max on poll latency). **The old semaphore "rate limiter" gated concurrency, not rate** — note the swap. Pure decision fn + paused-clock pacing tests. |
| TD-8 | `feat(worker): triage + implement remaining WorkerOptions fields` | `docs/worker-options-parity.md`, `worker.rs`, `pollers.rs` | Every Go `worker.Options` field classified implement/map/non-goal. New: `disable_workflow_worker`, `disable_activity_worker`, `task_list_activities_per_second` (server-side, sent as `TaskListMetadata`). |
| TD-1 | `feat(core,worker): around-execution interceptor seam` | `crabdance_core/src/interceptor.rs`, `crabdance_worker/src/interceptor.rs`, `handlers/activity.rs`, `executor/workflow.rs` | Lean `before`(veto)/`after`(timing) hooks, **not** Go's 21-method chain. Wired at the activity + workflow-decision-task boundaries; composes with `ContextPropagator`. Determinism documented. `TimingInterceptor` example. |
| TD-2 | `feat(worker): OTel/OTLP export + W3C trace propagation` | `executor/workflow.rs` (sink `outbound_header`), `context.rs` (`with_shared_propagation`), `crabdance_worker/src/otel.rs`, `handlers/activity.rs` | **Wire injection** (the unblocked part of #39): the sink injects the live propagation context into activity/child headers (was `None`); test asserts a seeded `traceparent` reaches the scheduled activity. **OTLP** behind the `otel` feature; activity spans parented from the header. |
| TD-4 | `feat(testsuite): mocking richness` | `crabdance_testsuite/src/env.rs`, `crabdance_workflow/src/driver.rs` (`run_with_callbacks`) | `.once()`/`.times(n)` + `assert_expectations`; lifecycle listeners; `register_delayed_callback` (new driver callback interleaving); `set_last_completion_result`; `on_activity_attempts` (retry seq). |
| TD-5 | `feat(worker): runnable replayer + shadow worker` | `crabdance_worker/src/replayer.rs`, `crabdance/src/shadow.rs` | `WorkflowReplayer` replays recorded histories through the **real** replay engine + verifier and reports non-determinism; `WorkflowShadower` streams a live domain via a `HistorySource` (client adapter in the umbrella crate). |
| TD-3 | `feat: sessions` | `crabdance_workflow/src/session.rs`, `context.rs`, `crabdance_worker/src/session.rs`, `handlers/activity.rs`, `worker.rs` | create/recreate/complete/`get_session_info` + `execute_activity_in_session`; token bucket bounds concurrency; `enable_session_worker` adds creation + resource-list pollers. In-memory test verifies resource-list routing; gated integration test runs it end-to-end. |

## The architectural spine (read these first)

- **`crabdance_worker/src/autoscaler.rs`** (TD-6) — the new poller-side primitives.
  If you review one new file, make it this one: the rate limiter and resizable
  gate are small, pure, and fully tested.
- **`crabdance_workflow/src/driver.rs` `run_with_callbacks`** (TD-4) — the only
  change to the determinism-critical driver. `run()` delegates to it with no
  callbacks, so the replay harness path is byte-for-byte unchanged.
- **`crabdance_worker/src/replayer.rs`** (TD-5) — ties the production replay engine
  (`WorkflowExecutor` + `match_replay_with_history`) into an offline tool.

## Risk notes / things worth a careful look

1. **`WorkerOptions` grew** several fields (idle-ttl, autoscaler, disable-workers,
   task-list/sec, interceptors, context_propagators). The three `load_test`
   profiles were converted to `..Default::default()` so future additions don't
   break them — confirm you're happy with that.
2. **Propagation injection snapshots at submit time** (`outbound_header`), reading
   the *live* propagation context per command. Mid-execution context changes
   before a given `execute_activity` are reflected; the contract is documented on
   the method.
3. **Replayer non-determinism semantics** — a history is flagged when the replayed
   decisions diverge from the recorded history (extra/missing/mismatched) or
   replay errors. The TD-5 tests include the "extra decision" case to prove it
   actually fires.
4. **Session simplification** (documented in `docs/sessions.md`): the creation
   activity returns the resource task list rather than long-running-heartbeating,
   so worker-death isn't auto-detected mid-session. Create / sticky-route /
   complete + bounded concurrency are complete; heartbeat-based failure detection
   is the tracked follow-up.

## Deliberately deferred (tracked, not silent)

- **OTel inbound client→workflow trace link** needs a `Header` field on the
  workflow-started history event, absent from the simplified wire types — the
  remaining piece of #39 (workflow→activity/child links are fully wired).
- **Session worker-death detection** (heartbeat) — see above.

Docs added: `worker-options-parity.md`, `replay-and-shadow.md`, `sessions.md`,
and an Interceptors + OTLP section in `observability.md`.
