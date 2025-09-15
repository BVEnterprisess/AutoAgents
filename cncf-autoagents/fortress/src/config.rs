//! Configuration structures for Fortress gateway

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main Fortress configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FortressConfig {
    pub auth: AuthConfig,
    pub rate_limit: RateLimitConfig,
    pub cache: CacheConfig,
    pub routing: RoutingConfig,
    pub mcp: McpConfig,
    pub security: SecurityConfig,
    pub observability: ObservabilityConfig,
}

impl Default for FortressConfig {
    fn default() -> Self {
        Self {
            auth: AuthConfig::default(),
            rate_limit: RateLimitConfig::default(),
            cache: CacheConfig::default(),
            routing: RoutingConfig::default(),
            mcp: McpConfig::default(),
            security: SecurityConfig::default(),
            observability: ObservabilityConfig::default(),
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwt_secret: Option<String>,
    pub mcp_auth_tokens: HashMap<String, String>,
    pub service_accounts: HashMap<String, ServiceAccount>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            jwt_secret: None,
            mcp_auth_tokens: HashMap::new(),
            service_accounts: HashMap::new(),
        }
    }
}

/// Service account for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccount {
    pub name: String,
    pub token: String,
    pub permissions: Vec<String>,
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

/// MCP registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub bv_enterprise_registry_url: String,
    pub awesome_servers_url: Option<String>,
    pub health_check_interval_seconds: u64,
    pub cache_ttl_seconds: u64,
    pub max_concurrent_requests: usize,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            bv_enterprise_registry_url: "https://raw.githubusercontent.com/BVEnterprisess/registry/main/registry.json".to_string(),
            awesome_servers_url: Some("https://raw.githubusercontent.com/modelcontextprotocol/awesome-mcp-servers/main/README.md".to_string()),
            health_check_interval_seconds: 60,
            cache_ttl_seconds: 300,
            max_concurrent_requests: 100,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub blocked_ips: Vec<String>,
    pub blocked_paths: Vec<String>,
    pub suspicious_patterns: Vec<String>,
    pub max_request_size_bytes: usize,
    pub allowed_origins: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            blocked_ips: vec![],
            blocked_paths: vec![
                "/\\.\\./".to_string(),  // Directory traversal
                "/<script".to_string(),  // XSS attempts
            ],
            suspicious_patterns: vec![
                "union.*select".to_string(),  // SQL injection
                "eval\\(".to_string(),        // Code injection
            ],
            max_request_size_bytes: 10 * 1024 * 1024, // 10MB
            allowed_origins: vec!["*".to_string()],
        }
    }
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub tracing_enabled: bool,
    pub metrics_enabled: bool,
    pub jaeger_endpoint: Option<String>,
    pub prometheus_port: Option<u16>,
    pub log_level: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            tracing_enabled: true,
            metrics_enabled: true,
            jaeger_endpoint: Some("http://localhost:16686".to_string()),
            prometheus_port: Some(9090),
            log_level: "info".to_string(),
        }
    }
}

/// Configuration builder
pub struct ConfigBuilder {
    config: FortressConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: FortressConfig::default(),
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

    pub fn with_mcp(mut self, mcp: McpConfig) -> Self {
        self.config.mcp = mcp;
        self
    }

    pub fn with_security(mut self, security: SecurityConfig) -> Self {
        self.config.security = security;
        self
    }

    pub fn with_observability(mut self, observability: ObservabilityConfig) -> Self {
        self.config.observability = observability;
        self
    }

    pub fn build(self) -> FortressConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Load configuration from file or environment
pub fn load_config() -> Result<FortressConfig, Box<dyn std::error::Error>> {
    // Try to load from config file first
    if let Ok(config_content) = std::fs::read_to_string("fortress.toml") {
        return Ok(toml::from_str(&config_content)?);
    }

    // Try to load from environment variables
    // This would be expanded with actual env var loading logic

    // Fall back to default configuration
    Ok(FortressConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FortressConfig::default();
        assert!(config.auth.enabled);
        assert!(config.rate_limit.enabled);
        assert!(config.cache.enabled);
        assert!(config.mcp.bv_enterprise_registry_url.contains("BVEnterprisess"));
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .with_auth(AuthConfig {
                enabled: false,
                ..Default::default()
            })
            .build();

        assert!(!config.auth.enabled);
    }
}
