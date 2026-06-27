//! Sticky workflow execution cache.
//!
//! The worker keeps decided-but-not-completed workflow executions in memory so a
//! follow-up decision task can resume from the cached dispatcher state instead of
//! replaying the whole history (Go's "sticky" cache). Two policies bound memory
//! growth under churn:
//!
//! * **LRU capacity** — at most `max_size` executions are retained; inserting past
//!   capacity evicts the least-recently-used entry.
//! * **Idle expiration** (optional) — an entry untouched for longer than the
//!   configured idle TTL is dropped, even if capacity has not been reached. This
//!   matches Go's `StickyWorkflowCacheSize` + sticky-cache TTL behavior and keeps
//!   abandoned executions from pinning memory until churn evicts them.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::executor::workflow::WorkflowExecution;

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct WorkflowExecutionKey {
    pub workflow_id: String,
    pub run_id: String,
}

pub struct CachedWorkflow {
    pub execution: Arc<Mutex<WorkflowExecution>>,
    pub last_event_id: i64,
}

/// Monotonic clock seam so idle expiration can be tested deterministically.
trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

struct Entry {
    value: CachedWorkflow,
    last_access: Instant,
}

struct CacheInner {
    map: HashMap<WorkflowExecutionKey, Entry>,
    access_order: VecDeque<WorkflowExecutionKey>,
}

pub struct WorkflowCache {
    inner: Mutex<CacheInner>,
    max_size: usize,
    /// Drop entries untouched for longer than this. `None` disables idle expiration.
    idle_ttl: Option<Duration>,
    clock: Arc<dyn Clock>,
}

impl WorkflowCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: Mutex::new(CacheInner {
                map: HashMap::new(),
                access_order: VecDeque::new(),
            }),
            max_size,
            idle_ttl: None,
            clock: Arc::new(SystemClock),
        }
    }

    /// Enable idle expiration: entries untouched for `ttl` are evicted lazily on the
    /// next cache operation. A zero or absent TTL disables idle expiration.
    pub fn with_idle_ttl(mut self, ttl: Duration) -> Self {
        self.idle_ttl = if ttl.is_zero() { None } else { Some(ttl) };
        self
    }

    pub fn get(&self, key: &WorkflowExecutionKey) -> Option<CachedWorkflow> {
        let mut inner = self.inner.lock().unwrap();
        let now = self.clock.now();
        self.expire_idle(&mut inner, now);

        let entry = inner.map.get_mut(key)?;
        entry.last_access = now;
        let found = CachedWorkflow {
            execution: entry.value.execution.clone(),
            last_event_id: entry.value.last_event_id,
        };

        // Update LRU recency.
        if let Some(pos) = inner.access_order.iter().position(|k| k == key) {
            inner.access_order.remove(pos);
            inner.access_order.push_back(key.clone());
        }

        Some(found)
    }

    pub fn insert(&self, key: WorkflowExecutionKey, value: CachedWorkflow) {
        let mut inner = self.inner.lock().unwrap();
        let now = self.clock.now();
        self.expire_idle(&mut inner, now);

        // Remove existing if present to update order.
        if inner.map.contains_key(&key) {
            if let Some(pos) = inner.access_order.iter().position(|k| k == &key) {
                inner.access_order.remove(pos);
            }
        } else if inner.map.len() >= self.max_size && self.max_size > 0 {
            // Capacity reached — evict least-recently-used.
            if let Some(oldest) = inner.access_order.pop_front() {
                inner.map.remove(&oldest);
            }
        }

        inner.access_order.push_back(key.clone());
        inner.map.insert(
            key,
            Entry {
                value,
                last_access: now,
            },
        );
    }

    pub fn remove(&self, key: &WorkflowExecutionKey) {
        let mut inner = self.inner.lock().unwrap();
        if inner.map.remove(key).is_some() {
            if let Some(pos) = inner.access_order.iter().position(|k| k == key) {
                inner.access_order.remove(pos);
            }
        }
    }

    /// Current number of cached executions (after no implicit eviction).
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Evict every entry whose idle interval has exceeded the configured TTL.
    fn expire_idle(&self, inner: &mut CacheInner, now: Instant) {
        let Some(ttl) = self.idle_ttl else {
            return;
        };
        let expired: Vec<WorkflowExecutionKey> = inner
            .map
            .iter()
            .filter(|(_, e)| now.duration_since(e.last_access) >= ttl)
            .map(|(k, _)| k.clone())
            .collect();
        for key in expired {
            inner.map.remove(&key);
            if let Some(pos) = inner.access_order.iter().position(|k| k == &key) {
                inner.access_order.remove(pos);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test clock whose `now()` is driven by the test.
    struct TestClock(Mutex<Instant>);
    impl TestClock {
        fn new(base: Instant) -> Arc<Self> {
            Arc::new(Self(Mutex::new(base)))
        }
        fn advance(&self, by: Duration) {
            let mut t = self.0.lock().unwrap();
            *t += by;
        }
    }
    impl Clock for TestClock {
        fn now(&self) -> Instant {
            *self.0.lock().unwrap()
        }
    }

    fn key(id: &str) -> WorkflowExecutionKey {
        WorkflowExecutionKey {
            workflow_id: id.to_string(),
            run_id: "run".to_string(),
        }
    }

    fn dummy() -> CachedWorkflow {
        use crate::executor::replay::ReplayEngine;
        use crabdance_core::WorkflowInfo;

        let info = WorkflowInfo {
            workflow_execution: crabdance_core::WorkflowExecution {
                workflow_id: "wf".to_string(),
                run_id: "run".to_string(),
            },
            workflow_type: crabdance_core::WorkflowType {
                name: "Test".to_string(),
            },
            task_list: "tl".to_string(),
            start_time: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap(),
            execution_start_to_close_timeout: std::time::Duration::from_secs(60),
            task_start_to_close_timeout: std::time::Duration::from_secs(10),
            attempt: 0,
            continued_execution_run_id: None,
            parent_workflow_execution: None,
            cron_schedule: None,
            memo: None,
            search_attributes: None,
        };
        CachedWorkflow {
            execution: Arc::new(Mutex::new(WorkflowExecution {
                engine: Arc::new(Mutex::new(ReplayEngine::new())),
                workflow_info: info,
            })),
            last_event_id: 0,
        }
    }

    fn with_clock(max_size: usize, clock: Arc<dyn Clock>) -> WorkflowCache {
        WorkflowCache {
            inner: Mutex::new(CacheInner {
                map: HashMap::new(),
                access_order: VecDeque::new(),
            }),
            max_size,
            idle_ttl: None,
            clock,
        }
    }

    #[test]
    fn test_lru_eviction_under_churn_bounds_size() {
        let cache = WorkflowCache::new(3);
        for i in 0..100 {
            cache.insert(key(&format!("wf-{i}")), dummy());
            assert!(cache.len() <= 3, "cache exceeded capacity under churn");
        }
        // Only the three most recent survive.
        assert_eq!(cache.len(), 3);
        assert!(cache.get(&key("wf-99")).is_some());
        assert!(cache.get(&key("wf-0")).is_none());
    }

    #[test]
    fn test_lru_evicts_least_recently_used_not_oldest_inserted() {
        let cache = WorkflowCache::new(2);
        cache.insert(key("a"), dummy());
        cache.insert(key("b"), dummy());
        // Touch "a" so "b" becomes the LRU victim.
        assert!(cache.get(&key("a")).is_some());
        cache.insert(key("c"), dummy());

        assert!(
            cache.get(&key("a")).is_some(),
            "recently-used entry survived"
        );
        assert!(cache.get(&key("b")).is_none(), "LRU entry evicted");
        assert!(cache.get(&key("c")).is_some());
    }

    #[test]
    fn test_idle_expiration_drops_untouched_entries() {
        let base = Instant::now();
        let clock = TestClock::new(base);
        let cache = with_clock(10, clock.clone()).with_idle_ttl(Duration::from_secs(60));

        cache.insert(key("a"), dummy());
        cache.insert(key("b"), dummy());

        // Advance past the TTL with no access — both should expire on next op.
        clock.advance(Duration::from_secs(61));
        cache.insert(key("c"), dummy());

        assert!(cache.get(&key("a")).is_none(), "idle entry expired");
        assert!(cache.get(&key("b")).is_none(), "idle entry expired");
        assert!(cache.get(&key("c")).is_some());
    }

    #[test]
    fn test_access_refreshes_idle_deadline() {
        let base = Instant::now();
        let clock = TestClock::new(base);
        let cache = with_clock(10, clock.clone()).with_idle_ttl(Duration::from_secs(60));

        cache.insert(key("a"), dummy());

        // Halfway to the deadline, touch "a" to refresh it.
        clock.advance(Duration::from_secs(40));
        assert!(cache.get(&key("a")).is_some());

        // Another 40s: 80s since insert, but only 40s since last access.
        clock.advance(Duration::from_secs(40));
        assert!(
            cache.get(&key("a")).is_some(),
            "access refreshed the idle deadline"
        );
    }

    #[test]
    fn test_idle_ttl_disabled_by_default() {
        let cache = WorkflowCache::new(10);
        assert!(cache.idle_ttl.is_none());
    }
}
