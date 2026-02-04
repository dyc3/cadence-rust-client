//! Local activity task queue for in-process execution.
//!
//! This module provides the in-memory task queue for local activities,
//! enabling communication between the workflow execution and local activity
//! executor.

use crate::registry::ActivityError;
use cadence_core::{Header, WorkflowInfo};
use cadence_workflow::LocalActivityOptions;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, oneshot, Mutex};

/// A task representing a local activity execution request
#[derive(Debug)]
pub struct LocalActivityTask {
    /// Unique identifier for this activity execution
    pub activity_id: String,

    /// Type/name of the activity to execute
    pub activity_type: String,

    /// Serialized activity arguments
    pub args: Option<Vec<u8>>,

    /// Local activity execution options
    pub options: LocalActivityOptions,

    /// Workflow context information
    pub workflow_info: WorkflowInfo,

    /// Header for context propagation (tracing, etc.)
    pub header: Option<Header>,

    /// Current attempt number (starting from 0)
    pub attempt: i32,

    /// Time when the activity was scheduled
    pub scheduled_time: SystemTime,

    /// Channel to send the result back to the workflow
    pub result_sender: oneshot::Sender<Result<Vec<u8>, ActivityError>>,
}

/// In-memory queue for local activity tasks
///
/// This queue decouples the workflow execution from the local activity
/// executor, allowing tasks to be submitted and processed asynchronously.
#[derive(Clone)]
pub struct LocalActivityQueue {
    sender: mpsc::UnboundedSender<LocalActivityTask>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<LocalActivityTask>>>,
}

impl LocalActivityQueue {
    /// Create a new local activity queue
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Send a local activity task to the queue
    ///
    /// Returns an error if the receiver has been dropped.
    #[allow(clippy::result_large_err)]
    pub fn send(&self, task: LocalActivityTask) -> Result<(), LocalActivityTask> {
        self.sender.send(task).map_err(|e| e.0)
    }

    /// Receive a local activity task from the queue
    ///
    /// This method blocks until a task is available or the queue is closed.
    /// Returns `None` when all senders have been dropped.
    pub async fn recv(&self) -> Option<LocalActivityTask> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }

    /// Get the sender for this queue
    ///
    /// This can be useful for cloning just the sender side.
    pub fn sender(&self) -> mpsc::UnboundedSender<LocalActivityTask> {
        self.sender.clone()
    }
}

impl Default for LocalActivityQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadence_core::{WorkflowExecution, WorkflowType};
    use std::time::Duration;

    fn create_test_workflow_info() -> WorkflowInfo {
        WorkflowInfo {
            workflow_execution: WorkflowExecution {
                workflow_id: "test-workflow".to_string(),
                run_id: "test-run".to_string(),
            },
            workflow_type: WorkflowType {
                name: "TestWorkflow".to_string(),
            },
            task_list: "test-task-list".to_string(),
            start_time: chrono::Utc::now(),
            execution_start_to_close_timeout: Duration::from_secs(3600),
            task_start_to_close_timeout: Duration::from_secs(10),
            attempt: 1,
            continued_execution_run_id: None,
            parent_workflow_execution: None,
            cron_schedule: None,
            memo: None,
            search_attributes: None,
        }
    }

    #[tokio::test]
    async fn test_send_recv() {
        let queue = LocalActivityQueue::new();
        let (tx, _rx) = oneshot::channel();

        let task = LocalActivityTask {
            activity_id: "test-1".to_string(),
            activity_type: "TestActivity".to_string(),
            args: Some(b"test args".to_vec()),
            options: LocalActivityOptions {
                schedule_to_close_timeout: Duration::from_secs(10),
                retry_policy: None,
            },
            workflow_info: create_test_workflow_info(),
            header: None,
            attempt: 0,
            scheduled_time: SystemTime::now(),
            result_sender: tx,
        };

        queue.send(task).expect("Failed to send task");

        let received = queue.recv().await.expect("Failed to receive task");
        assert_eq!(received.activity_id, "test-1");
        assert_eq!(received.activity_type, "TestActivity");
        assert_eq!(received.attempt, 0);
    }

    #[tokio::test]
    async fn test_multiple_tasks() {
        let queue = LocalActivityQueue::new();

        for i in 0..5 {
            let (tx, _rx) = oneshot::channel();
            let task = LocalActivityTask {
                activity_id: format!("test-{}", i),
                activity_type: "TestActivity".to_string(),
                args: None,
                options: LocalActivityOptions {
                    schedule_to_close_timeout: Duration::from_secs(10),
                    retry_policy: None,
                },
                workflow_info: create_test_workflow_info(),
                header: None,
                attempt: 0,
                scheduled_time: SystemTime::now(),
                result_sender: tx,
            };
            queue.send(task).expect("Failed to send task");
        }

        for i in 0..5 {
            let task = queue.recv().await.expect("Failed to receive task");
            assert_eq!(task.activity_id, format!("test-{}", i));
        }
    }

    #[tokio::test]
    async fn test_clone_queue() {
        let queue = LocalActivityQueue::new();
        let queue_clone = queue.clone();

        let (tx, _rx) = oneshot::channel();
        let task = LocalActivityTask {
            activity_id: "test-1".to_string(),
            activity_type: "TestActivity".to_string(),
            args: None,
            options: LocalActivityOptions {
                schedule_to_close_timeout: Duration::from_secs(10),
                retry_policy: None,
            },
            workflow_info: create_test_workflow_info(),
            header: None,
            attempt: 0,
            scheduled_time: SystemTime::now(),
            result_sender: tx,
        };

        // Send on the clone
        queue_clone.send(task).expect("Failed to send task");

        // Receive on the original
        let received = queue.recv().await.expect("Failed to receive task");
        assert_eq!(received.activity_id, "test-1");
    }
}
