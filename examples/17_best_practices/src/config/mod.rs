//! Configuration management best practices.
//!
//! This module demonstrates:
//! - Environment-based configuration
//! - Validation at load time
//! - Sensible defaults
//! - Documentation of all options

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Application configuration with validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Worker configuration
    pub worker: WorkerConfig,

    /// Retry configuration
    pub retry: RetryConfig,

    /// Timeout configuration
    pub timeouts: TimeoutConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Feature flags
    pub features: FeatureFlags,
}

/// Worker-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Number of concurrent workflow tasks to process
    #[serde(default = "default_worker_concurrency")]
    pub concurrency: usize,

    /// Task list name for this worker
    pub task_list: String,

    /// Identity string for this worker instance
    #[serde(default = "default_worker_identity")]
    pub identity: String,

    /// Whether to enable sticky execution (cache workflows locally)
    #[serde(default = "default_sticky_cache")]
    pub sticky_cache_enabled: bool,

    /// Maximum number of workflows to cache
    #[serde(default = "default_sticky_cache_size")]
    pub sticky_cache_size: usize,
}

fn default_worker_concurrency() -> usize {
    num_cpus::get()
}

fn default_worker_identity() -> String {
    format!("worker-{}", uuid::Uuid::new_v4())
}

fn default_sticky_cache() -> bool {
    true
}

fn default_sticky_cache_size() -> usize {
    10000
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Initial retry interval
    #[serde(with = "serde_duration", default = "default_initial_interval")]
    pub initial_interval: Duration,

    /// Backoff coefficient for exponential backoff
    #[serde(default = "default_backoff_coefficient")]
    pub backoff_coefficient: f64,

    /// Maximum interval between retries
    #[serde(with = "serde_duration", default = "default_maximum_interval")]
    pub maximum_interval: Duration,

    /// Maximum number of retry attempts
    #[serde(default = "default_maximum_attempts")]
    pub maximum_attempts: u32,

    /// Error types that should not be retried
    #[serde(default)]
    pub non_retryable_errors: Vec<String>,
}

fn default_initial_interval() -> Duration {
    Duration::from_secs(1)
}

fn default_backoff_coefficient() -> f64 {
    2.0
}

fn default_maximum_interval() -> Duration {
    Duration::from_secs(60)
}

fn default_maximum_attempts() -> u32 {
    3
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Default workflow execution timeout
    #[serde(with = "serde_duration", default = "default_workflow_timeout")]
    pub workflow_execution: Duration,

    /// Default activity start-to-close timeout
    #[serde(with = "serde_duration", default = "default_activity_timeout")]
    pub activity_start_to_close: Duration,

    /// Default activity schedule-to-close timeout
    #[serde(with = "serde_duration", default = "default_schedule_to_close")]
    pub activity_schedule_to_close: Duration,

    /// Default heartbeat timeout
    #[serde(with = "serde_duration", default = "default_heartbeat_timeout")]
    pub heartbeat: Duration,
}

fn default_workflow_timeout() -> Duration {
    Duration::from_secs(3600) // 1 hour
}

fn default_activity_timeout() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_schedule_to_close() -> Duration {
    Duration::from_secs(600) // 10 minutes
}

fn default_heartbeat_timeout() -> Duration {
    Duration::from_secs(60) // 1 minute
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Whether to include timestamps
    #[serde(default = "default_true")]
    pub include_timestamps: bool,

    /// Whether to include thread IDs
    #[serde(default = "default_true")]
    pub include_thread_ids: bool,

    /// Output format (json, pretty, compact)
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

fn default_log_format() -> String {
    "pretty".to_string()
}

/// Feature flags for gradual rollout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable detailed activity metrics
    #[serde(default)]
    pub detailed_metrics: bool,

    /// Enable distributed tracing
    #[serde(default = "default_true")]
    pub distributed_tracing: bool,

    /// Enable workflow caching
    #[serde(default = "default_true")]
    pub workflow_caching: bool,

    /// Enable idempotency checks
    #[serde(default = "default_true")]
    pub idempotency_checks: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            worker: WorkerConfig {
                concurrency: default_worker_concurrency(),
                task_list: "default-task-list".to_string(),
                identity: default_worker_identity(),
                sticky_cache_enabled: default_sticky_cache(),
                sticky_cache_size: default_sticky_cache_size(),
            },
            retry: RetryConfig {
                initial_interval: default_initial_interval(),
                backoff_coefficient: default_backoff_coefficient(),
                maximum_interval: default_maximum_interval(),
                maximum_attempts: default_maximum_attempts(),
                non_retryable_errors: vec![],
            },
            timeouts: TimeoutConfig {
                workflow_execution: default_workflow_timeout(),
                activity_start_to_close: default_activity_timeout(),
                activity_schedule_to_close: default_schedule_to_close(),
                heartbeat: default_heartbeat_timeout(),
            },
            logging: LoggingConfig {
                level: default_log_level(),
                include_timestamps: true,
                include_thread_ids: true,
                format: default_log_format(),
            },
            features: FeatureFlags {
                detailed_metrics: false,
                distributed_tracing: true,
                workflow_caching: true,
                idempotency_checks: true,
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        // In a real implementation, use envy or similar
        // For now, return default
        Ok(Self::default())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.worker.concurrency == 0 {
            return Err(ConfigError::Invalid(
                "Worker concurrency must be greater than 0".to_string(),
            ));
        }

        if self.worker.task_list.is_empty() {
            return Err(ConfigError::MissingField("worker.task_list".to_string()));
        }

        if self.retry.backoff_coefficient < 1.0 {
            return Err(ConfigError::Invalid(
                "Backoff coefficient must be >= 1.0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Serialization helper for Duration
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
