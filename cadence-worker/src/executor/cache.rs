use crate::executor::workflow::WorkflowExecution;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct WorkflowExecutionKey {
    pub workflow_id: String,
    pub run_id: String,
}

pub struct CachedWorkflow {
    pub execution: Arc<Mutex<WorkflowExecution>>,
    pub last_event_id: i64,
}

struct CacheInner {
    map: HashMap<WorkflowExecutionKey, CachedWorkflow>,
    access_order: VecDeque<WorkflowExecutionKey>,
}

pub struct WorkflowCache {
    inner: Mutex<CacheInner>,
    max_size: usize,
}

impl WorkflowCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: Mutex::new(CacheInner {
                map: HashMap::new(),
                access_order: VecDeque::new(),
            }),
            max_size,
        }
    }

    pub fn get(&self, key: &WorkflowExecutionKey) -> Option<CachedWorkflow> {
        let mut inner = self.inner.lock().unwrap();

        let found = inner.map.get(key).map(|entry| CachedWorkflow {
            execution: entry.execution.clone(),
            last_event_id: entry.last_event_id,
        });

        if found.is_some() {
            // Update LRU
            if let Some(pos) = inner.access_order.iter().position(|k| k == key) {
                inner.access_order.remove(pos);
                inner.access_order.push_back(key.clone());
            }
        }

        found
    }

    pub fn insert(&self, key: WorkflowExecutionKey, value: CachedWorkflow) {
        let mut inner = self.inner.lock().unwrap();

        // Remove existing if present to update order
        if inner.map.contains_key(&key) {
            if let Some(pos) = inner.access_order.iter().position(|k| k == &key) {
                inner.access_order.remove(pos);
            }
        } else {
            // Check capacity
            if inner.map.len() >= self.max_size && self.max_size > 0 {
                if let Some(oldest) = inner.access_order.pop_front() {
                    inner.map.remove(&oldest);
                }
            }
        }

        inner.access_order.push_back(key.clone());
        inner.map.insert(key, value);
    }

    pub fn remove(&self, key: &WorkflowExecutionKey) {
        let mut inner = self.inner.lock().unwrap();
        if inner.map.remove(key).is_some() {
            if let Some(pos) = inner.access_order.iter().position(|k| k == key) {
                inner.access_order.remove(pos);
            }
        }
    }
}
