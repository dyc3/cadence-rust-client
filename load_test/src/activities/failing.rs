// Failing activity

use std::pin::Pin;
use cadence_activity::ActivityContext;
use cadence_worker::registry::{Activity, ActivityError};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailingInput {
    pub id: usize,
    pub failure_rate: f64, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailingOutput {
    pub id: usize,
    pub attempt: u32,
}

/// An activity that fails based on a configured probability
/// 
/// This activity is used to test error handling and retry logic.
/// It randomly fails based on the failure_rate parameter.
#[derive(Clone)]
pub struct FailingActivity;

impl Activity for FailingActivity {
    fn execute(
        &self,
        ctx: &ActivityContext,
        input: Option<Vec<u8>>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, ActivityError>> + Send>> {
        let activity_info = ctx.get_info().clone();
        Box::pin(async move {
            let input_bytes = input.ok_or_else(|| ActivityError::ExecutionFailed("Missing input".to_string()))?;
            let input: FailingInput = serde_json::from_slice(&input_bytes)
                .map_err(|e| ActivityError::ExecutionFailed(e.to_string()))?;
            
            // Generate random number to determine if we should fail
            let mut rng = rand::thread_rng();
            let roll: f64 = rng.gen();
            
            if roll < input.failure_rate {
                return Err(ActivityError::Retryable(format!(
                    "Activity {} failed (attempt {}): random failure triggered",
                    input.id, activity_info.attempt
                )));
            }
            
            let output = FailingOutput {
                id: input.id,
                attempt: activity_info.attempt as u32,
            };
            
            serde_json::to_vec(&output).map_err(|e| ActivityError::ExecutionFailed(e.to_string()))
        })
    }
}
