use std::collections::HashSet;
use std::sync::Arc;

use hyper::{
    header::{CONTENT_TYPE, X_FORWARDED_FOR},
    http::StatusCode,
    Body, Request, Response,
};
use regex::Regex;
use tracing::{info, warn, error};

/// Security enforcer for the gateway
pub struct SecurityEnforcer {
    blocked_ips: HashSet<String>,
    blocked_paths: Vec<Regex>,
    allowed_origins: HashSet<String>,
    max_request_size: usize,
    suspicious_patterns: Vec<Regex>,
}

impl SecurityEnforcer {
    /// Create a new security enforcer
    pub fn new() -> Self {
        let mut blocked_paths = Vec::new();
        let mut suspicious_patterns = Vec::new();

        // Common blocked paths
        for pattern in &[
            r"\.\./",           // Directory traversal
            r"\.\.\\",          // Windows directory traversal
            r"<script",         // XSS attempts
            r"javascript:",     // JavaScript injection
            r"data:",           // Data URL injection
            r"vbscript:",       // VBScript injection
            r"onload=",         // Event handler injection
            r"onerror=",        // Event handler injection
        ] {
            if let Ok(regex) = Regex::new(pattern) {
                blocked_paths.push(regex);
            }
        }

        // Suspicious patterns
        for pattern in &[
            r"union.*select",   // SQL injection
            r"script.*alert",   // XSS
            r"eval\(",          // Code injection
            r"document\.cookie", // Cookie theft
            r"xmlhttprequest",  // AJAX injection
        ] {
            if let Ok(regex) = Regex::new(&format!("(?i){}", pattern)) {
                suspicious_patterns.push(regex);
            }
        }

        Self {
            blocked_ips: HashSet::new(),
            blocked_paths,
            allowed_origins: HashSet::new(),
            max_request_size: 10 * 1024 * 1024, // 10MB
            suspicious_patterns,
        }
    }

    /// Check if request passes security checks
    pub async fn check_request(&self, req: &Request<Body>) -> Result<(), SecurityError> {
        // Check IP address
        if let Some(client_ip) = self.get_client_ip(req) {
            if self.blocked_ips.contains(&client_ip) {
                warn!("Blocked request from IP: {}", client_ip);
                return Err(SecurityError::BlockedIP(client_ip));
            }
        }

        // Check path for malicious patterns
        let path = req.uri().path();
        for regex in &self.blocked_paths {
            if regex.is_match(path) {
                warn!("Blocked request with malicious path: {}", path);
                return Err(SecurityError::MaliciousPath(path.to_string()));
            }
        }

        // Check query parameters for suspicious patterns
        if let Some(query) = req.uri().query() {
            for regex in &self.suspicious_patterns {
                if regex.is_match(query) {
                    warn!("Blocked request with suspicious query: {}", query);
                    return Err(SecurityError::SuspiciousContent(query.to_string()));
                }
            }
        }

        // Check headers for suspicious content
        for (name, value) in req.headers() {
            if let Ok(value_str) = value.to_str() {
                for regex in &self.suspicious_patterns {
                    if regex.is_match(value_str) {
                        warn!("Blocked request with suspicious header {}: {}", name, value_str);
                        return Err(SecurityError::SuspiciousContent(value_str.to_string()));
                    }
                }
            }
        }

        // Check Content-Length header
        if let Some(content_length) = req.headers().get("content-length") {
            if let Ok(length_str) = content_length.to_str() {
                if let Ok(length) = length_str.parse::<usize>() {
                    if length > self.max_request_size {
                        warn!("Blocked request with excessive content length: {}", length);
                        return Err(SecurityError::RequestTooLarge(length));
                    }
                }
            }
        }

        // Check Origin header for CORS
        if let Some(origin) = req.headers().get("origin") {
            if let Ok(origin_str) = origin.to_str() {
                if !self.allowed_origins.is_empty() && !self.allowed_origins.contains(origin_str) {
                    warn!("Blocked request from unauthorized origin: {}", origin_str);
                    return Err(SecurityError::UnauthorizedOrigin(origin_str.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Get client IP address from request
    fn get_client_ip(&self, req: &Request<Body>) -> Option<String> {
        // Check X-Forwarded-For header first
        if let Some(forwarded_for) = req.headers().get(X_FORWARDED_FOR) {
            if let Ok(forwarded_str) = forwarded_for.to_str() {
                // Take the first IP in the chain (original client)
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    return Some(first_ip.trim().to_string());
                }
            }
        }

        // Fall back to socket address
        req.extensions()
            .get::<std::net::SocketAddr>()
            .map(|addr| addr.ip().to_string())
    }

    /// Add IP to blocklist
    pub fn block_ip(&mut self, ip: String) {
        self.blocked_ips.insert(ip);
    }

    /// Remove IP from blocklist
    pub fn unblock_ip(&mut self, ip: &str) {
        self.blocked_ips.remove(ip);
    }

    /// Add allowed origin for CORS
    pub fn add_allowed_origin(&mut self, origin: String) {
        self.allowed_origins.insert(origin);
    }

    /// Remove allowed origin
    pub fn remove_allowed_origin(&mut self, origin: &str) {
        self.allowed_origins.remove(origin);
    }

    /// Set maximum request size
    pub fn set_max_request_size(&mut self, size: usize) {
        self.max_request_size = size;
    }

    /// Get security statistics
    pub fn get_stats(&self) -> SecurityStats {
        SecurityStats {
            blocked_ips_count: self.blocked_ips.len(),
            allowed_origins_count: self.allowed_origins.len(),
            max_request_size: self.max_request_size,
        }
    }
}

impl Default for SecurityEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

/// Security statistics
#[derive(Debug, Clone)]
pub struct SecurityStats {
    pub blocked_ips_count: usize,
    pub allowed_origins_count: usize,
    pub max_request_size: usize,
}

/// Security errors
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Blocked IP address: {0}")]
    BlockedIP(String),
    #[error("Malicious path detected: {0}")]
    MaliciousPath(String),
    #[error("Suspicious content detected: {0}")]
    SuspiciousContent(String),
    #[error("Request too large: {0} bytes")]
    RequestTooLarge(usize),
    #[error("Unauthorized origin: {0}")]
    UnauthorizedOrigin(String),
}

/// Security middleware
pub struct SecurityMiddleware<S> {
    inner: S,
    enforcer: Arc<SecurityEnforcer>,
}

impl<S> SecurityMiddleware<S> {
    pub fn new(inner: S, enforcer: SecurityEnforcer) -> Self {
        Self {
            inner,
            enforcer: Arc::new(enforcer),
        }
    }
}

impl<S> tower::Service<Request<Body>> for SecurityMiddleware<S>
where
    S: tower::Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let enforcer = self.enforcer.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Perform security checks
            match enforcer.check_request(&req).await {
                Ok(()) => {
                    // Request passed security checks
                    inner.call(req).await
                }
                Err(security_error) => {
                    // Request failed security checks
                    error!("Security check failed: {:?}", security_error);

                    let (status, message) = match security_error {
                        SecurityError::BlockedIP(_) => (StatusCode::FORBIDDEN, "Access denied"),
                        SecurityError::MaliciousPath(_) => (StatusCode::BAD_REQUEST, "Invalid request path"),
                        SecurityError::SuspiciousContent(_) => (StatusCode::BAD_REQUEST, "Invalid request content"),
                        SecurityError::RequestTooLarge(_) => (StatusCode::PAYLOAD_TOO_LARGE, "Request too large"),
                        SecurityError::UnauthorizedOrigin(_) => (StatusCode::FORBIDDEN, "Unauthorized origin"),
                    };

                    let response = Response::builder()
                        .status(status)
                        .header(CONTENT_TYPE, "application/json")
                        .body(Body::from(format!(
                            r#"{{"error": "{}", "message": "{}"}}"#,
                            status.as_u16(),
                            message
                        )))
                        .unwrap();

                    Ok(response)
                }
            }
        })
    }
}

/// Rate limiting for security (additional layer beyond the main rate limiter)
pub struct SecurityRateLimiter {
    requests_per_window: usize,
    window_duration: std::time::Duration,
    request_counts: std::sync::Mutex<std::collections::HashMap<String, Vec<std::time::Instant>>>,
}

impl SecurityRateLimiter {
    pub fn new(requests_per_window: usize, window_duration: std::time::Duration) -> Self {
        Self {
            requests_per_window,
            window_duration,
            request_counts: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn check_rate_limit(&self, key: &str) -> bool {
        let mut counts = self.request_counts.lock().unwrap();
        let now = std::time::Instant::now();

        let timestamps = counts.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove old timestamps outside the window
        timestamps.retain(|&timestamp| now.duration_since(timestamp) < self.window_duration);

        // Check if under limit
        if timestamps.len() < self.requests_per_window {
            timestamps.push(now);
            true
        } else {
            false
        }
    }
}

impl Default for SecurityRateLimiter {
    fn default() -> Self {
        Self::new(100, std::time::Duration::from_secs(60)) // 100 requests per minute
    }
}
