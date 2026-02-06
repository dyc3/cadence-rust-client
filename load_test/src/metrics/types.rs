//! Metric types

#[derive(Debug, Clone, Default)]
pub struct WorkflowMetrics {
    pub started: usize,
    pub completed: usize,
    pub failed: usize,
    pub in_flight: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ActivityMetrics {
    pub started: usize,
    pub completed: usize,
    pub failed: usize,
    pub in_flight: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
}

#[derive(Debug, Clone, Default)]
pub struct TestMetrics {
    pub workflow: WorkflowMetrics,
    pub activity: ActivityMetrics,
    pub system: SystemMetrics,
}
