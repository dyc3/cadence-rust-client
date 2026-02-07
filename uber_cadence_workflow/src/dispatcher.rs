//! Deterministic task scheduler for workflow execution.
//!
//! The WorkflowDispatcher manages the execution of spawned workflow tasks
//! in a deterministic manner by polling tasks in creation order until all
//! tasks are blocked.
//!
//! This ensures that workflow execution is reproducible during replay.

use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to be polled
    Ready,
    /// Task is blocked waiting for something
    Blocked,
    /// Task has completed
    Completed,
}

/// A workflow task that can be executed by the dispatcher
pub struct WorkflowTask {
    /// Unique task ID
    pub id: u64,
    /// Task name for debugging
    pub name: String,
    /// The future to execute
    future: Pin<Box<dyn Future<Output = Box<dyn Any + Send>> + Send>>,
    /// Current state of the task
    state: TaskState,
}

impl WorkflowTask {
    /// Create a new workflow task
    pub fn new<F, T>(id: u64, name: String, future: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // Wrap the future to type-erase its output
        let boxed = Box::pin(async move {
            let result = future.await;
            Box::new(result) as Box<dyn Any + Send>
        });

        Self {
            id,
            name,
            future: boxed,
            state: TaskState::Ready,
        }
    }

    /// Poll the task's future
    fn poll(&mut self, waker: &Waker) -> Poll<Box<dyn Any + Send>> {
        let mut cx = Context::from_waker(waker);
        self.future.as_mut().poll(&mut cx)
    }
}

/// Dispatcher error type
#[derive(Debug, Clone)]
pub enum DispatcherError {
    /// Task not found
    TaskNotFound(u64),
    /// Task already completed
    TaskAlreadyCompleted(u64),
    /// Generic error
    Generic(String),
}

impl std::fmt::Display for DispatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatcherError::TaskNotFound(id) => write!(f, "task {} not found", id),
            DispatcherError::TaskAlreadyCompleted(id) => {
                write!(f, "task {} already completed", id)
            }
            DispatcherError::Generic(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for DispatcherError {}

/// Deterministic workflow task dispatcher
///
/// The dispatcher maintains a list of tasks and executes them in a deterministic
/// order by polling tasks in creation order until all are blocked.
pub struct WorkflowDispatcher {
    /// List of tasks in creation order
    tasks: Vec<WorkflowTask>,
    /// Task sequence counter
    sequence: u64,
    /// Whether the dispatcher is currently executing
    executing: bool,
    /// Results of completed tasks
    completed_results: Arc<Mutex<HashMap<u64, Box<dyn Any + Send>>>>,
    /// Pending tasks to add (allows spawning while executing)
    pub(crate) pending_tasks: Arc<Mutex<Vec<WorkflowTask>>>,
}

impl WorkflowDispatcher {
    /// Create a new workflow dispatcher
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            sequence: 0,
            executing: false,
            completed_results: Arc::new(Mutex::new(HashMap::new())),
            pending_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get the next task ID
    pub fn next_task_id(&mut self) -> u64 {
        let id = self.sequence;
        self.sequence += 1;
        id
    }

    /// Spawn a new task
    ///
    /// Returns the task ID. If the dispatcher is currently executing,
    /// the task is added to a pending queue and will be added on the next iteration.
    pub fn spawn_task(&mut self, task: WorkflowTask) -> u64 {
        let id = task.id;
        if self.executing {
            // Add to pending queue if we're in the middle of execution
            let mut pending = self.pending_tasks.lock().unwrap();
            pending.push(task);
        } else {
            // Add directly if not executing
            self.tasks.push(task);
        }
        id
    }

    /// Check if a task is complete
    pub fn is_task_complete(&self, task_id: u64) -> bool {
        let results = self.completed_results.lock().unwrap();
        results.contains_key(&task_id)
    }

    /// Get the result of a completed task
    pub fn get_task_result(&self, task_id: u64) -> Option<Box<dyn Any + Send>> {
        let mut results = self.completed_results.lock().unwrap();
        results.remove(&task_id)
    }

    /// Get a clone of the completed_results Arc for lock-free access from JoinHandles
    pub fn get_completed_results_arc(&self) -> Arc<Mutex<HashMap<u64, Box<dyn Any + Send>>>> {
        self.completed_results.clone()
    }

    /// Execute tasks until all are blocked
    ///
    /// This is the core scheduling algorithm:
    /// 1. Loop while any task is Ready or just Completed
    /// 2. Poll each Ready task in creation order
    /// 3. Update task states based on poll results
    /// 4. If any task changed state, loop again
    /// 5. Exit when all tasks are Blocked or all tasks completed
    ///
    /// Returns Ok(true) if all tasks completed, Ok(false) if some tasks are still blocked
    pub fn execute_until_all_blocked(&mut self) -> Result<bool, DispatcherError> {
        if self.executing {
            return Err(DispatcherError::Generic(
                "Dispatcher is already executing".to_string(),
            ));
        }

        self.executing = true;

        // Create a no-op waker since we're doing manual polling
        let waker = create_noop_waker();

        loop {
            // Add any pending tasks that were spawned during execution
            {
                let mut pending = self.pending_tasks.lock().unwrap();
                if !pending.is_empty() {
                    println!(
                        "[Dispatcher] Adding {} pending spawned tasks",
                        pending.len()
                    );
                    self.tasks.append(&mut *pending);
                }
            }

            let mut any_changed = false;

            // Poll each task in creation order (including blocked ones - they may have unblocked)
            for task in &mut self.tasks {
                match task.state {
                    TaskState::Completed => {
                        // Skip completed tasks
                        continue;
                    }
                    TaskState::Ready | TaskState::Blocked => {
                        // Poll the task (it might have been woken by a channel operation)
                        println!("[Dispatcher] Polling task {} ({})", task.id, task.name);
                        match task.poll(&waker) {
                            Poll::Ready(result) => {
                                // Task completed
                                if task.state != TaskState::Completed {
                                    println!(
                                        "[Dispatcher] Task {} ({}) COMPLETED",
                                        task.id, task.name
                                    );
                                    task.state = TaskState::Completed;
                                    any_changed = true;

                                    // Store result
                                    let mut results = self.completed_results.lock().unwrap();
                                    results.insert(task.id, result);
                                    println!(
                                        "[Dispatcher] Stored result for task {} (now {} results)",
                                        task.id,
                                        results.len()
                                    );
                                }
                            }
                            Poll::Pending => {
                                // Task is blocked
                                if task.state != TaskState::Blocked {
                                    println!(
                                        "[Dispatcher] Task {} ({}) BLOCKED",
                                        task.id, task.name
                                    );
                                    task.state = TaskState::Blocked;
                                    any_changed = true;
                                } else {
                                    println!(
                                        "[Dispatcher] Task {} ({}) still blocked",
                                        task.id, task.name
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // If nothing changed state, all tasks are stable (either blocked or completed)
            println!(
                "[Dispatcher] Loop iteration complete. any_changed={}, task_count={}",
                any_changed,
                self.tasks.len()
            );
            if !any_changed {
                println!("[Dispatcher] No state changes, exiting loop");
                break;
            }
        }

        self.executing = false;

        // Check if all tasks completed
        let all_done = self.tasks.iter().all(|t| t.state == TaskState::Completed);

        Ok(all_done)
    }

    /// Wake a specific task (mark it as Ready)
    ///
    /// This is used by channels and other coordination primitives to unblock tasks
    pub fn wake_task(&mut self, task_id: u64) -> Result<(), DispatcherError> {
        for task in &mut self.tasks {
            if task.id == task_id {
                if task.state == TaskState::Completed {
                    return Err(DispatcherError::TaskAlreadyCompleted(task_id));
                }
                task.state = TaskState::Ready;
                return Ok(());
            }
        }
        Err(DispatcherError::TaskNotFound(task_id))
    }

    /// Get the number of tasks
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get the number of ready tasks
    pub fn ready_task_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.state == TaskState::Ready)
            .count()
    }

    /// Get the number of blocked tasks
    pub fn blocked_task_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.state == TaskState::Blocked)
            .count()
    }

    /// Get the number of completed tasks
    pub fn completed_task_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.state == TaskState::Completed)
            .count()
    }
}

impl Default for WorkflowDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a no-op waker for manual polling
///
/// Since we control the execution loop and poll manually, we don't need
/// actual wake functionality. Tasks are woken by the dispatcher when
/// channel operations complete.
fn create_noop_waker() -> Waker {
    unsafe fn noop_clone(_data: *const ()) -> RawWaker {
        noop_raw_waker()
    }

    unsafe fn noop(_data: *const ()) {}

    fn noop_raw_waker() -> RawWaker {
        RawWaker::new(
            std::ptr::null(),
            &RawWakerVTable::new(noop_clone, noop, noop, noop),
        )
    }

    unsafe { Waker::from_raw(noop_raw_waker()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher_basic_task() {
        let mut dispatcher = WorkflowDispatcher::new();

        let task = WorkflowTask::new(0, "test".to_string(), async { 42 });
        dispatcher.spawn_task(task);

        let all_done = dispatcher.execute_until_all_blocked().unwrap();
        assert!(all_done);
        assert_eq!(dispatcher.completed_task_count(), 1);
    }

    #[test]
    fn test_dispatcher_multiple_tasks() {
        let mut dispatcher = WorkflowDispatcher::new();

        let task1 = WorkflowTask::new(0, "task1".to_string(), async { 1 });
        let task2 = WorkflowTask::new(1, "task2".to_string(), async { 2 });
        let task3 = WorkflowTask::new(2, "task3".to_string(), async { 3 });

        dispatcher.spawn_task(task1);
        dispatcher.spawn_task(task2);
        dispatcher.spawn_task(task3);

        let all_done = dispatcher.execute_until_all_blocked().unwrap();
        assert!(all_done);
        assert_eq!(dispatcher.completed_task_count(), 3);
    }

    #[tokio::test]
    async fn test_dispatcher_with_pending_task() {
        let mut dispatcher = WorkflowDispatcher::new();

        // Create a task that will pend forever
        let task = WorkflowTask::new(0, "pending".to_string(), async {
            std::future::pending::<()>().await;
            42
        });

        dispatcher.spawn_task(task);

        let all_done = dispatcher.execute_until_all_blocked().unwrap();
        assert!(!all_done);
        assert_eq!(dispatcher.blocked_task_count(), 1);
        assert_eq!(dispatcher.completed_task_count(), 0);
    }

    #[test]
    fn test_dispatcher_task_ordering() {
        let mut dispatcher = WorkflowDispatcher::new();

        // Tasks that complete immediately
        let task1 = WorkflowTask::new(0, "task1".to_string(), async { "first" });
        let task2 = WorkflowTask::new(1, "task2".to_string(), async { "second" });
        let task3 = WorkflowTask::new(2, "task3".to_string(), async { "third" });

        dispatcher.spawn_task(task1);
        dispatcher.spawn_task(task2);
        dispatcher.spawn_task(task3);

        dispatcher.execute_until_all_blocked().unwrap();

        // Results should be retrievable in any order
        let result1 = dispatcher.get_task_result(0).unwrap();
        let result2 = dispatcher.get_task_result(1).unwrap();
        let result3 = dispatcher.get_task_result(2).unwrap();

        assert_eq!(*result1.downcast::<&str>().unwrap(), "first");
        assert_eq!(*result2.downcast::<&str>().unwrap(), "second");
        assert_eq!(*result3.downcast::<&str>().unwrap(), "third");
    }

    #[test]
    fn test_is_task_complete() {
        let mut dispatcher = WorkflowDispatcher::new();

        let task = WorkflowTask::new(0, "test".to_string(), async { 42 });
        dispatcher.spawn_task(task);

        assert!(!dispatcher.is_task_complete(0));

        dispatcher.execute_until_all_blocked().unwrap();

        assert!(dispatcher.is_task_complete(0));
    }

    #[test]
    fn test_next_task_id() {
        let mut dispatcher = WorkflowDispatcher::new();

        assert_eq!(dispatcher.next_task_id(), 0);
        assert_eq!(dispatcher.next_task_id(), 1);
        assert_eq!(dispatcher.next_task_id(), 2);
    }
}
