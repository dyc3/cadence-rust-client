# Sessions

A **session** pins a sequence of activities to a single worker host, so they can
share host-local resources (a GPU, a scratch directory, an open connection). This
is the Rust analogue of the Go client's session API.

## Using sessions in a workflow

```rust
use crabdance::workflow::{SessionOptions, WorkflowContext};

async fn my_workflow(ctx: WorkflowContext, input: Job) -> Result<Out, WorkflowError> {
    // Acquire a session on a worker (base task list defaults to the workflow's).
    let session = ctx.create_session("", SessionOptions::default()).await?;

    // These activities all run on that same worker.
    let a = ctx.execute_activity_in_session(&session, "load", args, opts.clone()).await?;
    let b = ctx.execute_activity_in_session(&session, "transform", a, opts.clone()).await?;
    ctx.execute_activity_in_session(&session, "save", b, opts).await?;

    ctx.complete_session(&session).await;
    Ok(out)
}
```

- `create_session` / `recreate_session` / `complete_session` / `get_session_info`
  mirror the Go API. `recreate_session(token, opts)` re-establishes a session on
  the same worker from `session.recreate_token()`, for long sessions split across
  runs.
- `create_session` fails with **"too many outstanding sessions"** when the target
  worker is at its `max_concurrent_session_execution_size`, and with **"found
  existing open session"** if the context already holds an open one.

## Enabling the session worker

A worker must opt in to host sessions:

```rust
let worker = CadenceWorker::new(domain, task_list, WorkerOptions {
    enable_session_worker: true,
    max_concurrent_session_execution_size: 4, // concurrency bound
    ..Default::default()
}, registry, service);
```

With it on, the worker:

- serves the internal **creation** and **completion** activities (token
  acquire / release),
- polls the **creation task list** (`{task_list}__internal_session_creation`) and
  the host-specific **resource task list** (`{resourceID}@{host}`), so a session's
  activities are only picked up by that worker, and
- enforces concurrency with a token bucket: each `create_session` takes a token
  held until `complete_session`.

## How it works

1. `create_session` schedules the internal creation activity on the creation task
   list. The session worker acquires a token and returns its resource task list.
2. `execute_activity_in_session` schedules on that resource task list — only the
   session's worker polls it, so every activity runs on the same host.
3. `complete_session` runs the completion activity, releasing the token.

> **Simplification vs. Go:** the creation activity returns immediately rather than
> long-running-heartbeating, so a session is not auto-failed if its worker dies
> mid-session (the token would be released only on the workflow's
> `complete_session`). The create / sticky-route / complete + bounded-concurrency
> contract is complete; heartbeat-based worker-death detection is a tracked
> follow-up.
