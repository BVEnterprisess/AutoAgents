use std::{
    collections::HashMap,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use hyper::{
    header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
    http::{HeaderMap, StatusCode},
    Body, Request, Response,
};
use redis::AsyncCommands;
use tower::{Layer, Service};
use tracing::{info, warn};

use crate::config::CacheConfig;

/// Caching middleware using Redis for distributed caching
#[derive(Clone)]
pub struct CacheMiddleware {
    config: Arc<CacheConfig>,
    redis_client: Option<redis::Client>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    body: Vec<u8>,
    headers: HashMap<String, String>,
    content_type: String,
    etag: String,
    timestamp: Instant,
}

impl CacheMiddleware {
    /// Create a new caching middleware
    pub fn new(config: CacheConfig) -> Self {
        let redis_client = config.redis_url.as_ref().and_then(|url| {
            redis::Client::open(url.clone()).ok()
        });

        Self {
            config: Arc::new(config),
            redis_client,
        }
    }

    /// Generate cache key from request
    fn generate_cache_key(req: &Request<Body>) -> String {
        let mut key = format!("cache:{}:{}", req.method(), req.uri().path());

        // Include query parameters in cache key
        if let Some(query) = req.uri().query() {
            key.push_str(&format!("?{}", query));
        }

        // Include relevant headers in cache key
        let relevant_headers = ["Accept", "Accept-Language", "Authorization"];
        for header_name in &relevant_headers {
            if let Some(header_value) = req.headers().get(*header_name) {
                if let Ok(value_str) = header_value.to_str() {
                    key.push_str(&format!(":{}:{}", header_name, value_str));
                }
            }
        }

        // Hash the key to keep it reasonable length
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("cache:{}", hasher.finish())
    }

    /// Check if request is cacheable
    fn is_cacheable(req: &Request<Body>) -> bool {
        // Only cache GET requests
        if req.method() != hyper::Method::GET {
            return false;
        }

        // Don't cache requests with authorization headers (unless configured)
        if req.headers().contains_key("authorization") {
            return false;
        }

        // Don't cache requests with cache-control: no-cache
        if let Some(cache_control) = req.headers().get(CACHE_CONTROL) {
            if let Ok(value) = cache_control.to_str() {
                if value.contains("no-cache") {
                    return false;
                }
            }
        }

        true
    }

    /// Check if response is cacheable
    fn is_response_cacheable(response: &Response<Body>) -> bool {
        // Only cache successful responses
        if !response.status().is_success() {
            return false;
        }

        // Check cache-control header
        if let Some(cache_control) = response.headers().get(CACHE_CONTROL) {
            if let Ok(value) = cache_control.to_str() {
                if value.contains("no-cache") || value.contains("private") {
                    return false;
                }
            }
        }

        true
    }

    /// Store response in cache
    async fn store_in_cache(&self, key: &str, response: &Response<Body>, body: &[u8]) {
        if !self.config.enabled {
            return;
        }

        let entry = CacheEntry {
            body: body.to_vec(),
            headers: response.headers()
                .iter()
                .filter_map(|(name, value)| {
                    value.to_str().ok().map(|v| (name.to_string(), v.to_string()))
                })
                .collect(),
            content_type: response.headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string(),
            etag: format!("\"{}\"", uuid::Uuid::new_v4().simple()),
            timestamp: Instant::now(),
        };

        if let Some(client) = &self.redis_client {
            if let Ok(mut conn) = client.get_async_connection().await {
                let serialized = serde_json::to_string(&entry).unwrap_or_default();
                let _: Result<(), _> = conn.set_ex(key, serialized, self.config.ttl_seconds).await;
            }
        }
    }

    /// Retrieve response from cache
    async fn get_from_cache(&self, key: &str) -> Option<CacheEntry> {
        if !self.config.enabled {
            return None;
        }

        if let Some(client) = &self.redis_client {
            if let Ok(mut conn) = client.get_async_connection().await {
                if let Ok(Some(serialized)) = conn.get::<_, Option<String>>(key).await {
                    if let Ok(entry) = serde_json::from_str::<CacheEntry>(&serialized) {
                        // Check if entry is still fresh
                        if entry.timestamp.elapsed() < Duration::from_secs(self.config.ttl_seconds) {
                            return Some(entry);
                        }
                    }
                }
            }
        }

        None
    }

    /// Create response from cache entry
    fn create_response_from_cache(entry: CacheEntry) -> Response<Body> {
        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, &entry.content_type)
            .header(ETAG, &entry.etag);

        // Add cached headers
        for (name, value) in entry.headers {
            builder = builder.header(name, value);
        }

        builder.body(Body::from(entry.body)).unwrap()
    }
}

impl<S> Layer<S> for CacheMiddleware {
    type Service = CacheMiddlewareService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheMiddlewareService {
            inner,
            config: self.config.clone(),
            redis_client: self.redis_client.clone(),
        }
    }
}

/// Service wrapper for caching middleware
#[derive(Clone)]
pub struct CacheMiddlewareService<S> {
    inner: S,
    config: Arc<CacheConfig>,
    redis_client: Option<redis::Client>,
}

impl<S> Service<Request<Body>> for CacheMiddlewareService<S>
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
        let mut inner = self.inner.clone();

        Box::pin(async move {
            if !config.enabled || !Self::is_cacheable(&req) {
                return inner.call(req).await;
            }

            let cache_key = Self::generate_cache_key(&req);

            // Check for conditional request (If-None-Match)
            if let Some(if_none_match) = req.headers().get(IF_NONE_MATCH) {
                if let Ok(etag) = if_none_match.to_str() {
                    if let Some(cached_entry) = Self::get_from_cache_static(&cache_key, &redis_client, &config).await {
                        if format!("\"{}\"", cached_entry.etag) == etag {
                            info!("Cache hit with conditional request for key: {}", cache_key);
                            return Ok(Response::builder()
                                .status(StatusCode::NOT_MODIFIED)
                                .header(ETAG, etag)
                                .body(Body::empty())
                                .unwrap());
                        }
                    }
                }
            }

            // Try to get from cache first
            if let Some(cached_entry) = Self::get_from_cache_static(&cache_key, &redis_client, &config).await {
                info!("Cache hit for key: {}", cache_key);
                return Ok(Self::create_response_from_cache_static(cached_entry));
            }

            // Cache miss - forward request
            info!("Cache miss for key: {}", cache_key);
            let response = inner.call(req).await?;

            // Cache the response if it's cacheable
            if Self::is_response_cacheable(&response) {
                if let Ok(body_bytes) = hyper::body::to_bytes(response.body()).await {
                    Self::store_in_cache_static(&cache_key, &response, &body_bytes, &redis_client, &config).await;

                    // Recreate response since we consumed the body
                    let mut builder = Response::builder().status(response.status());
                    for (name, value) in response.headers() {
                        builder = builder.header(name, value);
                    }
                    return Ok(builder.body(Body::from(body_bytes)).unwrap());
                }
            }

            Ok(response)
        })
    }
}

impl<S> CacheMiddlewareService<S> {
    fn is_cacheable(req: &Request<Body>) -> bool {
        // Only cache GET requests
        if req.method() != hyper::Method::GET {
            return false;
        }

        // Don't cache requests with authorization headers (unless configured)
        if req.headers().contains_key("authorization") {
            return false;
        }

        // Don't cache requests with cache-control: no-cache
        if let Some(cache_control) = req.headers().get(CACHE_CONTROL) {
            if let Ok(value) = cache_control.to_str() {
                if value.contains("no-cache") {
                    return false;
                }
            }
        }

        true
    }

    fn is_response_cacheable(response: &Response<Body>) -> bool {
        // Only cache successful responses
        if !response.status().is_success() {
            return false;
        }

        // Check cache-control header
        if let Some(cache_control) = response.headers().get(CACHE_CONTROL) {
            if let Ok(value) = cache_control.to_str() {
                if value.contains("no-cache") || value.contains("private") {
                    return false;
                }
            }
        }

        true
    }

    async fn get_from_cache_static(
        key: &str,
        redis_client: &Option<redis::Client>,
        config: &CacheConfig,
    ) -> Option<CacheEntry> {
        if !config.enabled {
            return None;
        }

        if let Some(client) = redis_client {
            if let Ok(mut conn) = client.get_async_connection().await {
                if let Ok(Some(serialized)) = conn.get::<_, Option<String>>(key).await {
                    if let Ok(entry) = serde_json::from_str::<CacheEntry>(&serialized) {
                        // Check if entry is still fresh
                        if entry.timestamp.elapsed() < Duration::from_secs(config.ttl_seconds) {
                            return Some(entry);
                        }
                    }
                }
            }
        }

        None
    }

    async fn store_in_cache_static(
        key: &str,
        response: &Response<Body>,
        body: &[u8],
        redis_client: &Option<redis::Client>,
        config: &CacheConfig,
    ) {
        if !config.enabled {
            return;
        }

        let entry = CacheEntry {
            body: body.to_vec(),
            headers: response.headers()
                .iter()
                .filter_map(|(name, value)| {
                    value.to_str().ok().map(|v| (name.to_string(), v.to_string()))
                })
                .collect(),
            content_type: response.headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string(),
            etag: format!("\"{}\"", uuid::Uuid::new_v4().simple()),
            timestamp: Instant::now(),
        };

        if let Some(client) = redis_client {
            if let Ok(mut conn) = client.get_async_connection().await {
                let serialized = serde_json::to_string(&entry).unwrap_or_default();
                let _: Result<(), _> = conn.set_ex(key, serialized, config.ttl_seconds).await;
            }
        }
    }

    fn create_response_from_cache_static(entry: CacheEntry) -> Response<Body> {
        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, &entry.content_type)
            .header(ETAG, &entry.etag);

        // Add cached headers
        for (name, value) in entry.headers {
            builder = builder.header(name, value);
        }

        builder.body(Body::from(entry.body)).unwrap()
    }
}
