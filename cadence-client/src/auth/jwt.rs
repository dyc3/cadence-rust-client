//! JWT authentication provider implementation.

use async_trait::async_trait;
use jsonwebtoken::{encode, EncodingKey, Header};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::provider::{AuthProvider, AuthToken};

/// Default JWT token TTL in seconds (10 minutes)
const DEFAULT_TTL_SECONDS: i64 = 600;

/// Default token expiration buffer in seconds
const EXPIRATION_BUFFER_SECONDS: i64 = 60;

/// JWT claims for Cadence authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Issuer of the token
    pub iss: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Subject (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    /// User name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Groups (space-separated, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<String>,
    /// Admin flag
    pub admin: bool,
    /// TTL for backwards compatibility
    pub ttl: i64,
}

/// Custom claims for JWT token
#[derive(Debug, Clone)]
pub struct CustomClaims {
    /// Subject identifier
    pub subject: Option<String>,
    /// User name
    pub name: Option<String>,
    /// User groups
    pub groups: Option<Vec<String>>,
    /// Admin flag
    pub admin: bool,
}

impl Default for CustomClaims {
    fn default() -> Self {
        Self {
            subject: None,
            name: None,
            groups: None,
            admin: true,
        }
    }
}

/// JWT authentication provider with token caching
pub struct JwtAuthProvider {
    /// RSA private key in PEM format
    private_key: EncodingKey,
    /// JWT issuer
    issuer: String,
    /// Token TTL in seconds
    ttl_seconds: i64,
    /// Cached token (thread-safe)
    cached_token: Arc<RwLock<Option<AuthToken>>>,
    /// Custom claims
    custom_claims: Option<CustomClaims>,
}

impl std::fmt::Debug for JwtAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtAuthProvider")
            .field("private_key", &"[REDACTED]")
            .field("issuer", &self.issuer)
            .field("ttl_seconds", &self.ttl_seconds)
            .field("cached_token", &self.cached_token)
            .field("custom_claims", &self.custom_claims)
            .finish()
    }
}

impl JwtAuthProvider {
    /// Create a new JWT authentication provider with default settings
    ///
    /// # Arguments
    /// * `private_key` - RSA private key in PEM format
    ///
    /// # Errors
    /// Returns an error if the private key is invalid
    pub fn new(private_key: Vec<u8>) -> Result<Self, jsonwebtoken::errors::Error> {
        Ok(Self {
            private_key: EncodingKey::from_rsa_pem(&private_key)?,
            issuer: "cadence-rust-client".to_string(),
            ttl_seconds: DEFAULT_TTL_SECONDS,
            cached_token: Arc::new(RwLock::new(None)),
            custom_claims: None,
        })
    }

    /// Create a new JWT authentication provider with custom options
    ///
    /// # Arguments
    /// * `private_key` - RSA private key in PEM format
    /// * `issuer` - JWT issuer claim
    /// * `ttl_seconds` - Token time-to-live in seconds
    /// * `custom_claims` - Optional custom claims
    ///
    /// # Errors
    /// Returns an error if the private key is invalid
    ///
    /// # Panics
    /// Panics if `ttl_seconds` is not positive
    pub fn with_options(
        private_key: Vec<u8>,
        issuer: impl Into<String>,
        ttl_seconds: i64,
        custom_claims: Option<CustomClaims>,
    ) -> Result<Self, jsonwebtoken::errors::Error> {
        assert!(ttl_seconds > 0, "TTL must be positive");

        Ok(Self {
            private_key: EncodingKey::from_rsa_pem(&private_key)?,
            issuer: issuer.into(),
            ttl_seconds,
            cached_token: Arc::new(RwLock::new(None)),
            custom_claims,
        })
    }

    /// Generate a new JWT token
    fn generate_token(&self) -> Result<AuthToken, GenerateTokenError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let expires_at = now + self.ttl_seconds;

        // Build groups string from custom claims
        let groups = self
            .custom_claims
            .as_ref()
            .and_then(|c| c.groups.as_ref().map(|g| g.join(" ")));

        let claims = JwtClaims {
            iss: self.issuer.clone(),
            iat: now,
            exp: expires_at,
            sub: self.custom_claims.as_ref().and_then(|c| c.subject.clone()),
            name: self.custom_claims.as_ref().and_then(|c| c.name.clone()),
            groups,
            admin: self.custom_claims.as_ref().map(|c| c.admin).unwrap_or(true),
            ttl: self.ttl_seconds,
        };

        let token = encode(
            &Header::new(jsonwebtoken::Algorithm::RS256),
            &claims,
            &self.private_key,
        )?;

        Ok(AuthToken::new(token, expires_at))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GenerateTokenError {
    #[error("JWT generation error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),
    #[error("System time error: {0}")]
    SystemTimeError(#[from] std::time::SystemTimeError),
}

#[async_trait]
impl AuthProvider for JwtAuthProvider {
    async fn get_token(&self) -> Result<AuthToken, GenerateTokenError> {
        // Fast path: read lock, check cache
        {
            let cache = self.cached_token.read();
            if let Some(token) = cache.as_ref() {
                if !token.is_expired(EXPIRATION_BUFFER_SECONDS) {
                    return Ok(token.clone());
                }
            }
        }

        // Slow path: write lock, double-check, generate new token
        let mut cache = self.cached_token.write();

        // Double-check after acquiring write lock
        if let Some(token) = cache.as_ref() {
            if !token.is_expired(EXPIRATION_BUFFER_SECONDS) {
                return Ok(token.clone());
            }
        }

        // Generate new token
        let new_token = self.generate_token()?;
        *cache = Some(new_token.clone());
        Ok(new_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{decode, DecodingKey, Validation};

    // Generate a test RSA key pair (2048 bits)
    fn generate_test_rsa_key() -> (Vec<u8>, Vec<u8>) {
        use rand::thread_rng;
        use rsa::{
            pkcs1::EncodeRsaPrivateKey, pkcs8::EncodePublicKey, RsaPrivateKey, RsaPublicKey,
        };

        let mut rng = thread_rng();
        let private_key =
            RsaPrivateKey::new(&mut rng, 2048).expect("Failed to generate private key");
        let public_key = RsaPublicKey::from(&private_key);

        let private_pem = private_key
            .to_pkcs1_pem(rsa::pkcs8::LineEnding::default())
            .expect("Failed to encode private key")
            .as_bytes()
            .to_vec();

        let public_pem = public_key
            .to_public_key_pem(rsa::pkcs8::LineEnding::default())
            .expect("Failed to encode public key")
            .as_bytes()
            .to_vec();

        (private_pem, public_pem)
    }

    #[tokio::test]
    async fn test_jwt_token_generation() {
        let (private_key, public_key) = generate_test_rsa_key();

        let provider = JwtAuthProvider::new(private_key).unwrap();
        let token = provider.get_token().await.unwrap();

        // Verify we got a non-empty token
        assert!(!token.token.is_empty());
        assert!(token.expires_at > 0);

        // Decode and verify claims
        let decoding_key = DecodingKey::from_rsa_pem(&public_key).unwrap();
        let validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        let decoded = decode::<JwtClaims>(&token.token, &decoding_key, &validation).unwrap();

        assert_eq!(decoded.claims.iss, "cadence-rust-client");
        assert!(decoded.claims.admin);
        assert_eq!(decoded.claims.ttl, DEFAULT_TTL_SECONDS);
    }

    #[tokio::test]
    async fn test_jwt_token_caching() {
        let (private_key, _public_key) = generate_test_rsa_key();

        let provider = JwtAuthProvider::new(private_key).unwrap();

        // Get token twice
        let token1 = provider.get_token().await.unwrap();
        let token2 = provider.get_token().await.unwrap();

        // Should be the same cached token
        assert_eq!(token1.token, token2.token);
        assert_eq!(token1.expires_at, token2.expires_at);
    }

    #[tokio::test]
    async fn test_jwt_token_refresh_on_expiration() {
        let (private_key, _public_key) = generate_test_rsa_key();

        // Create provider with 1-second TTL
        let provider = JwtAuthProvider::with_options(
            private_key,
            "test-issuer",
            1, // 1 second TTL
            None,
        )
        .unwrap();

        // Get initial token
        let token1 = provider.get_token().await.unwrap();

        // Wait for token to expire (plus buffer)
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Get token again - should be different
        let token2 = provider.get_token().await.unwrap();

        assert_ne!(token1.token, token2.token);
        assert_ne!(token1.expires_at, token2.expires_at);
    }

    #[tokio::test]
    async fn test_custom_claims() {
        let (private_key, public_key) = generate_test_rsa_key();

        let custom_claims = CustomClaims {
            subject: Some("user@example.com".to_string()),
            name: Some("Alice".to_string()),
            groups: Some(vec!["engineering".to_string(), "admin".to_string()]),
            admin: true,
        };

        let provider =
            JwtAuthProvider::with_options(private_key, "my-service", 600, Some(custom_claims))
                .unwrap();

        let token = provider.get_token().await.unwrap();

        // Decode and verify custom claims
        let decoding_key = DecodingKey::from_rsa_pem(&public_key).unwrap();
        let validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        let decoded = decode::<JwtClaims>(&token.token, &decoding_key, &validation).unwrap();

        assert_eq!(decoded.claims.iss, "my-service");
        assert_eq!(decoded.claims.sub, Some("user@example.com".to_string()));
        assert_eq!(decoded.claims.name, Some("Alice".to_string()));
        assert_eq!(decoded.claims.groups, Some("engineering admin".to_string()));
        assert!(decoded.claims.admin);
    }

    #[test]
    #[should_panic(expected = "TTL must be positive")]
    fn test_invalid_ttl() {
        let (private_key, _public_key) = generate_test_rsa_key();

        let _result = JwtAuthProvider::with_options(
            private_key,
            "test",
            0, // Invalid TTL
            None,
        );
    }
}
