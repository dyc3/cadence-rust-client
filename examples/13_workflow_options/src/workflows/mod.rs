//! Workflow implementations for workflow options example.

use crate::activities::{
    ReportRequest, ReportResult, ScheduledTaskInput, TaskExecutionResult,
};
use cadence_core::{ActivityOptions, WorkflowIdReusePolicy};
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use std::time::Duration;
use tracing::info;

/// Workflow demonstrating execution timeout configuration
pub async fn timeout_configured_workflow(
    ctx: &mut WorkflowContext,
    request: ReportRequest,
) -> Result<ReportResult, WorkflowError> {
    info!(
        "Starting timeout-configured workflow for {} report",
        request.report_type
    );
    
    // Activity with specific timeout configuration
    let activity_options = ActivityOptions {
        task_list: ctx.workflow_info().task_list.clone(),
        start_to_close_timeout: match request.report_type.as_str() {
            "summary" => Duration::from_secs(5),
            "detailed" => Duration::from_secs(30),
            "audit" => Duration::from_secs(120),
            _ => Duration::from_secs(30),
        },
        schedule_to_close_timeout: Duration::from_secs(300),
        ..Default::default()
    };
    
    let result = ctx
        .execute_activity(
            "generate_report",
            Some(serde_json::to_vec(&request).unwrap()),
            activity_options,
        )
        .await?;
    
    let report: ReportResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse report: {}", e)))?;
    
    info!("Timeout-configured workflow completed: report {}", report.report_id);
    
    Ok(report)
}

/// Workflow that demonstrates cron schedule configuration
pub async fn scheduled_report_workflow(
    ctx: &mut WorkflowContext,
    request: ReportRequest,
) -> Result<TaskExecutionResult, WorkflowError> {
    info!(
        "Starting scheduled report workflow for {} report",
        request.report_type
    );
    
    // Validate input first
    let validation = ctx
        .execute_activity(
            "validate_input",
            Some(serde_json::to_vec(&format!("{:?}", request)).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let is_valid: bool = serde_json::from_slice(&validation)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation: {}", e)))?;
    
    if !is_valid {
        return Err(WorkflowError::Generic("Invalid report request".to_string()));
    }
    
    // Generate report with appropriate timeout
    let timeout = match request.report_type.as_str() {
        "summary" => Duration::from_secs(10),
        "detailed" => Duration::from_secs(60),
        "audit" => Duration::from_secs(300),
        _ => Duration::from_secs(60),
    };
    
    let activity_options = ActivityOptions {
        task_list: ctx.workflow_info().task_list.clone(),
        start_to_close_timeout: timeout,
        ..Default::default()
    };
    
    let result = ctx
        .execute_activity(
            "generate_report",
            Some(serde_json::to_vec(&request).unwrap()),
            activity_options,
        )
        .await?;
    
    let report: ReportResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse report: {}", e)))?;
    
    let started_at = chrono::Utc::now().timestamp();
    
    // Log completion
    info!(
        "Scheduled report workflow completed: {} with {} records",
        report.report_id, report.record_count
    );
    
    let completed_at = chrono::Utc::now().timestamp();
    
    Ok(TaskExecutionResult {
        execution_id: report.report_id,
        task_name: format!("report_{}", request.report_type),
        started_at,
        completed_at,
        status: crate::activities::TaskStatus::Success,
    })
}

/// Workflow demonstrating memo usage
pub async fn workflow_with_memo(
    ctx: &mut WorkflowContext,
    request: ReportRequest,
) -> Result<ReportResult, WorkflowError> {
    let workflow_info = ctx.workflow_info();
    
    info!("Starting workflow with memo for {}", request.report_type);
    
    // Access memo data if available
    if let Some(memo) = &workflow_info.memo {
        info!("Workflow memo contains {} fields", memo.len());
        for (key, value) in memo.iter() {
            info!("Memo field: {} = {:?}", key, value);
        }
    }
    
    // Generate report
    let activity_options = ActivityOptions {
        task_list: workflow_info.task_list.clone(),
        start_to_close_timeout: Duration::from_secs(60),
        ..Default::default()
    };
    
    let result = ctx
        .execute_activity(
            "generate_report",
            Some(serde_json::to_vec(&request).unwrap()),
            activity_options,
        )
        .await?;
    
    let report: ReportResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse report: {}", e)))?;
    
    info!("Workflow with memo completed");
    
    Ok(report)
}

/// Workflow demonstrating various workflow ID reuse policies
pub async fn id_reuse_policy_workflow(
    ctx: &mut WorkflowContext,
    request: ReportRequest,
) -> Result<ReportResult, WorkflowError> {
    let workflow_info = ctx.workflow_info();
    
    info!(
        "Starting workflow with ID: {}",
        workflow_info.workflow_execution.workflow_id,
    );
    
    // Generate report
    let activity_options = ActivityOptions {
        task_list: workflow_info.task_list.clone(),
        start_to_close_timeout: Duration::from_secs(60),
        ..Default::default()
    };
    
    let result = ctx
        .execute_activity(
            "generate_report",
            Some(serde_json::to_vec(&request).unwrap()),
            activity_options,
        )
        .await?;
    
    let report: ReportResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse report: {}", e)))?;
    
    info!("ID reuse policy workflow completed");
    
    Ok(report)
}

/// Workflow demonstrating scheduled/cron execution
pub async fn cron_scheduled_workflow(
    ctx: &mut WorkflowContext,
    input: ScheduledTaskInput,
) -> Result<TaskExecutionResult, WorkflowError> {
    info!(
        "Starting cron scheduled workflow for task '{}' at {}",
        input.task_name, input.scheduled_time
    );
    
    let started_at = chrono::Utc::now().timestamp();
    
    // Execute scheduled cleanup
    let result = ctx
        .execute_activity(
            "scheduled_cleanup",
            Some(serde_json::to_vec(&input).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let task_result: TaskExecutionResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    info!(
        "Cron scheduled workflow completed: task '{}' finished at {}",
        task_result.task_name, task_result.completed_at
    );
    
    let completed_at = chrono::Utc::now().timestamp();
    
    Ok(TaskExecutionResult {
        execution_id: task_result.execution_id,
        task_name: input.task_name,
        started_at,
        completed_at,
        status: crate::activities::TaskStatus::Success,
    })
}

/// Workflow demonstrating archival configuration
pub async fn archival_configured_workflow(
    ctx: &mut WorkflowContext,
    archive_date: String,
) -> Result<TaskExecutionResult, WorkflowError> {
    info!("Starting archival configured workflow for date: {}", archive_date);
    
    let started_at = chrono::Utc::now().timestamp();
    
    // Archive data with configured options
    let result = ctx
        .execute_activity(
            "archive_data",
            Some(serde_json::to_vec(&archive_date).unwrap()),
            ActivityOptions {
                task_list: ctx.workflow_info().task_list.clone(),
                start_to_close_timeout: Duration::from_secs(120),
                ..Default::default()
            },
        )
        .await?;
    
    let archival_result: TaskExecutionResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    info!(
        "Archival workflow completed: {} archived at {}",
        archival_result.task_name, archival_result.completed_at
    );
    
    let completed_at = chrono::Utc::now().timestamp();
    
    Ok(TaskExecutionResult {
        execution_id: archival_result.execution_id,
        task_name: format!("archival_{}", archive_date),
        started_at,
        completed_at,
        status: crate::activities::TaskStatus::Success,
    })
}
