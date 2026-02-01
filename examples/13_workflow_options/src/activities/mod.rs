//! Activity implementations for workflow options example.
//!
//! This example demonstrates activities used with various workflow options:
//! - Activities with different timeouts
//! - Cron-compatible activities
//! - Activities that work with memo data

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

/// Report generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    pub report_type: String,
    pub date_range: DateRange,
    pub filters: Vec<String>,
}

/// Date range for reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start_date: String,
    pub end_date: String,
}

/// Report generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportResult {
    pub report_id: String,
    pub generated_at: String,
    pub record_count: usize,
    pub file_path: String,
}

/// Scheduled task input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskInput {
    pub task_name: String,
    pub scheduled_time: i64,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    pub execution_id: String,
    pub task_name: String,
    pub started_at: i64,
    pub completed_at: i64,
    pub status: TaskStatus,
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Success,
    Failed,
    Partial,
}

/// Activity that generates reports (can take variable time)
pub async fn generate_report_activity(
    ctx: &ActivityContext,
    request: ReportRequest,
) -> Result<ReportResult, ActivityError> {
    info!(
        "Generating {} report for {} to {}",
        request.report_type, request.date_range.start_date, request.date_range.end_date
    );
    
    // Simulate variable report generation time based on date range
    let duration_ms = match request.report_type.as_str() {
        "summary" => 500,
        "detailed" => 2000,
        "audit" => 5000,
        _ => 1000,
    };
    
    // Record heartbeat
    ctx.record_heartbeat(None);
    
    tokio::time::sleep(Duration::from_millis(duration_ms)).await;
    
    // Generate report ID
    let report_id = Uuid::new_v4().to_string();
    
    info!("Report {} generated successfully", report_id);
    
    Ok(ReportResult {
        report_id: report_id.clone(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        record_count: (duration_ms / 100) as usize,
        file_path: format!("/reports/{}/{}.pdf", request.report_type, report_id),
    })
}

/// Activity for scheduled/cron tasks
pub async fn scheduled_cleanup_activity(
    _ctx: &ActivityContext,
    input: ScheduledTaskInput,
) -> Result<TaskExecutionResult, ActivityError> {
    info!("Executing scheduled task: {} at {}", input.task_name, input.scheduled_time);
    
    let started_at = chrono::Utc::now().timestamp();
    
    // Simulate cleanup work
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let completed_at = chrono::Utc::now().timestamp();
    
    info!("Scheduled task {} completed", input.task_name);
    
    Ok(TaskExecutionResult {
        execution_id: Uuid::new_v4().to_string(),
        task_name: input.task_name,
        started_at,
        completed_at,
        status: TaskStatus::Success,
    })
}

/// Activity that performs data archival
pub async fn archive_data_activity(
    ctx: &ActivityContext,
    archive_date: String,
) -> Result<TaskExecutionResult, ActivityError> {
    info!("Archiving data for date: {}", archive_date);
    
    let started_at = chrono::Utc::now().timestamp();
    
    // Record heartbeat
    ctx.record_heartbeat(None);
    
    // Simulate archival work
    tokio::time::sleep(Duration::from_millis(800)).await;
    
    ctx.record_heartbeat(None);
    
    let completed_at = chrono::Utc::now().timestamp();
    
    info!("Data archival for {} completed", archive_date);
    
    Ok(TaskExecutionResult {
        execution_id: Uuid::new_v4().to_string(),
        task_name: format!("archive_{}", archive_date),
        started_at,
        completed_at,
        status: TaskStatus::Success,
    })
}

/// Activity that validates input
pub async fn validate_input_activity(
    _ctx: &ActivityContext,
    input_data: String,
) -> Result<bool, ActivityError> {
    info!("Validating input data");
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Simple validation
    let valid = !input_data.is_empty() && input_data.len() < 10000;
    
    info!("Validation result: {}", valid);
    
    Ok(valid)
}
