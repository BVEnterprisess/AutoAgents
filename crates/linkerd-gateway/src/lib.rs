pub mod config;
pub mod gateway;
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
    config::GatewayConfig,
    gateway::GatewayService,
    middleware::{AuthMiddleware, RateLimitMiddleware, CacheMiddleware},
    metrics::MetricsCollector,
};

/// Main gateway structure implementing Linkerd2-proxy patterns
pub struct LinkerdGateway {
    config: GatewayConfig,
    metrics: MetricsCollector,
}

impl LinkerdGateway {
    /// Create a new gateway instance
    pub fn new(config: GatewayConfig) -> Self {
        let metrics = MetricsCollector::new();
        Self { config, metrics }
    }

    /// Start the gateway server
    pub async fn serve(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Starting Linkerd Gateway on {}", addr);

        let listener = TcpListener::bind(addr).await?;
        let gateway_service = GatewayService::new(self.config.clone(), self.metrics.clone());

        // Build middleware stack inspired by Linkerd2-proxy
        let service = ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(CorsLayer::permissive())
            .layer(AuthMiddleware::new(self.config.auth.clone()))
            .layer(RateLimitMiddleware::new(self.config.rate_limit.clone()))
            .layer(CacheMiddleware::new(self.config.cache.clone()))
            .service(gateway_service);

        loop {
            let (stream, _) = listener.accept().await?;
            let service = service.clone();

            tokio::spawn(async move {
                if let Err(err) = hyper::server::conn::Http::new()
                    .serve_connection(stream, service)
                    .await
                {
                    tracing::error!("Error serving connection: {}", err);
                }
            });
        }
    }

    /// Get metrics collector
    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics
    }

    /// Get gateway configuration
    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }
}

/// Builder pattern for gateway configuration
pub struct GatewayBuilder {
    config: GatewayConfig,
}

impl GatewayBuilder {
    pub fn new() -> Self {
        Self {
            config: GatewayConfig::default(),
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

    pub fn build(self) -> LinkerdGateway {
        LinkerdGateway::new(self.config)
    }
}

impl Default for GatewayBuilder {
    fn default() -> Self {
        Self::new()
    }
}
