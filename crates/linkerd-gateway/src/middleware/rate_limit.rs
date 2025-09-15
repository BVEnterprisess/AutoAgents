use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use hyper::{
    header::CONTENT_TYPE,
    http::StatusCode,
    Body, Request, Response,
};
use redis::AsyncCommands;
use tokio::sync::Mutex;
use tower::{Layer, Service};
use tracing::{info, warn};

use crate::config::RateLimitConfig;

/// Rate limiting middleware using Redis for distributed rate limiting
#[derive(Clone)]
pub struct RateLimitMiddleware {
    config: Arc<RateLimitConfig>,
    redis_client: Option<redis::Client>,
    local_limits: Arc<Mutex<HashMap<String, RateLimitState>>>,
}

#[derive(Debug, Clone)]
struct RateLimitState {
    tokens: u32,
    last_refill: Instant,
}

impl RateLimitMiddleware {
    /// Create a new rate limiting middleware
    pub fn new(config: RateLimitConfig) -> Self {
        let redis_client = config.redis_url.as_ref().and_then(|url| {
            redis::Client::open(url.clone()).ok()
        });

        Self {
            config: Arc::new(config),
            redis_client,
            local_limits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Extract client identifier (IP address or user ID)
    fn get_client_key(&self, req: &Request<Body>) -> String {
        // Try to get user ID from headers first (for authenticated requests)
        if let Some(user_id) = req.headers().get("X-User-ID") {
            if let Ok(user_id_str) = user_id.to_str() {
                return format!("user:{}", user_id_str);
            }
        }

        // Fall back to IP address
        req.extensions()
            .get::<std::net::SocketAddr>()
            .map(|addr| format!("ip:{}", addr.ip()))
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Check rate limit using Redis (distributed)
    async fn check_redis_limit(&self, key: &str) -> Result<bool, redis::RedisError> {
        if let Some(client) = &self.redis_client {
            let mut conn = client.get_async_connection().await?;

            // Use Redis sorted set to track requests with timestamps
            let now = chrono::Utc::now().timestamp();
            let window_start = now - 60; // 1 minute window

            // Add current request timestamp
            let _: () = conn.zadd(key, now, now).await?;

            // Remove old entries outside the window
            let _: () = conn.zremrangebyscore(key, 0, window_start).await?;

            // Count requests in the current window
            let count: i64 = conn.zcount(key, window_start, now).await?;

            Ok(count <= self.config.requests_per_minute as i64)
        } else {
            // Fallback to local rate limiting
            self.check_local_limit(key).await
        }
    }

    /// Check rate limit using local storage (single instance)
    async fn check_local_limit(&self, key: &str) -> bool {
        let mut limits = self.local_limits.lock().await;
        let now = Instant::now();

        let state = limits.entry(key.to_string()).or_insert_with(|| RateLimitState {
            tokens: self.config.requests_per_minute,
            last_refill: now,
        });

        // Refill tokens based on time elapsed
        let elapsed = now.duration_since(state.last_refill);
        let refill_amount = (elapsed.as_secs() * self.config.requests_per_minute as u64) / 60;

        if refill_amount > 0 {
            state.tokens = (state.tokens + refill_amount as u32).min(self.config.requests_per_minute);
            state.last_refill = now;
        }

        // Check if we have tokens available
        if state.tokens > 0 {
            state.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Get retry-after duration in seconds
    fn get_retry_after(&self) -> u64 {
        60 // 1 minute
    }
}

impl<S> Layer<S> for RateLimitMiddleware {
    type Service = RateLimitMiddlewareService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddlewareService {
            inner,
            config: self.config.clone(),
            redis_client: self.redis_client.clone(),
            local_limits: self.local_limits.clone(),
        }
    }
}

/// Service wrapper for rate limiting middleware
#[derive(Clone)]
pub struct RateLimitMiddlewareService<S> {
    inner: S,
    config: Arc<RateLimitConfig>,
    redis_client: Option<redis::Client>,
    local_limits: Arc<Mutex<HashMap<String, RateLimitState>>>,
}

impl<S> Service<Request<Body>> for RateLimitMiddlewareService<S>
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

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let config = self.config.clone();
        let redis_client = self.redis_client.clone();
        let local_limits = self.local_limits.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            if !config.enabled {
                return inner.call(req).await;
            }

            let client_key = Self::get_client_key(&req);

            // Check rate limit
            let allowed = if redis_client.is_some() {
                Self::check_redis_limit_static(&client_key, &redis_client, &config).await
                    .unwrap_or(false)
            } else {
                Self::check_local_limit_static(&client_key, &local_limits, &config).await
            };

            if allowed {
                info!("Rate limit check passed for client: {}", client_key);
                inner.call(req).await
            } else {
                warn!("Rate limit exceeded for client: {}", client_key);
                let retry_after = Self::get_retry_after_static(&config);

                let response = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header(CONTENT_TYPE, "application/json")
                    .header("Retry-After", retry_after.to_string())
                    .header("X-Rate-Limit-Limit", config.requests_per_minute.to_string())
                    .body(Body::from(format!(
                        r#"{{"error": "Rate limit exceeded", "retry_after": {}}}"#,
                        retry_after
                    )))
                    .unwrap();

                Ok(response)
            }
        })
    }
}

impl<S> RateLimitMiddlewareService<S> {
    fn get_client_key(req: &Request<Body>) -> String {
        // Try to get user ID from headers first (for authenticated requests)
        if let Some(user_id) = req.headers().get("X-User-ID") {
            if let Ok(user_id_str) = user_id.to_str() {
                return format!("user:{}", user_id_str);
            }
        }

        // Fall back to IP address
        req.extensions()
            .get::<std::net::SocketAddr>()
            .map(|addr| format!("ip:{}", addr.ip()))
            .unwrap_or_else(|| "unknown".to_string())
    }

    async fn check_redis_limit_static(
        key: &str,
        redis_client: &Option<redis::Client>,
        config: &RateLimitConfig,
    ) -> Result<bool, redis::RedisError> {
        if let Some(client) = redis_client {
            let mut conn = client.get_async_connection().await?;

            // Use Redis sorted set to track requests with timestamps
            let now = chrono::Utc::now().timestamp();
            let window_start = now - 60; // 1 minute window

            // Add current request timestamp
            let _: () = conn.zadd(key, now, now).await?;

            // Remove old entries outside the window
            let _: () = conn.zremrangebyscore(key, 0, window_start).await?;

            // Count requests in the current window
            let count: i64 = conn.zcount(key, window_start, now).await?;

            Ok(count <= config.requests_per_minute as i64)
        } else {
            Ok(false)
        }
    }

    async fn check_local_limit_static(
        key: &str,
        local_limits: &Arc<Mutex<HashMap<String, RateLimitState>>>,
        config: &RateLimitConfig,
    ) -> bool {
        let mut limits = local_limits.lock().await;
        let now = Instant::now();

        let state = limits.entry(key.to_string()).or_insert_with(|| RateLimitState {
            tokens: config.requests_per_minute,
            last_refill: now,
        });

        // Refill tokens based on time elapsed
        let elapsed = now.duration_since(state.last_refill);
        let refill_amount = (elapsed.as_secs() * config.requests_per_minute as u64) / 60;

        if refill_amount > 0 {
            state.tokens = (state.tokens + refill_amount as u32).min(config.requests_per_minute);
            state.last_refill = now;
        }

        // Check if we have tokens available
        if state.tokens > 0 {
            state.tokens -= 1;
            true
        } else {
            false
        }
    }

    fn get_retry_after_static(config: &RateLimitConfig) -> u64 {
        60 // 1 minute
    }
}
