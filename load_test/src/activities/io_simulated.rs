// IO-simulated activity

use std::pin::Pin;
use std::time::Duration;
use cadence_activity::ActivityContext;
use cadence_worker::registry::{Activity, ActivityError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoInput {
    pub id: usize,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoOutput {
    pub id: usize,
    pub completed: bool,
}

/// An activity that simulates I/O delays
/// 
/// This activity uses tokio::time::sleep to simulate I/O-bound work
/// such as database queries, API calls, etc.
#[derive(Clone)]
pub struct IoSimulatedActivity;

impl Activity for IoSimulatedActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move {
            let input_bytes =
                input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
            let input: IoInput = serde_json::from_slice(&input_bytes)
                .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))?;
            
            // Simulate I/O delay
            tokio::time::sleep(Duration::from_millis(input.delay_ms)).await;
            
            let output = IoOutput {
                id: input.id,
                completed: true,
            };
            
            serde_json::to_vec(&output).map_err(|e| ActivityError::ExecutionFailed(e.to_string()))
        })
    }
}
