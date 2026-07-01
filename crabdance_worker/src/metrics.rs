//! Native worker metrics, emitted through the [`metrics`] facade.
//!
//! These are the Rust analogue of the Go client's tally metrics. The worker emits them
//! at the decision-task, activity-task and poller boundaries; applications install
//! their own recorder/exporter (e.g. `metrics-exporter-prometheus`) to collect them —
//! the SDK does not bundle one.
//!
//! # Example: exporting to Prometheus
//!
//! ```ignore
//! // Cargo.toml: metrics-exporter-prometheus = "0.16"
//! use metrics_exporter_prometheus::PrometheusBuilder;
//!
//! // Installs a global recorder and serves /metrics on 0.0.0.0:9000.
//! PrometheusBuilder::new()
//!     .install()
//!     .expect("failed to install Prometheus recorder");
//! // ... start your worker; metrics below will be scraped by Prometheus.
//! ```

use std::time::Duration;

// Counter metric names.
pub const DECISION_TASK_STARTED: &str = "cadence_decision_task_started_total";
pub const DECISION_TASK_COMPLETED: &str = "cadence_decision_task_completed_total";
pub const DECISION_TASK_FAILED: &str = "cadence_decision_task_failed_total";
pub const ACTIVITY_TASK_STARTED: &str = "cadence_activity_task_started_total";
pub const ACTIVITY_TASK_COMPLETED: &str = "cadence_activity_task_completed_total";
pub const ACTIVITY_TASK_FAILED: &str = "cadence_activity_task_failed_total";
/// Activities that handed off to async/external completion — counted so
/// `started ≈ completed + failed + async_pending` reconciles (the worker does not
/// finish these; the result arrives out of band).
pub const ACTIVITY_TASK_ASYNC_PENDING: &str = "cadence_activity_task_async_pending_total";
pub const WORKFLOW_PANIC: &str = "cadence_workflow_panic_total";
pub const POLLER_START: &str = "cadence_poller_start_total";

// Histogram metric names (seconds).
pub const DECISION_TASK_LATENCY: &str = "cadence_decision_task_execution_latency_seconds";
pub const ACTIVITY_TASK_LATENCY: &str = "cadence_activity_task_execution_latency_seconds";

// Gauge metric names.
pub const CONCURRENT_TASK_QUOTA: &str = "cadence_concurrent_task_quota";

/// Increment a named counter by one, with a single `(key, value)` tag.
pub fn incr(name: &'static str, tag_key: &'static str, tag_value: &str) {
    metrics::counter!(name, tag_key => tag_value.to_string()).increment(1);
}

/// Record a duration (in seconds) on a named histogram, with a single `(key, value)` tag.
pub fn record_latency(
    name: &'static str,
    tag_key: &'static str,
    tag_value: &str,
    elapsed: Duration,
) {
    metrics::histogram!(name, tag_key => tag_value.to_string()).record(elapsed.as_secs_f64());
}

/// Set a named gauge, with a single `(key, value)` tag.
pub fn set_gauge(name: &'static str, tag_key: &'static str, tag_value: &str, value: f64) {
    metrics::gauge!(name, tag_key => tag_value.to_string()).set(value);
}

/// Conventional tag keys.
pub const TAG_TASK_LIST: &str = "task_list";
pub const TAG_ACTIVITY_TYPE: &str = "activity_type";
