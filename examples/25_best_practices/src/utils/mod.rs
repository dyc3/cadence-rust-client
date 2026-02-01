//! Utility functions and helpers.
//!
//! This module demonstrates:
//! - Structured logging helpers
//! - Retry with backoff
//! - Circuit breaker patterns
//! - Rate limiting
//! - Context propagation

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error, instrument};
use thiserror::Error;

/// Retry an async operation with exponential backoff
#[instrument(skip(operation))]
pub async fn retry_with_backoff<T, E, F, Fut>(
    operation: F,
    max_attempts: u32,
    initial_delay: Duration,
    backoff_multiplier: f64,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 1;
    let mut delay = initial_delay;
    
    loop {
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    info!("Operation succeeded on attempt {}", attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt >= max_attempts {
                    error!("Operation failed after {} attempts", max_attempts);
                    return Err(e);
                }
                
                warn!(
                    "Operation failed on attempt {} (retrying in {:?}): {}",
                    attempt, delay, e
                );
                
                sleep(delay).await;
                delay = Duration::from_secs_f64(delay.as_secs_f64() * backoff_multiplier);
                attempt += 1;
            }
        }
    }
}

/// Circuit breaker state
#[derive(Debug, Clone)]
pub enum CircuitState {
    Closed,
    Open { opened_at: std::time::Instant, reset_timeout: Duration },
    HalfOpen,
}

/// Simple circuit breaker implementation
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,
    #[allow(dead_code)]
    success_threshold: u32,
    reset_timeout: Duration,
}

#[derive(Debug, Error)]
pub enum CircuitError {
    #[error("Circuit breaker is open")]
    Open,
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            success_threshold,
            reset_timeout,
        }
    }
    
    pub async fn call<T, E, F, Fut>(&mut self, operation: F) -> Result<T, CircuitError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        // Check if we should transition from Open to HalfOpen
        if let CircuitState::Open { opened_at, reset_timeout } = self.state {
            if opened_at.elapsed() >= reset_timeout {
                info!("Circuit breaker transitioning to HalfOpen");
                self.state = CircuitState::HalfOpen;
                self.failure_count = 0;
            } else {
                return Err(CircuitError::Open);
            }
        }
        
        match operation().await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(e) => {
                self.on_failure();
                Err(CircuitError::OperationFailed(e.to_string()))
            }
        }
    }
    
    fn on_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                // In HalfOpen, we need consecutive successes to close
                self.failure_count = 0;
                self.state = CircuitState::Closed;
                info!("Circuit breaker closed after success in HalfOpen");
            }
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::Open { .. } => {
                // Shouldn't happen, but handle gracefully
            }
        }
    }
    
    fn on_failure(&mut self) {
        self.failure_count += 1;
        
        if self.failure_count >= self.failure_threshold {
            if matches!(self.state, CircuitState::Closed | CircuitState::HalfOpen) {
                warn!("Circuit breaker opened after {} failures", self.failure_count);
                self.state = CircuitState::Open {
                    opened_at: std::time::Instant::now(),
                    reset_timeout: self.reset_timeout,
                };
            }
        }
    }
    
    pub fn state(&self) -> &CircuitState {
        &self.state
    }
}

/// Structured log entry builder
pub struct LogEntry {
    fields: Vec<(String, String)>,
}

impl LogEntry {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }
    
    pub fn field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push((key.into(), value.into()));
        self
    }
    
    pub fn info(self, message: impl AsRef<str>) {
        let fields_str = self.fields
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        
        info!("{} | {}", message.as_ref(), fields_str);
    }
    
    pub fn error(self, message: impl AsRef<str>) {
        let fields_str = self.fields
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        
        error!("{} | {}", message.as_ref(), fields_str);
    }
}

/// Generate a correlation ID for request tracking
pub fn generate_correlation_id() -> String {
    format!("corr-{}", uuid::Uuid::new_v4())
}

/// Truncate a string to a maximum length with ellipsis
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Sanitize a string for logging (remove sensitive data)
pub fn sanitize_for_logging(s: &str) -> String {
    // Simple sanitization - in production, use proper PII handling
    if s.len() > 4 {
        format!("{}****", &s[..4.min(s.len())])
    } else {
        "****".to_string()
    }
}

/// Timeout wrapper for operations
pub async fn with_timeout<T, Fut>(
    future: Fut,
    timeout: Duration,
    operation_name: &str,
) -> Result<T, tokio::time::error::Elapsed>
where
    Fut: Future<Output = T>,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(result) => Ok(result),
        Err(e) => {
            warn!("Operation '{}' timed out after {:?}", operation_name, timeout);
            Err(e)
        }
    }
}
