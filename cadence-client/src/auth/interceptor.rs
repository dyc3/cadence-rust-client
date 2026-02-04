//! Tonic interceptor for authentication.

use std::sync::Arc;
use tonic::{Request, Status};

use super::provider::AuthProvider;

/// Interceptor that adds authentication headers to gRPC requests
#[derive(Clone)]
pub struct AuthInterceptor {
    auth_provider: Option<Arc<dyn AuthProvider>>,
}

impl AuthInterceptor {
    /// Create a new auth interceptor
    ///
    /// If `auth_provider` is `None`, the interceptor will be a no-op
    pub fn new(auth_provider: Option<Arc<dyn AuthProvider>>) -> Self {
        Self { auth_provider }
    }
}

impl tonic::service::Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        // If no provider, skip (allows optional auth)
        let Some(provider) = &self.auth_provider else {
            return Ok(request);
        };

        // Clone provider Arc for the blocking task
        let provider = provider.clone();

        // Use spawn_blocking to avoid blocking the async runtime
        let token = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move { provider.get_token().await })
        })
        .map_err(|e| Status::unauthenticated(format!("Authentication failed: {}", e)))?;

        // Inject header
        request.metadata_mut().insert(
            "cadence-authorization",
            token
                .token
                .parse()
                .map_err(|_| Status::internal("Invalid token format"))?,
        );

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use tonic::service::Interceptor;

    use super::*;
    use async_trait::async_trait;
    use cadence_core::CadenceResult;

    use super::super::provider::{AuthProvider, AuthToken};

    struct MockAuthProvider {
        token: String,
    }

    #[async_trait]
    impl AuthProvider for MockAuthProvider {
        async fn get_token(&self) -> CadenceResult<AuthToken> {
            Ok(AuthToken {
                token: self.token.clone(),
                expires_at: i64::MAX,
            })
        }
    }

    #[tokio::test]
    async fn test_interceptor_adds_header() {
        let provider: Arc<dyn AuthProvider> = Arc::new(MockAuthProvider {
            token: "test-jwt-token".to_string(),
        });
        let mut interceptor = AuthInterceptor::new(Some(provider));

        // Use spawn_blocking since the interceptor uses block_in_place
        let result = tokio::task::spawn_blocking(move || {
            let request = Request::new(());
            interceptor.call(request)
        })
        .await
        .unwrap();

        assert!(result.is_ok());
        let request = result.unwrap();
        let header = request.metadata().get("cadence-authorization");
        assert!(header.is_some());
        assert_eq!(header.unwrap(), "test-jwt-token");
    }

    #[tokio::test]
    async fn test_interceptor_no_op_without_provider() {
        let mut interceptor = AuthInterceptor::new(None);

        // Use spawn_blocking since the interceptor uses block_in_place
        let result = tokio::task::spawn_blocking(move || {
            let request = Request::new(());
            interceptor.call(request)
        })
        .await
        .unwrap();

        assert!(result.is_ok());
        let request = result.unwrap();
        let header = request.metadata().get("cadence-authorization");
        assert!(header.is_none());
    }
}
