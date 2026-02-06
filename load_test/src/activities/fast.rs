// Fast activity - sub-millisecond execution

use std::pin::Pin;
use cadence_activity::ActivityContext;
use cadence_worker::registry::{Activity, ActivityError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastInput {
    pub id: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastOutput {
    pub id: usize,
    pub result: usize,
}

/// A fast activity that performs minimal computation
/// 
/// This activity completes in less than 1ms and is used to test
/// the overhead of activity scheduling and execution.
#[derive(Clone)]
pub struct FastActivity;

impl Activity for FastActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move {
            let input_bytes = input.unwrap_or_default();
            if input_bytes.is_empty() {
                return Ok(vec![]);
            }
            let input: FastInput = serde_json::from_slice(&input_bytes)
                .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))?;
            
            // Minimal computation - just a simple calculation
            let result = input.id * 2 + 1;
            
            let output = FastOutput {
                id: input.id,
                result,
            };
            
            serde_json::to_vec(&output).map_err(|e| ActivityError::ExecutionFailed(e.to_string()))
        })
    }
}
