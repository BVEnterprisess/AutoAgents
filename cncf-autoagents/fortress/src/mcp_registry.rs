//! MCP Registry Integration
//!
//! Integrates with BVEnterprisess MCP registry and awesome-mcp-servers
//! to provide a unified, health-checked, and cached MCP server directory.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

use crate::config::McpConfig;

/// MCP Server Registry
#[derive(Clone)]
pub struct McpRegistry {
    config: McpConfig,
    bv_servers: Arc<RwLock<HashMap<String, BvServer>>>,
    awesome_servers: Arc<RwLock<HashMap<String, AwesomeServer>>>,
    health_status: Arc<RwLock<HashMap<String, ServerHealth>>>,
}

impl McpRegistry {
    /// Create a new MCP registry
    pub async fn new(config: McpConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let registry = Self {
            config,
            bv_servers: Arc::new(RwLock::new(HashMap::new())),
            awesome_servers: Arc::new(RwLock::new(HashMap::new())),
            health_status: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initial load of servers
        registry.load_bv_servers().await?;
        if let Some(url) = &registry.config.awesome_servers_url {
            registry.load_awesome_servers(url).await?;
        }

        // Start health check loop
        registry.start_health_checks();

        Ok(registry)
    }

    /// Load BVEnterprisess registry servers
    async fn load_bv_servers(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading BVEnterprisess MCP registry from: {}", self.config.bv_enterprise_registry_url);

        let client = reqwest::Client::new();
        let response = client
            .get(&self.config.bv_enterprise_registry_url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            warn!("Failed to load BV registry: HTTP {}", response.status());
            return Ok(()); // Don't fail, just warn
        }

        let registry_data: BvRegistryResponse = response.json().await?;
        let mut bv_servers = self.bv_servers.write().await;

        for server in registry_data.servers {
            bv_servers.insert(server.name.clone(), server);
        }

        info!("Loaded {} BVEnterprisess MCP servers", bv_servers.len());
        Ok(())
    }

    /// Load awesome-mcp-servers
    async fn load_awesome_servers(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading awesome MCP servers from: {}", url);

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            warn!("Failed to load awesome servers: HTTP {}", response.status());
            return Ok(());
        }

        let readme_content = response.text().await?;
        let servers = self.parse_awesome_servers(&readme_content)?;
        let mut awesome_servers = self.awesome_servers.write().await;

        for server in servers {
            awesome_servers.insert(server.name.clone(), server);
        }

        info!("Loaded {} awesome MCP servers", awesome_servers.len());
        Ok(())
    }

    /// Parse awesome-mcp-servers README for server information
    fn parse_awesome_servers(&self, content: &str) -> Result<Vec<AwesomeServer>, Box<dyn std::error::Error>> {
        let mut servers = Vec::new();

        // Simple regex-based parsing of markdown links
        let link_regex = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)")?;

        for line in content.lines() {
            if let Some(captures) = link_regex.captures(line) {
                let name = captures.get(1).unwrap().as_str().to_string();
                let url = captures.get(2).unwrap().as_str().to_string();

                // Extract GitHub repo information
                if url.contains("github.com") {
                    servers.push(AwesomeServer {
                        name,
                        github_url: url,
                        description: None,
                        tags: vec![],
                    });
                }
            }
        }

        Ok(servers)
    }

    /// Start periodic health checks
    fn start_health_checks(&self) {
        let registry = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(registry.config.health_check_interval_seconds)
            );

            loop {
                interval.tick().await;
                registry.perform_health_checks().await;
            }
        });
    }

    /// Perform health checks on all servers
    async fn perform_health_checks(&self) {
        let bv_servers = self.bv_servers.read().await.clone();
        let awesome_servers = self.awesome_servers.read().await.clone();

        // Check BV servers
        for (name, server) in bv_servers {
            let health = self.check_server_health(&server.endpoint).await;
            self.update_server_health(&name, health).await;
        }

        // Check awesome servers (simplified - just check if repo exists)
        for (name, server) in awesome_servers {
            let health = self.check_github_repo_health(&server.github_url).await;
            self.update_server_health(&name, health).await;
        }
    }

    /// Check server health by making a simple request
    async fn check_server_health(&self, endpoint: &str) -> ServerHealth {
        let client = reqwest::Client::new();
        let start = std::time::Instant::now();

        match client
            .get(endpoint)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                let latency = start.elapsed();
                if response.status().is_success() {
                    ServerHealth::Healthy { latency }
                } else {
                    ServerHealth::Unhealthy {
                        reason: format!("HTTP {}", response.status()),
                    }
                }
            }
            Err(e) => ServerHealth::Unhealthy {
                reason: format!("Connection failed: {}", e),
            },
        }
    }

    /// Check GitHub repo health
    async fn check_github_repo_health(&self, github_url: &str) -> ServerHealth {
        // Simplified health check - just verify the URL is accessible
        let client = reqwest::Client::new();

        match client
            .head(github_url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    ServerHealth::Healthy {
                        latency: std::time::Duration::from_millis(100), // Placeholder
                    }
                } else {
                    ServerHealth::Unhealthy {
                        reason: format!("HTTP {}", response.status()),
                    }
                }
            }
            Err(e) => ServerHealth::Unhealthy {
                reason: format!("Connection failed: {}", e),
            },
        }
    }

    /// Update server health status
    async fn update_server_health(&self, server_name: &str, health: ServerHealth) {
        let mut health_status = self.health_status.write().await;
        health_status.insert(server_name.to_string(), health);
    }

    /// Get all healthy BV servers
    pub async fn get_healthy_bv_servers(&self) -> HashMap<String, BvServer> {
        let bv_servers = self.bv_servers.read().await;
        let health_status = self.health_status.read().await;

        bv_servers
            .iter()
            .filter(|(name, _)| matches!(
                health_status.get(*name),
                Some(ServerHealth::Healthy { .. })
            ))
            .map(|(name, server)| (name.clone(), server.clone()))
            .collect()
    }

    /// Get all healthy awesome servers
    pub async fn get_healthy_awesome_servers(&self) -> HashMap<String, AwesomeServer> {
        let awesome_servers = self.awesome_servers.read().await;
        let health_status = self.health_status.read().await;

        awesome_servers
            .iter()
            .filter(|(name, _)| matches!(
                health_status.get(*name),
                Some(ServerHealth::Healthy { .. })
            ))
            .map(|(name, server)| (name.clone(), server.clone()))
            .collect()
    }

    /// Get server health status
    pub async fn get_server_health(&self, server_name: &str) -> Option<ServerHealth> {
        self.health_status.read().await.get(server_name).cloned()
    }

    /// Get server by name from either registry
    pub async fn get_server(&self, name: &str) -> Option<McpServerInfo> {
        // Check BV servers first
        if let Some(server) = self.bv_servers.read().await.get(name) {
            return Some(McpServerInfo::Bv(server.clone()));
        }

        // Check awesome servers
        if let Some(server) = self.awesome_servers.read().await.get(name) {
            return Some(McpServerInfo::Awesome(server.clone()));
        }

        None
    }

    /// Search servers by capability
    pub async fn search_by_capability(&self, capability: &str) -> Vec<McpServerInfo> {
        let mut results = Vec::new();

        // Search BV servers
        for server in self.bv_servers.read().await.values() {
            if server.capabilities.iter().any(|cap| cap.contains(capability)) {
                results.push(McpServerInfo::Bv(server.clone()));
            }
        }

        // Search awesome servers by tags
        for server in self.awesome_servers.read().await.values() {
            if server.tags.iter().any(|tag| tag.contains(capability)) {
                results.push(McpServerInfo::Awesome(server.clone()));
            }
        }

        results
    }

    /// Get registry statistics
    pub async fn get_stats(&self) -> RegistryStats {
        let bv_count = self.bv_servers.read().await.len();
        let awesome_count = self.awesome_servers.read().await.len();
        let health_status = self.health_status.read().await;

        let healthy_count = health_status.values()
            .filter(|health| matches!(health, ServerHealth::Healthy { .. }))
            .count();

        RegistryStats {
            total_servers: bv_count + awesome_count,
            bv_servers: bv_count,
            awesome_servers: awesome_count,
            healthy_servers: healthy_count,
        }
    }
}

/// BVEnterprisess registry response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BvRegistryResponse {
    pub servers: Vec<BvServer>,
}

/// BVEnterprisess MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BvServer {
    pub name: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub auth_required: bool,
    pub description: Option<String>,
}

/// Awesome MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwesomeServer {
    pub name: String,
    pub github_url: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

/// Server health status
#[derive(Debug, Clone)]
pub enum ServerHealth {
    Healthy { latency: std::time::Duration },
    Unhealthy { reason: String },
}

/// Unified server information
#[derive(Debug, Clone)]
pub enum McpServerInfo {
    Bv(BvServer),
    Awesome(AwesomeServer),
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_servers: usize,
    pub bv_servers: usize,
    pub awesome_servers: usize,
    pub healthy_servers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let config = McpConfig::default();
        let registry = McpRegistry::new(config).await;
        assert!(registry.is_ok());
    }

    #[test]
    fn test_registry_stats() {
        let stats = RegistryStats {
            total_servers: 10,
            bv_servers: 5,
            awesome_servers: 5,
            healthy_servers: 8,
        };

        assert_eq!(stats.total_servers, 10);
        assert_eq!(stats.healthy_servers, 8);
    }
}
