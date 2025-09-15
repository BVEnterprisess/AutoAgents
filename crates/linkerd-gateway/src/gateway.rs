use std::{
    collections::HashMap,
    convert::Infallible,
    sync::Arc,
    time::{Duration, Instant},
};

use hyper::{
    body::Bytes,
    client::HttpConnector,
    http::{HeaderMap, Method, StatusCode, Uri, Version},
    Body, Client, Request, Response,
};
use tower::{Service, ServiceExt};
use tracing::{info, warn, error, instrument};

use crate::{
    config::{GatewayConfig, Route},
    metrics::MetricsCollector,
    routing::Router,
};

/// Main gateway service implementing Linkerd2-proxy patterns
pub struct GatewayService {
    config: GatewayConfig,
    client: Client<HttpConnector>,
    router: Router,
    metrics: MetricsCollector,
}

impl GatewayService {
    /// Create a new gateway service
    pub fn new(config: GatewayConfig, metrics: MetricsCollector) -> Self {
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .http2_only(false)
            .build_http();

        let router = Router::new(config.routing.clone());

        Self {
            config,
            client,
            router,
            metrics,
        }
    }

    /// Route request to appropriate upstream service
    #[instrument(skip(self, req), fields(method = %req.method(), uri = %req.uri()))]
    async fn route_request(
        &self,
        mut req: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        let start_time = Instant::now();

        // Find matching route
        let route = match self.router.find_route(req.uri().path(), req.method()) {
            Some(route) => route,
            None => {
                warn!("No route found for {} {}", req.method(), req.uri().path());
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

        // Update request URI and headers
        *req.uri_mut() = upstream_uri;
        self.add_upstream_headers(&mut req, &route);

        // Forward request to upstream
        match self.client.request(req).await {
            Ok(mut response) => {
                // Add gateway headers
                self.add_gateway_headers(&mut response);

                // Record metrics
                self.metrics.record_request(response.status(), start_time.elapsed());

                info!(
                    "Request completed: {} {} -> {}",
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

    /// Add headers for upstream request
    fn add_upstream_headers(&self, req: &mut Request<Body>, route: &Route) {
        let headers = req.headers_mut();

        // Add route-specific headers
        for (key, value) in &route.headers {
            headers.insert(key.parse().unwrap(), value.parse().unwrap());
        }

        // Add gateway identification headers
        headers.insert("X-Gateway", "Linkerd-AutoAgents".parse().unwrap());
        headers.insert("X-Forwarded-Host", req.uri().host().unwrap_or("unknown").parse().unwrap());
        headers.insert("X-Forwarded-Proto", "http".parse().unwrap());
        headers.insert("X-Request-ID", uuid::Uuid::new_v4().to_string().parse().unwrap());
    }

    /// Add gateway headers to response
    fn add_gateway_headers(&self, response: &mut Response<Body>) {
        let headers = response.headers_mut();
        headers.insert("X-Gateway-Version", "1.0.0".parse().unwrap());
        headers.insert("X-Powered-By", "AutoAgents-Gateway".parse().unwrap());
    }

    /// Create error response
    fn create_error_response(&self, status: StatusCode, message: &str) -> Response<Body> {
        let body = serde_json::json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
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

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
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

impl Clone for GatewayService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: self.client.clone(),
            router: self.router.clone(),
            metrics: self.metrics.clone(),
        }
    }
}
