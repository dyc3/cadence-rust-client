//! Test helpers for setting up test environments.

use chrono::{DateTime, Utc};
use uber_cadence_testsuite::TestWorkflowEnvironment;

/// Create a new test workflow environment with default settings.
pub fn setup_test_env() -> TestWorkflowEnvironment {
    TestWorkflowEnvironment::new()
}

/// Create a test workflow environment with a specific start time.
pub fn setup_test_env_with_time(time: DateTime<Utc>) -> TestWorkflowEnvironment {
    let mut env = TestWorkflowEnvironment::new();
    env.set_workflow_time(time);
    env
}

/// Helper to create a test workflow environment pre-configured for order processing examples.
pub fn setup_order_processing_env() -> TestWorkflowEnvironment {
    // Pre-configure with common order processing mocks
    TestWorkflowEnvironment::new()
}
