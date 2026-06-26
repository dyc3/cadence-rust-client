# Idiomatic parity with the Go client, not 1:1 API parity

We are bringing this client "up to par" with the Uber Cadence Go client by matching
its **capabilities and wire-protocol behavior**, expressed in **idiomatic Rust** —
not by mirroring its API surface. Anything you can *do* in a Go workflow you must be
able to *do* in a Rust workflow, even where the API shape differs.

Concretely, Rust `async`/`await` and tokio primitives **subsume** several Go
constructs, which therefore become non-goals: `Selector` (→ `tokio::select!`),
`WaitGroup` (→ `futures::join_all`), `workflow.Go` coroutines (→ `ctx.spawn` / `.await`),
`Await(condition)` (→ awaiting a future), and the `WithActivityOptions(ctx, …)`
context-builder pattern (→ plain options structs).

## Consequence: determinism becomes our runtime's responsibility

Go's `Selector`/channel/coroutine model is not just ergonomics — it is Go's
*deterministic* concurrency model for replay. By replacing it with idiomatic async,
the determinism guarantee shifts entirely onto our `WorkflowDispatcher`
(`crabdance_workflow/src/dispatcher.rs`), which polls spawned tasks in creation order
with custom wakers rather than handing them to the (non-deterministic) tokio scheduler.

Because of this, **determinism correctness is a first-class workstream with a replay
test gate in CI**, not an assumed property. Any workflow-side operation whose result or
ordering escapes the dispatcher (detached `tokio::spawn`, wall-clock time, `rand`) is a
replay-corrupting bug. (Known example at time of writing: a detached `tokio::spawn` in
`get_version`, `crabdance_workflow/src/context.rs:735`.)

## Status

Accepted — 2026-06-26.
