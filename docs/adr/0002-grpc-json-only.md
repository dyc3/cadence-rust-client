# gRPC + JSON only; Thrift codec is out of scope

The Rust client communicates with Cadence over **gRPC only**, and its default (and only
first-party) `DataConverter` is **JSON**. The Go client additionally supports a Thrift
codec and Thrift/TChannel transport; we deliberately do **not** port these.

## Why

A prior attempt at the Thrift codec path was extremely buggy and difficult to get
correct in Rust. The gRPC + JSON path covers the full functional surface we need, and
JSON keeps payloads debuggable. The maintenance and correctness cost of Thrift is not
justified.

This is recorded so the decision is not silently re-litigated: if someone proposes
"adding Thrift for Go compatibility," the answer is a considered no, not an oversight.
Users who need a different encoding implement the object-safe `DataConverter` seam
(`crabdance_core/src/encoded.rs`); that extension point remains open.

## Status

Accepted — 2026-06-26. Applies to all milestones.
