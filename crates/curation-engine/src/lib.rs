pub mod config;
pub mod handlers;
pub mod models;
pub mod services;
pub mod middleware;
pub mod database;
pub mod cache;
pub mod queue;
pub mod wasm_runtime;
pub mod mcp_client;

use std::net::SocketAddr;
use axum::{
    routing::{get, post, put, delete},
    Router, middleware as axum_middleware,
};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
};

use crate::{
    config::EngineConfig,
    handlers::*,
    middleware::{auth::AuthMiddleware, rate_limit::RateLimitMiddleware},
    services::{AgentService, WasmService, MetricsService},
};

/// Main curation engine structure
pub struct CurationEngine {
    config: EngineConfig,
    agent_service: AgentService,
    wasm_service: WasmService,
    metrics_service: MetricsService,
}

impl CurationEngine {
    /// Create a new curation engine instance
    pub async fn new(config: EngineConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let agent_service = AgentService::new(config.database.clone(), config.redis.clone()).await?;
        let wasm_service = WasmService::new(config.wasm.clone()).await?;
        let metrics_service = MetricsService::new(config.metrics.clone()).await?;

        Ok(Self {
            config,
            agent_service,
            wasm_service,
            metrics_service,
        })
    }

    /// Start the curation engine server
    pub async fn serve(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Starting Curation Engine on {}", addr);

        let app = self.create_router().await?;

        let server = axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(shutdown_signal());

        server.await?;
        Ok(())
    }

    /// Create the main router with all routes and middleware
    async fn create_router(&self) -> Result<Router, Box<dyn std::error::Error>> {
        let agent_service = self.agent_service.clone();
        let wasm_service = self.wasm_service.clone();
        let metrics_service = self.metrics_service.clone();

        let app = Router::new()
            // Health check
            .route("/health", get(health_check))

            // Agent management
            .route("/api/v1/agents", post(create_agent))
            .route("/api/v1/agents/:id", get(get_agent))
            .route("/api/v1/agents/:id", put(update_agent))
            .route("/api/v1/agents/:id", delete(delete_agent))
            .route("/api/v1/agents", get(list_agents))

            // WASM module management
            .route("/api/v1/wasm/modules", post(upload_wasm_module))
            .route("/api/v1/wasm/modules/:id", get(get_wasm_module))
            .route("/api/v1/wasm/modules/:id/execute", post(execute_wasm_module))
            .route("/api/v1/wasm/modules", get(list_wasm_modules))

            // Job queue management
            .route("/api/v1/jobs", post(submit_job))
            .route("/api/v1/jobs/:id", get(get_job_status))
            .route("/api/v1/jobs/:id/cancel", post(cancel_job))
            .route("/api/v1/jobs", get(list_jobs))

            // Metrics and monitoring
            .route("/metrics", get(get_metrics))
            .route("/api/v1/metrics/agents", get(get_agent_metrics))
            .route("/api/v1/metrics/wasm", get(get_wasm_metrics))

            // MCP integration
            .route("/api/v1/mcp/tools", get(list_mcp_tools))
            .route("/api/v1/mcp/tools/:name/execute", post(execute_mcp_tool))

            // System management
            .route("/api/v1/system/status", get(get_system_status))
            .route("/api/v1/system/config", get(get_system_config))
            .route("/api/v1/system/config", put(update_system_config))

            // Layer middleware
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CompressionLayer::new())
                    .layer(CorsLayer::permissive())
                    .layer(AuthMiddleware::new(self.config.auth.clone()))
                    .layer(RateLimitMiddleware::new(self.config.rate_limit.clone()))
            )

            // Add state to all routes
            .with_state(EngineState {
                agent_service,
                wasm_service,
                metrics_service,
            });

        Ok(app)
    }

    /// Get agent service
    pub fn agent_service(&self) -> &AgentService {
        &self.agent_service
    }

    /// Get WASM service
    pub fn wasm_service(&self) -> &WasmService {
        &self.wasm_service
    }

    /// Get metrics service
    pub fn metrics_service(&self) -> &MetricsService {
        &self.metrics_service
    }
}

/// Shared state for all handlers
#[derive(Clone)]
pub struct EngineState {
    pub agent_service: AgentService,
    pub wasm_service: WasmService,
    pub metrics_service: MetricsService,
}

/// Shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("signal received, starting graceful shutdown");
}

/// Builder pattern for engine configuration
pub struct EngineBuilder {
    config: EngineConfig,
}

impl EngineBuilder {
    pub fn new() -> Self {
        Self {
            config: EngineConfig::default(),
        }
    }

    pub fn with_database(mut self, database: config::DatabaseConfig) -> Self {
        self.config.database = database;
        self
    }

    pub fn with_redis(mut self, redis: config::RedisConfig) -> Self {
        self.config.redis = redis;
        self
    }

    pub fn with_wasm(mut self, wasm: config::WasmConfig) -> Self {
        self.config.wasm = wasm;
        self
    }

    pub fn with_auth(mut self, auth: config::AuthConfig) -> Self {
        self.config.auth = auth;
        self
    }

    pub fn with_metrics(mut self, metrics: config::MetricsConfig) -> Self {
        self.config.metrics = metrics;
        self
    }

    pub async fn build(self) -> Result<CurationEngine, Box<dyn std::error::Error>> {
        CurationEngine::new(self.config).await
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
