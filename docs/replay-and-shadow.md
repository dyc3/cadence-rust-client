# Replayer & Shadow worker

Two pre-deploy safety tools for catching workflow non-determinism *before* it
reaches production, both built on the worker's real replay engine
(`WorkflowExecutor` + the determinism verifier). They wire the previously-unused
`ShadowOptions` / `ReplayOptions` ideas into runnable tools.

- **`WorkflowReplayer`** — replays recorded histories against your current
  workflow code, offline, and reports whether each is deterministic.
- **`WorkflowShadower`** — streams histories from a live domain and replays each,
  so a code change can be vetted against real production executions without
  touching their state.

## Replaying recorded histories

```rust
use std::sync::Arc;
use crabdance_worker::replayer::WorkflowReplayer;

// `registry` has the same workflows you register on a production worker.
let replayer = WorkflowReplayer::new(registry);

let report = replayer.replay_history("order-123", "run-abc", history).await;
if !report.is_deterministic() {
    eprintln!("non-deterministic: {:?}", report.outcome);
}

// Or a batch:
let reports = replayer.replay_histories(vec![
    ("order-1".into(), "run-1".into(), history_1),
    ("order-2".into(), "run-2".into(), history_2),
]).await;
let bad = reports.iter().filter(|r| !r.is_deterministic()).count();
```

A history is reported **non-deterministic** when replaying the current code
produces a decision sequence that diverges from the one recorded in the history
(an extra, missing, or mismatched decision), or when replay fails outright.

## Shadowing a live domain

The shadower reads executions and histories through a `HistorySource`. The worker
crate defines the seam; `crabdance::shadow::ClientHistorySource` adapts a live
`WorkflowClient` to it.

```rust
use std::sync::Arc;
use crabdance::shadow::ClientHistorySource;
use crabdance::worker::replayer::{ShadowMode, WorkflowReplayer, WorkflowShadower};

let source = Arc::new(ClientHistorySource::new(client));
let replayer = WorkflowReplayer::new(registry);

let shadower = WorkflowShadower::new(source, "WorkflowType = 'OrderWorkflow'", replayer)
    .with_mode(ShadowMode::Once);            // or Continuous, bounded by with_max_executions

let reports = shadower.run().await?;
for report in reports.iter().filter(|r| !r.is_deterministic()) {
    eprintln!("would break: {} / {}", report.workflow_id, report.run_id);
}
```

- `ShadowMode::Once` runs a single sweep over the visibility query's matches.
- `ShadowMode::Continuous` re-sweeps until `with_max_executions(n)` reports are
  collected (the safety bound) or a sweep returns nothing.

Run this in CI against a sampling query before promoting a workflow change, and
fail the build if any report is non-deterministic.
