//! Authentication provider trait and types.

use async_trait::async_trait;
use cadence_core::CadenceResult;
use std::sync::Arc;

/// Authentication token with expiration information
#[derive(Debug, Clone)]
pub struct AuthToken {
    /// Raw token string (e.g., JWT token)
    pub token: String,
    /// Unix timestamp when the token expires
    pub expires_at: i64,
}

impl AuthToken {
    /// Create a new auth token
    pub fn new(token: String, expires_at: i64) -> Self {
        Self { token, expires_at }
    }

    /// Check if the token is expired or will expire within the buffer period
    pub fn is_expired(&self, buffer_seconds: i64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.expires_at - buffer_seconds <= now
    }
}

/// Trait for authentication providers
///
/// Implementations can generate various types of authentication tokens
/// (JWT, OAuth, etc.) and manage token caching internally.
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Get an authentication token (may be cached internally)
    ///
    /// Returns a token that can be used for authentication. The provider
    /// should handle caching and refresh internally.
    async fn get_token(&self) -> CadenceResult<AuthToken>;
}

/// Type alias for boxed auth provider
pub type BoxedAuthProvider = Arc<dyn AuthProvider>;
