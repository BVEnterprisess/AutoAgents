use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub auth: AuthConfig,
    pub rate_limit: RateLimitConfig,
    pub cache: CacheConfig,
    pub routing: RoutingConfig,
    pub tls: Option<TlsConfig>,
    pub observability: ObservabilityConfig,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            auth: AuthConfig::default(),
            rate_limit: RateLimitConfig::default(),
            cache: CacheConfig::default(),
            routing: RoutingConfig::default(),
            tls: None,
            observability: ObservabilityConfig::default(),
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwt_secret: Option<String>,
    pub oauth_providers: Vec<OAuthProvider>,
    pub mcp_auth_tokens: HashMap<String, String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            jwt_secret: None,
            oauth_providers: vec![],
            mcp_auth_tokens: HashMap::new(),
        }
    }
}

/// OAuth provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProvider {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
    pub burst_limit: u32,
    pub redis_url: Option<String>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: 1000,
            burst_limit: 100,
            redis_url: Some("redis://127.0.0.1:6379".to_string()),
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub max_size_mb: usize,
    pub redis_url: Option<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_seconds: 300, // 5 minutes
            max_size_mb: 512,
            redis_url: Some("redis://127.0.0.1:6379".to_string()),
        }
    }
}

/// Routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub routes: Vec<Route>,
    pub default_upstream: Option<String>,
    pub load_balancing: LoadBalancingStrategy,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            routes: vec![],
            default_upstream: Some("http://localhost:8081".to_string()),
            load_balancing: LoadBalancingStrategy::RoundRobin,
        }
    }
}

/// Route definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub path: String,
    pub upstream: String,
    pub methods: Vec<String>,
    pub headers: HashMap<String, String>,
    pub timeout_ms: Option<u64>,
}

/// Load balancing strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    Random,
    Weighted { weights: HashMap<String, u32> },
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
    pub ca_cert_path: Option<String>,
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub tracing_enabled: bool,
    pub metrics_enabled: bool,
    pub jaeger_endpoint: Option<String>,
    pub prometheus_port: Option<u16>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            tracing_enabled: true,
            metrics_enabled: true,
            jaeger_endpoint: Some("http://localhost:16686".to_string()),
            prometheus_port: Some(9090),
        }
    }
}

/// Configuration builder
pub struct ConfigBuilder {
    config: GatewayConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: GatewayConfig::default(),
        }
    }

    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.config.auth = auth;
        self
    }

    pub fn with_rate_limit(mut self, rate_limit: RateLimitConfig) -> Self {
        self.config.rate_limit = rate_limit;
        self
    }

    pub fn with_cache(mut self, cache: CacheConfig) -> Self {
        self.config.cache = cache;
        self
    }

    pub fn with_routing(mut self, routing: RoutingConfig) -> Self {
        self.config.routing = routing;
        self
    }

    pub fn with_tls(mut self, tls: TlsConfig) -> Self {
        self.config.tls = Some(tls);
        self
    }

    pub fn with_observability(mut self, observability: ObservabilityConfig) -> Self {
        self.config.observability = observability;
        self
    }

    pub fn build(self) -> GatewayConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
