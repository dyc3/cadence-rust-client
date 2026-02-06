// CPU-intensive activity

use std::pin::Pin;
use cadence_activity::ActivityContext;
use cadence_worker::registry::{Activity, ActivityError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInput {
    pub id: usize,
    pub iterations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuOutput {
    pub id: usize,
    pub result: u64,
}

/// A CPU-intensive activity that performs computation
/// 
/// This activity performs CPU-bound work with configurable iterations
/// to simulate heavy computational load.
#[derive(Clone)]
pub struct CpuIntensiveActivity;

impl Activity for CpuIntensiveActivity {
    fn execute(
        &self,
        _ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        Box::pin(async move {
            let input_bytes =
                input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
            let input: CpuInput = serde_json::from_slice(&input_bytes)
                .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))?;
            
            // Perform CPU-intensive work
            let mut result: u64 = 0;
            for i in 0..input.iterations {
                result = result.wrapping_add((i as u64).wrapping_mul(17).wrapping_add(23));
            }
            
            let output = CpuOutput {
                id: input.id,
                result,
            };
            
            serde_json::to_vec(&output).map_err(|e| ActivityError::ExecutionFailed(e.to_string()))
        })
    }
}
