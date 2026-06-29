//! Sessions: bind a sequence of activities to a single worker.
//!
//! A session pins all of its activities to one worker host so they can share
//! host-local resources (a GPU, a scratch directory, an open connection). This is
//! the Rust analogue of the Go client's session API.
//!
//! Mechanism (matching the Go client):
//!
//! 1. [`create_session`](crate::context::WorkflowContext::create_session) schedules
//!    an internal **creation activity** on a creation task list
//!    (`{base}__internal_session_creation`). A session worker picks it up, acquires
//!    a concurrency token, and **returns** the host-specific *resource task list*
//!    (`{resourceID}@{host}`).
//! 2. Activities run with [`execute_activity_in_session`](crate::context::WorkflowContext::execute_activity_in_session)
//!    are scheduled on that resource task list, so only the session's worker picks
//!    them up — they all run on the same host.
//! 3. [`complete_session`](crate::context::WorkflowContext::complete_session) runs an
//!    internal **completion activity** on the resource task list to release the token.
//!
//! Concurrency is bounded by the session worker's token bucket
//! (`max_concurrent_session_execution_size`): `create_session` fails with
//! "too many outstanding sessions" once the worker is full.
//!
//! > **Simplification vs. Go:** the creation activity here signals back and returns
//! > rather than long-running-heartbeating, so worker-death is not auto-detected as
//! > a failed session. Tracked as a follow-up; the create/route/complete + bounded
//! > concurrency contract is complete.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Name of the internal activity that establishes a session on a worker.
pub const SESSION_CREATION_ACTIVITY: &str = "internalSessionCreationActivity";
/// Name of the internal activity that tears a session down.
pub const SESSION_COMPLETION_ACTIVITY: &str = "internalSessionCompletionActivity";

/// The task list suffix a session's creation activity is scheduled on.
pub const SESSION_CREATION_TASKLIST_SUFFIX: &str = "__internal_session_creation";

/// The creation task list derived from a base task list.
pub fn session_creation_tasklist(base: &str) -> String {
    format!("{base}{SESSION_CREATION_TASKLIST_SUFFIX}")
}

/// The host-specific resource task list a session's activities are pinned to.
pub fn resource_specific_tasklist(resource_id: &str, host: &str) -> String {
    format!("{resource_id}@{host}")
}

/// Options for creating a session (Go's `SessionOptions`).
#[derive(Debug, Clone)]
pub struct SessionOptions {
    /// Maximum time the whole session may run.
    pub execution_timeout: Duration,
    /// Maximum time session creation may take before failing.
    pub creation_timeout: Duration,
    /// Heartbeat timeout for the creation activity (default 20s).
    pub heartbeat_timeout: Duration,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            execution_timeout: Duration::from_secs(60),
            creation_timeout: Duration::from_secs(60),
            heartbeat_timeout: Duration::from_secs(20),
        }
    }
}

/// Lifecycle state of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Open,
    Failed,
    Closed,
}

/// Information about a created session.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Unique session id (a `SideEffect`-stable uuid).
    pub session_id: String,
    /// The worker host running the session.
    pub host_name: String,
    /// The resource the session is bound to.
    pub resource_id: String,
    /// The host-specific task list the session's activities are pinned to.
    pub tasklist: String,
    /// Current lifecycle state.
    pub state: SessionState,
}

impl SessionInfo {
    /// A token that [`recreate_session`](crate::context::WorkflowContext::recreate_session)
    /// can use to re-establish a session on the same worker (carries the resource
    /// task list, as in Go's `GetRecreateToken`).
    pub fn recreate_token(&self) -> Vec<u8> {
        serde_json::to_vec(&RecreateToken {
            tasklist: self.tasklist.clone(),
        })
        .unwrap_or_default()
    }
}

/// The payload the creation activity signals back to the workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreationResponse {
    pub tasklist: String,
    pub host_name: String,
    pub resource_id: String,
}

/// The serialized form of a recreate token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RecreateToken {
    pub tasklist: String,
}
