//! Client options.

/// Client configuration options
#[derive(Debug, Clone)]
pub struct ClientOptions {
    pub identity: String,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            identity: format!(
                "cadence-client-{}-{}-{}-{}",
                std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
                std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()),
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
        }
    }
}
