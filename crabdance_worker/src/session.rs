//! Worker-side session support: the token bucket that bounds concurrent sessions
//! and the per-worker session environment.
//!
//! See [`crabdance_workflow::session`] for the workflow-facing API and the overall
//! mechanism. The session worker is enabled via
//! [`WorkerOptions::enable_session_worker`](crate::WorkerOptions::enable_session_worker);
//! when on, the worker registers two extra activity pollers (the creation task list
//! and the host-specific resource task list) and special-cases the internal
//! creation/completion activities (see the activity handler).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use crabdance_workflow::resource_specific_tasklist;

/// Bounds the number of concurrently-running sessions on a worker. Each
/// `create_session` consumes a token (held until `complete_session`), so once the
/// bucket is empty further session creation fails with "too many outstanding
/// sessions" — Go's `sessionTokenBucket`.
pub struct SessionTokenBucket {
    sem: Semaphore,
    max: usize,
}

impl SessionTokenBucket {
    pub fn new(max: usize) -> Self {
        Self {
            sem: Semaphore::new(max),
            max,
        }
    }

    /// Take a token without blocking. Returns `false` if the bucket is empty.
    pub fn try_acquire(&self) -> bool {
        match self.sem.try_acquire() {
            Ok(permit) => {
                // Hold the token until an explicit `release` (on session completion).
                permit.forget();
                true
            }
            Err(_) => false,
        }
    }

    /// Return a token to the bucket (never exceeds the configured maximum).
    pub fn release(&self) {
        if self.sem.available_permits() < self.max {
            self.sem.add_permits(1);
        }
    }

    /// Currently-available tokens.
    pub fn available(&self) -> usize {
        self.sem.available_permits()
    }
}

/// Per-worker session state: the resource it owns, the host-specific task list its
/// sessions pin to, and the concurrency token bucket.
pub struct SessionEnvironment {
    pub resource_id: String,
    pub host: String,
    pub resource_tasklist: String,
    bucket: SessionTokenBucket,
    /// Session ids currently holding a token. Makes acquire/release exactly-once
    /// per session id, so a Cadence retry of the creation/completion activity does
    /// not double-acquire or over-release.
    active_sessions: Mutex<HashSet<String>>,
}

impl SessionEnvironment {
    pub fn new(resource_id: impl Into<String>, max_sessions: usize) -> Arc<Self> {
        let resource_id = resource_id.into();
        let host = hostname();
        let resource_tasklist = resource_specific_tasklist(&resource_id, &host);
        Arc::new(Self {
            resource_id,
            host,
            resource_tasklist,
            bucket: SessionTokenBucket::new(max_sessions),
            active_sessions: Mutex::new(HashSet::new()),
        })
    }

    /// Acquire a token for `session_id`, idempotently. Returns `true` if the session
    /// holds a token afterward (already-held sessions return `true` without taking a
    /// second token); `false` when the bucket is full.
    pub fn acquire(&self, session_id: &str) -> bool {
        let mut active = self.active_sessions.lock().unwrap();
        if active.contains(session_id) {
            return true;
        }
        if self.bucket.try_acquire() {
            active.insert(session_id.to_string());
            true
        } else {
            false
        }
    }

    /// Release `session_id`'s token, idempotently (a no-op if it holds none).
    pub fn release(&self, session_id: &str) {
        let mut active = self.active_sessions.lock().unwrap();
        if active.remove(session_id) {
            self.bucket.release();
        }
    }

    /// Currently-available session tokens.
    pub fn available_tokens(&self) -> usize {
        self.bucket.available()
    }
}

/// The worker's host name, used to build the resource-specific task list.
pub fn hostname() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_bounds_concurrency() {
        let bucket = SessionTokenBucket::new(2);
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire(), "third session exceeds the bound");
        assert_eq!(bucket.available(), 0);

        bucket.release();
        assert_eq!(bucket.available(), 1);
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire());
    }

    #[test]
    fn test_release_never_exceeds_max() {
        let bucket = SessionTokenBucket::new(1);
        bucket.release();
        bucket.release();
        assert_eq!(bucket.available(), 1, "release must not grow past max");
    }

    #[test]
    fn test_resource_tasklist_is_host_specific() {
        let env = SessionEnvironment::new("gpu-0", 4);
        assert_eq!(env.resource_tasklist, format!("gpu-0@{}", env.host));
        assert_eq!(env.available_tokens(), 4);
    }

    #[test]
    fn test_env_acquire_release_is_idempotent_per_session() {
        let env = SessionEnvironment::new("res", 2);

        // Re-acquiring the same session id does not take a second token (retry-safe).
        assert!(env.acquire("s1"));
        assert!(env.acquire("s1"));
        assert_eq!(env.available_tokens(), 1);

        assert!(env.acquire("s2"));
        assert!(!env.acquire("s3"), "bucket is full");

        // Releasing twice frees exactly one token.
        env.release("s1");
        env.release("s1");
        assert_eq!(env.available_tokens(), 1);
        assert!(env.acquire("s3"));
    }
}
