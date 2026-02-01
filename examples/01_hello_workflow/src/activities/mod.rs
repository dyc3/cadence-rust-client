//! Activity implementations for the hello workflow example.

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Greeting type options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GreetingType {
    Formal,
    Casual,
    Excited,
}

/// Input for the hello workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloInput {
    pub name: String,
    pub greeting_type: GreetingType,
}

/// Output from the hello workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloOutput {
    pub message: String,
    pub timestamp: i64,
}

/// A simple activity that formats a greeting
pub async fn format_greeting_activity(
    ctx: &ActivityContext,
    input: HelloInput,
) -> Result<HelloOutput, ActivityError> {
    info!("Executing format_greeting_activity for: {}", input.name);

    // Get activity info
    let activity_info = ctx.get_info();
    info!(
        "Activity ID: {}, Attempt: {}",
        activity_info.activity_id, activity_info.attempt
    );

    // Simulate some work
    tokio::time::sleep(Duration::from_millis(100)).await;

    let message = match input.greeting_type {
        GreetingType::Formal => format!("Hello, {}. Welcome!", input.name),
        GreetingType::Casual => format!("Hey {}! What's up?", input.name),
        GreetingType::Excited => format!("WOW! It's {}! Amazing to see you!", input.name),
    };

    // Record heartbeat to show progress
    ctx.record_heartbeat(None);

    Ok(HelloOutput {
        message,
        timestamp: chrono::Utc::now().timestamp(),
    })
}
