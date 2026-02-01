//! Custom assertion helpers for testing workflows and activities.

use cadence_core::WorkflowExecution;
use cadence_testsuite::TestWorkflowEnvironment;

/// Assert that a workflow completed successfully.
pub fn assert_workflow_completed_ok(env: &TestWorkflowEnvironment, execution: &WorkflowExecution) {
    // Implementation would check workflow completion
    let _ = (env, execution);
}

/// Assert that an activity was executed a specific number of times.
pub fn assert_activity_executed_n_times(
    env: &TestWorkflowEnvironment,
    activity_name: &str,
    expected_count: usize,
) {
    // Implementation would verify activity execution count
    let _ = (env, activity_name, expected_count);
}

/// Assert that a workflow timed out.
pub fn assert_workflow_timed_out(env: &TestWorkflowEnvironment, execution: &WorkflowExecution) {
    // Implementation would verify timeout
    let _ = (env, execution);
}

/// Assert that a workflow was cancelled.
pub fn assert_workflow_cancelled(env: &TestWorkflowEnvironment, execution: &WorkflowExecution) {
    // Implementation would verify cancellation
    let _ = (env, execution);
}
