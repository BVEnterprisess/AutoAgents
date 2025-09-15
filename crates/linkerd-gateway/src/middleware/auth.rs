use std::{
    collections::HashMap,
    sync::Arc,
    task::{Context, Poll},
};

use hyper::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    http::{HeaderMap, StatusCode},
    Body, Request, Response,
};
use tower::{Layer, Service};
use tracing::{info, warn};

use crate::config::{AuthConfig, OAuthProvider};

/// Authentication middleware for the gateway
#[derive(Clone)]
pub struct AuthMiddleware {
    config: Arc<AuthConfig>,
}

impl AuthMiddleware {
    /// Create a new authentication middleware
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Validate JWT token
    fn validate_jwt(&self, token: &str) -> Result<Claims, AuthError> {
        if !self.config.enabled {
            return Ok(Claims::default());
        }

        // TODO: Implement proper JWT validation
        // For now, just check if token exists
        if token.is_empty() {
            return Err(AuthError::InvalidToken);
        }

        Ok(Claims {
            sub: "user".to_string(),
            exp: 0,
            roles: vec!["user".to_string()],
        })
    }

    /// Validate MCP auth token
    fn validate_mcp_token(&self, token: &str, service: &str) -> Result<(), AuthError> {
        if let Some(expected_token) = self.config.mcp_auth_tokens.get(service) {
            if token == expected_token {
                return Ok(());
            }
        }
        Err(AuthError::InvalidToken)
    }

    /// Extract bearer token from Authorization header
    fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
        headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
    }

    /// Check if path requires authentication
    fn requires_auth(&self, path: &str) -> bool {
        // Public paths that don't require authentication
        let public_paths = ["/health", "/metrics", "/api/v1/auth"];
        public_paths.iter().any(|public_path| path.starts_with(public_path))
    }
}

impl<S> Layer<S> for AuthMiddleware {
    type Service = AuthMiddlewareService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddlewareService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Service wrapper for authentication middleware
#[derive(Clone)]
pub struct AuthMiddlewareService<S> {
    inner: S,
    config: Arc<AuthConfig>,
}

impl<S> Service<Request<Body>> for AuthMiddlewareService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let config = self.config.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let path = req.uri().path();

            // Skip authentication for public paths
            if !config.enabled || config.as_ref().requires_auth(path) {
                return inner.call(req).await;
            }

            // Check for MCP auth token in headers
            if let Some(mcp_token) = req.headers().get("X-MCP-Token") {
                if let Ok(token_str) = mcp_token.to_str() {
                    // Extract service name from path or headers
                    let service = req.headers()
                        .get("X-MCP-Service")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("default");

                    if config.validate_mcp_token(token_str, service).is_ok() {
                        info!("MCP authentication successful for service: {}", service);
                        return inner.call(req).await;
                    }
                }
            }

            // Check for JWT token
            if let Some(token) = Self::extract_bearer_token(req.headers()) {
                match config.validate_jwt(token) {
                    Ok(claims) => {
                        // Add user claims to request headers
                        req.headers_mut().insert(
                            "X-User-ID",
                            claims.sub.parse().unwrap(),
                        );
                        req.headers_mut().insert(
                            "X-User-Roles",
                            claims.roles.join(",").parse().unwrap(),
                        );

                        info!("JWT authentication successful for user: {}", claims.sub);
                        return inner.call(req).await;
                    }
                    Err(err) => {
                        warn!("JWT authentication failed: {:?}", err);
                    }
                }
            }

            // Authentication failed
            warn!("Authentication failed for path: {}", path);
            let response = Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"error": "Authentication required"}"#))
                .unwrap();

            Ok(response)
        })
    }
}

/// JWT claims structure
#[derive(Debug, Clone, Default)]
struct Claims {
    sub: String,
    exp: u64,
    roles: Vec<String>,
}

/// Authentication errors
#[derive(Debug, thiserror::Error)]
enum AuthError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid signature")]
    InvalidSignature,
}

impl AuthConfig {
    fn requires_auth(&self, path: &str) -> bool {
        // Public paths that don't require authentication
        let public_paths = ["/health", "/metrics", "/api/v1/auth"];
        public_paths.iter().any(|public_path| path.starts_with(public_path))
    }

    fn validate_jwt(&self, token: &str) -> Result<Claims, AuthError> {
        if !self.enabled {
            return Ok(Claims::default());
        }

        // TODO: Implement proper JWT validation
        // For now, just check if token exists
        if token.is_empty() {
            return Err(AuthError::InvalidToken);
        }

        Ok(Claims {
            sub: "user".to_string(),
            exp: 0,
            roles: vec!["user".to_string()],
        })
    }

    fn validate_mcp_token(&self, token: &str, service: &str) -> Result<(), AuthError> {
        if let Some(expected_token) = self.mcp_auth_tokens.get(service) {
            if token == expected_token {
                return Ok(());
            }
        }
        Err(AuthError::InvalidToken)
    }
}
