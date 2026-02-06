//! Metrics collector - thread-safe collection with latency tracking

use super::types::TestMetrics;
use hdrhistogram::Histogram;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

#[derive(Clone)]
pub struct MetricsCollector {
    metrics: Arc<RwLock<TestMetrics>>,
    workflow_latencies: Arc<RwLock<Histogram<u64>>>,
    activity_latencies: Arc<RwLock<Histogram<u64>>>,
    system: Arc<RwLock<System>>,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        // Create histogram with 3 significant digits of precision
        let workflow_hist = Histogram::new(3).expect("Failed to create workflow histogram");
        let activity_hist = Histogram::new(3).expect("Failed to create activity histogram");

        // Initialize system monitor
        let system = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        Self {
            metrics: Arc::new(RwLock::new(TestMetrics::default())),
            workflow_latencies: Arc::new(RwLock::new(workflow_hist)),
            activity_latencies: Arc::new(RwLock::new(activity_hist)),
            system: Arc::new(RwLock::new(system)),
            start_time: Instant::now(),
        }
    }

    pub fn workflow_started(&self) {
        let mut metrics = self.metrics.write();
        metrics.workflow.started += 1;
        metrics.workflow.in_flight += 1;
    }

    pub fn workflow_completed(&self, duration_ms: u64) {
        let mut metrics = self.metrics.write();
        metrics.workflow.completed += 1;
        metrics.workflow.in_flight = metrics.workflow.in_flight.saturating_sub(1);
        drop(metrics);

        // Record latency
        if let Some(mut hist) = self.workflow_latencies.try_write() {
            let _ = hist.record(duration_ms);
        }
    }

    pub fn workflow_failed(&self, duration_ms: u64) {
        let mut metrics = self.metrics.write();
        metrics.workflow.failed += 1;
        metrics.workflow.in_flight = metrics.workflow.in_flight.saturating_sub(1);
        drop(metrics);

        // Still record latency for failed workflows
        if let Some(mut hist) = self.workflow_latencies.try_write() {
            let _ = hist.record(duration_ms);
        }
    }

    #[expect(dead_code)]
    pub fn activity_started(&self) {
        let mut metrics = self.metrics.write();
        metrics.activity.started += 1;
        metrics.activity.in_flight += 1;
    }

    #[expect(dead_code)]
    pub fn activity_completed(&self, duration_ms: u64) {
        let mut metrics = self.metrics.write();
        metrics.activity.completed += 1;
        metrics.activity.in_flight = metrics.activity.in_flight.saturating_sub(1);
        drop(metrics);

        // Record latency
        if let Some(mut hist) = self.activity_latencies.try_write() {
            let _ = hist.record(duration_ms);
        }
    }

    #[expect(dead_code)]
    pub fn activity_failed(&self, duration_ms: u64) {
        let mut metrics = self.metrics.write();
        metrics.activity.failed += 1;
        metrics.activity.in_flight = metrics.activity.in_flight.saturating_sub(1);
        drop(metrics);

        // Still record latency for failed activities
        if let Some(mut hist) = self.activity_latencies.try_write() {
            let _ = hist.record(duration_ms);
        }
    }

    /// Update system metrics (CPU, memory)
    pub fn update_system_metrics(&self) {
        let mut system = self.system.write();
        system.refresh_cpu_all();
        system.refresh_memory();

        let mut metrics = self.metrics.write();

        // Get global CPU usage
        metrics.system.cpu_usage = system.global_cpu_usage();

        // Get memory usage
        metrics.system.memory_used_mb = system.used_memory() / 1024 / 1024;
        metrics.system.memory_total_mb = system.total_memory() / 1024 / 1024;
    }

    pub fn get_snapshot(&self) -> TestMetrics {
        self.metrics.read().clone()
    }

    pub fn get_workflow_latency_percentiles(&self) -> LatencyStats {
        let hist = self.workflow_latencies.read();
        LatencyStats {
            min: hist.min(),
            p50: hist.value_at_quantile(0.50),
            p95: hist.value_at_quantile(0.95),
            p99: hist.value_at_quantile(0.99),
            max: hist.max(),
            mean: hist.mean(),
            count: hist.len(),
        }
    }

    pub fn get_activity_latency_percentiles(&self) -> LatencyStats {
        let hist = self.activity_latencies.read();
        LatencyStats {
            min: hist.min(),
            p50: hist.value_at_quantile(0.50),
            p95: hist.value_at_quantile(0.95),
            p99: hist.value_at_quantile(0.99),
            max: hist.max(),
            mean: hist.mean(),
            count: hist.len(),
        }
    }

    pub fn elapsed_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct LatencyStats {
    pub min: u64,
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
    pub max: u64,
    pub mean: f64,
    pub count: u64,
}
