use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u32,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: u32,
    pub result: String,
}

/// Process a job (simulates some work)
pub async fn process_job(_ctx: &ActivityContext, job: Job) -> Result<JobResult, ActivityError> {
    // Simulate some processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(JobResult {
        job_id: job.id,
        result: format!("Processed: {}", job.data),
    })
}

/// Fast processing activity for testing
pub async fn fast_process(_ctx: &ActivityContext, value: u32) -> Result<u32, ActivityError> {
    Ok(value * 2)
}
