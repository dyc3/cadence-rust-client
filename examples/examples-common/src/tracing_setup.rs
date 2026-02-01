//! Tracing setup helpers for consistent logging across examples.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing for examples with a standard format.
pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize tracing for tests (doesn't panic if already initialized).
pub fn init_test_tracing() {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();
}
