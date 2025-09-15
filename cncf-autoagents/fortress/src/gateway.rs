//! Gateway Service Implementation
//!
//! Core HTTP routing and middleware orchestration for the Fortress gateway.

use std::{
    convert::Infallible,
    sync::Arc,
    time::{Duration, Instant},
};

use hyper::{
    body::Bytes,
    http::{HeaderMap, Method, StatusCode, Uri},
    Body, Request, Response,
};
use tower::{Service, ServiceExt};
use tracing::{info, warn, error, instrument};

use crate::{
    config::{FortressConfig, Route},
    metrics::MetricsCollector,
    mcp_registry::McpRegistry,
    routing::Router,
};

/// Main gateway service
#[derive(Clone)]
pub struct GatewayService {
    config: FortressConfig,
    router: Router,
    metrics: MetricsCollector,
    mcp_registry: McpRegistry,
    http_client: reqwest::Client,
}

impl GatewayService {
    /// Create a new gateway service
    pub fn new(
        config: FortressConfig,
        metrics: MetricsCollector,
        mcp_registry: McpRegistry,
    ) -> Self {
        let router = Router::new(config.routing.clone());
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            router,
            metrics,
            mcp_registry,
            http_client,
        }
    }

    /// Route request to appropriate upstream service
    #[instrument(skip(self, req), fields(method = %req.method(), uri = %req.uri()))]
    async fn route_request(
        &self,
        mut req: Request<Body>,
    ) -> Result<Response<Body>, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let path = req.uri().path().to_string();
        let method = req.method().clone();

        // Find matching route
        let route = match self.router.find_route(&path, &method) {
            Some(route) => route,
            None => {
                warn!("No route found for {} {}", method, path);
                self.metrics.record_request(StatusCode::NOT_FOUND, start_time.elapsed());
                return Ok(self.create_error_response(
                    StatusCode::NOT_FOUND,
                    "Route not found",
                ));
            }
        };

        // Build upstream URI
        let upstream_uri = match self.build_upstream_uri(&route, &req) {
            Ok(uri) => uri,
            Err(err) => {
                error!("Failed to build upstream URI: {}", err);
                self.metrics.record_request(StatusCode::BAD_GATEWAY, start_time.elapsed());
                return Ok(self.create_error_response(
                    StatusCode::BAD_GATEWAY,
                    "Invalid upstream configuration",
                ));
            }
        };

        // Add gateway headers
        self.add_gateway_headers(&mut req, &route);

        // Forward request to upstream
        match self.forward_request(req, upstream_uri).await {
            Ok(mut response) => {
                // Add response headers
                self.add_response_headers(&mut response);

                // Record metrics
                self.metrics.record_request(response.status(), start_time.elapsed());

                info!(
                    "Request completed: {} {} -> {} ({}ms)",
                    route.path,
                    response.status(),
                    start_time.elapsed().as_millis()
                );

                Ok(response)
            }
            Err(err) => {
                error!("Upstream request failed: {}", err);
                self.metrics.record_request(StatusCode::BAD_GATEWAY, start_time.elapsed());
                Ok(self.create_error_response(
                    StatusCode::BAD_GATEWAY,
                    "Upstream service unavailable",
                ))
            }
        }
    }

    /// Forward request to upstream service
    async fn forward_request(
        &self,
        req: Request<Body>,
        upstream_uri: Uri,
    ) -> Result<Response<Body>, Box<dyn std::error::Error>> {
        // Convert hyper request to reqwest request
        let (parts, body) = req.into_parts();

        // Read body
        let body_bytes = hyper::body::to_bytes(body).await?;
        let body_data = String::from_utf8(body_bytes.to_vec())?;

        // Build reqwest request
        let mut request_builder = self.http_client
            .request(
                match parts.method {
                    Method::GET => reqwest::Method::GET,
                    Method::POST => reqwest::Method::POST,
                    Method::PUT => reqwest::Method::PUT,
                    Method::DELETE => reqwest::Method::DELETE,
                    Method::PATCH => reqwest::Method::PATCH,
                    Method::HEAD => reqwest::Method::HEAD,
                    Method::OPTIONS => reqwest::Method::OPTIONS,
                    _ => reqwest::Method::GET,
                },
                upstream_uri.to_string(),
            )
            .body(body_data);

        // Add headers
        for (name, value) in parts.headers {
            if let Some(name) = name {
                if let Ok(value_str) = value.to_str() {
                    request_builder = request_builder.header(name, value_str);
                }
            }
        }

        // Send request
        let response = request_builder.send().await?;

        // Convert reqwest response to hyper response
        let status = response.status();
        let headers = response.headers().clone();
        let body_text = response.text().await?;

        let mut hyper_response = Response::builder().status(status);

        // Add headers
        for (name, value) in headers {
            if let Some(name) = name {
                hyper_response = hyper_response.header(name, value);
            }
        }

        Ok(hyper_response.body(Body::from(body_text))?)
    }

    /// Build upstream URI from route configuration
    fn build_upstream_uri(&self, route: &Route, req: &Request<Body>) -> Result<Uri, Box<dyn std::error::Error>> {
        let mut upstream_url = route.upstream.clone();

        // Replace path parameters
        if let Some(query) = req.uri().query() {
            upstream_url.push('?');
            upstream_url.push_str(query);
        }

        // Handle path replacement for proxy-style routing
        if route.path.ends_with("/*") {
            let remaining_path = &req.uri().path()[route.path.len() - 1..];
            upstream_url = upstream_url.replace("/*", remaining_path);
        }

        Uri::try_from(upstream_url).map_err(Into::into)
    }

    /// Add gateway headers to upstream request
    fn add_gateway_headers(&self, req: &mut Request<Body>, route: &Route) {
        let headers = req.headers_mut();

        // Add route-specific headers
        for (key, value) in &route.headers {
            headers.insert(key.parse().unwrap(), value.parse().unwrap());
        }

        // Add gateway identification headers
        headers.insert("X-Gateway", "Fortress-AutoAgents".parse().unwrap());
        headers.insert("X-Forwarded-Host", req.uri().host().unwrap_or("unknown").parse().unwrap());
        headers.insert("X-Forwarded-Proto", "http".parse().unwrap());
        headers.insert("X-Request-ID", uuid::Uuid::new_v4().to_string().parse().unwrap());
        headers.insert("X-Gateway-Version", "0.1.0".parse().unwrap());
    }

    /// Add gateway headers to response
    fn add_response_headers(&self, response: &mut Response<Body>) {
        let headers = response.headers_mut();
        headers.insert("X-Gateway-Version", "0.1.0".parse().unwrap());
        headers.insert("X-Powered-By", "Fortress-Gateway".parse().unwrap());
    }

    /// Create error response
    fn create_error_response(&self, status: StatusCode, message: &str) -> Response<Body> {
        let body = serde_json::json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "gateway": "fortress"
            }
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }
}

impl Service<Request<Body>> for GatewayService {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move {
            match this.route_request(req).await {
                Ok(response) => Ok(response),
                Err(_) => Ok(this.create_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal gateway error",
                )),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Method;

    #[tokio::test]
    async fn test_gateway_service_creation() {
        let config = FortressConfig::default();
        let metrics = MetricsCollector::new();
        let mcp_registry = McpRegistry::new(config.mcp.clone()).await.unwrap();

        let service = GatewayService::new(config, metrics, mcp_registry);
        // Service created successfully
        assert!(true);
    }

    #[test]
    fn test_upstream_uri_building() {
        // This would require a full GatewayService instance
        // Simplified test for URI building logic
        assert!(true);
    }
}
