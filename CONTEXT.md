# Cadence Rust Client

Domain vocabulary for the Cadence Rust client. The architecture follows the
Uber Cadence Go client (the source of truth per `AGENTS.md`); terms here favour
the names that client uses.

## Language

**DataConverter**:
The single seam for serializing workflow/activity inputs, outputs, and signals to
bytes and back. Configured once on both the client and the worker — Cadence carries
no per-payload encoding tag, so both sides must agree out of band. JSON is the
default adapter.
_Avoid_: serializer, codec, encoder, marshaller.

**Payload**:
The opaque byte form of a value once a `DataConverter` has encoded it. On the wire
it is `{ data: bytes }` with no format metadata — the decoder relies on its
configured `DataConverter`, not on the payload describing itself.
_Avoid_: blob, message, encoded value.

**Adapter** (of `DataConverter`):
A concrete converter satisfying the seam. `JsonDataConverter` is the production
adapter; a framing/fake converter proves the seam is real and swappable in tests.
Generic `encode`/`decode` live on the `DataConverterExt` extension trait so the
seam itself stays object-safe (`Arc<dyn DataConverter>`). The Rust client is
JSON-only over gRPC; Go's Thrift codec path is a deliberate non-goal.

## Workflow Authoring

**Replay**:
Re-executing workflow code against recorded history to reconstruct in-memory state.
Correctness requires every async path to be driven by the deterministic dispatcher
(creation-order polling with custom wakers), never the tokio scheduler. Because the
Rust client expresses concurrency with idiomatic async/await rather than Go's
`Selector`/coroutines, the dispatcher *is* the determinism contract.
_Avoid_: re-run, rehydrate.

**Determinism**:
The property that replaying a workflow produces the identical sequence of commands.
A leak is any workflow-side operation whose result or ordering is decided outside
the dispatcher (e.g. a detached `tokio::spawn`, wall-clock time, `rand`).

## Visibility & Management

**Visibility**:
The server-side, queryable index of workflow executions backing list, scan, count,
and search-attribute queries. Distinct from history (the per-execution event log).
_Avoid_: search index, listing API.

**Search Attribute**:
An indexed key/value attached to a workflow execution and queried via Visibility.
Mutated from inside a workflow by `upsert_search_attributes`, which must flow through
the deterministic command pipeline (not a side-effecting no-op).
_Avoid_: tag, label, metadata.

## Worker & Activity

**Async (External) Activity Completion**:
Completing an activity out-of-band — the activity function signals that its result is
pending and an external process later completes it by task token via the client.
The worker must recognize the pending signal and *not* auto-respond to the server.
_Avoid_: deferred completion, manual completion.

**Session**:
A Cadence affinity construct binding a sequence of activities to a single worker
(e.g. for shared local resources). Requires sticky task-list routing.
_Avoid_: activity pinning, worker stickiness (which is a separate replay-cache concept).

**Replayer / Shadow Worker**:
Tooling that replays real production histories against current workflow code to detect
non-determinism before deploy. The replayer runs histories offline; the shadow worker
streams them from a live domain.
_Avoid_: verifier (reserved for the in-process replay check).

**Context Propagator**:
The seam that injects/extracts ambient data — W3C trace context, user baggage — into
Cadence task `Header`s so it crosses the workflow→activity/child boundary. The only
extensibility seam shipped in Milestone 1; general around-execution interceptor hooks
are a documented deferral.
_Avoid_: interceptor (reserved for the deferred around-execution hooks), middleware.
