//! Heartbeat manager for long-running activities.

use uber_cadence_core::CadenceError;
use uber_cadence_proto::workflow_service::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};

/// Heartbeat manager for activities
pub struct HeartbeatManager {
    service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync>,
    identity: String,
}

impl HeartbeatManager {
    pub fn new(
        service: Arc<dyn WorkflowService<Error = CadenceError> + Send + Sync>,
        identity: String,
    ) -> Self {
        Self { service, identity }
    }

    /// Start a heartbeat task for an activity
    ///
    /// The heartbeat will be sent at the specified interval until either:
    /// - The cancel channel is signaled (activity completes)
    /// - The server requests cancellation
    pub fn start_heartbeat(
        &self,
        task_token: Vec<u8>,
        heartbeat_interval: Duration,
        cancel_rx: oneshot::Receiver<()>,
        on_server_cancel: Option<tokio::sync::broadcast::Sender<()>>,
        details: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> tokio::task::JoinHandle<()> {
        let service = Arc::clone(&self.service);
        let identity = self.identity.clone();

        // Calculate heartbeat interval (typically 80% of heartbeat timeout)
        let interval = heartbeat_interval.mul_f32(0.8);

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            let mut cancel_rx = cancel_rx;

            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        // Get latest details
                        let current_details = {
                            let guard = details.lock().await;
                            guard.clone()
                        };

                        // Send heartbeat
                        let request = RecordActivityTaskHeartbeatRequest {
                            task_token: task_token.clone(),
                            details: current_details,
                            identity: identity.clone(),
                        };

                        match service.record_activity_task_heartbeat(request).await {
                            Ok(response) => {
                                if response.cancel_requested {
                                    tracing::info!("Activity cancellation requested by server");
                                    // Cancellation requested - signal activity to stop
                                    if let Some(tx) = &on_server_cancel {
                                        let _ = tx.send(());
                                    }
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Heartbeat failed: {}", e);
                                // Continue sending heartbeats even if one fails
                            }
                        }
                    }
                    _ = &mut cancel_rx => {
                        // Activity completed or cancelled
                        tracing::debug!("Heartbeat task cancelled - activity completed");
                        break;
                    }
                }
            }
        })
    }
}
