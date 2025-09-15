//! # Fortress - Bulletproof CNCF Gateway
//!
//! A production-ready, Linkerd2-proxy inspired high-performance gateway
//! with integrated BVEnterprisess MCP registry support.

pub mod config;
pub mod gateway;
pub mod mcp_registry;
pub mod middleware;
pub mod metrics;
pub mod routing;
pub mod security;

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
};

use crate::{
    config::FortressConfig,
    gateway::GatewayService,
    middleware::{auth::AuthMiddleware, rate_limit::RateLimitMiddleware, cache::CacheMiddleware},
    metrics::MetricsCollector,
    mcp_registry::McpRegistry,
};

/// Main Fortress gateway structure
#[derive(Clone)]
pub struct Fortress {
    config: FortressConfig,
    metrics: MetricsCollector,
    mcp_registry: McpRegistry,
}

impl Fortress {
    /// Create a new Fortress instance
    pub async fn new(config: FortressConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let metrics = MetricsCollector::new();
        let mcp_registry = McpRegistry::new(config.mcp.clone()).await?;

        Ok(Self {
            config,
            metrics,
            mcp_registry,
        })
    }

    /// Start the Fortress gateway server
    pub async fn serve(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("🚀 Starting Fortress Gateway on {}", addr);
        tracing::info!("📊 Metrics available at http://{}:{}/metrics", addr.ip(), addr.port() + 1);

        let listener = TcpListener::bind(addr).await?;
        let gateway_service = GatewayService::new(
            self.config.clone(),
            self.metrics.clone(),
            self.mcp_registry.clone(),
        );

        // Build middleware stack inspired by Linkerd2-proxy
        let service = ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(CorsLayer::permissive())
            .layer(AuthMiddleware::new(self.config.auth.clone()))
            .layer(RateLimitMiddleware::new(self.config.rate_limit.clone()))
            .layer(CacheMiddleware::new(self.config.cache.clone()))
            .service(gateway_service);

        // Start metrics server
        let metrics_addr = SocketAddr::new(addr.ip(), addr.port() + 1);
        tokio::spawn(async move {
            if let Err(e) = start_metrics_server(metrics_addr, self.metrics).await {
                tracing::error!("Metrics server error: {}", e);
            }
        });

        // Main server loop
        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let service = service.clone();

            tokio::spawn(async move {
                let service = hyper::service::service_fn(move |req| {
                    let mut service = service.clone();
                    async move {
                        // Add client address to request extensions
                        req.extensions_mut().insert(remote_addr);
                        service.call(req).await
                    }
                });

                if let Err(err) = hyper::server::conn::Http::new()
                    .serve_connection(stream, service)
                    .await
                {
                    tracing::error!("Connection error: {}", err);
                }
            });
        }
    }

    /// Get MCP registry for external access
    pub fn mcp_registry(&self) -> &McpRegistry {
        &self.mcp_registry
    }

    /// Get metrics collector
    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics
    }

    /// Get configuration
    pub fn config(&self) -> &FortressConfig {
        &self.config
    }
}

/// Start metrics server
async fn start_metrics_server(
    addr: SocketAddr,
    metrics: MetricsCollector,
) -> Result<(), Box<dyn std::error::Error>> {
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{Body, Request, Response, Server};

    let make_svc = make_service_fn(move |_conn| {
        let metrics = metrics.clone();
        async move {
            Ok::<_, std::convert::Infallible>(service_fn(move |_req: Request<Body>| {
                let metrics = metrics.clone();
                async move {
                    let metrics_data = metrics.gather_metrics()
                        .unwrap_or_else(|_| "# Error collecting metrics\n".to_string());

                    Ok::<Response<Body>, std::convert::Infallible>(
                        Response::builder()
                            .header("content-type", "text/plain; charset=utf-8")
                            .body(Body::from(metrics_data))
                            .unwrap()
                    )
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    tracing::info!("📈 Metrics server listening on {}", addr);
    server.await?;
    Ok(())
}

/// Builder pattern for Fortress configuration
pub struct FortressBuilder {
    config: FortressConfig,
}

impl FortressBuilder {
    pub fn new() -> Self {
        Self {
            config: FortressConfig::default(),
        }
    }

    pub fn with_auth(mut self, auth: config::AuthConfig) -> Self {
        self.config.auth = auth;
        self
    }

    pub fn with_rate_limit(mut self, rate_limit: config::RateLimitConfig) -> Self {
        self.config.rate_limit = rate_limit;
        self
    }

    pub fn with_cache(mut self, cache: config::CacheConfig) -> Self {
        self.config.cache = cache;
        self
    }

    pub fn with_routing(mut self, routing: config::RoutingConfig) -> Self {
        self.config.routing = routing;
        self
    }

    pub fn with_mcp(mut self, mcp: config::McpConfig) -> Self {
        self.config.mcp = mcp;
        self
    }

    pub fn with_security(mut self, security: config::SecurityConfig) -> Self {
        self.config.security = security;
        self
    }

    pub async fn build(self) -> Result<Fortress, Box<dyn std::error::Error>> {
        Fortress::new(self.config).await
    }
}

impl Default for FortressBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check endpoint
pub async fn health_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fortress_builder() {
        let fortress = FortressBuilder::new()
            .build()
            .await;

        assert!(fortress.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        assert_eq!(health_check().await, "OK");
    }
}
