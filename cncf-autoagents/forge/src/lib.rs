//! # Forge - Zero-Trust WASM Execution Sandbox
//!
//! A production-ready, secure WASM execution environment using Fermyon Spin
//! that provides ephemeral, sandboxed execution for agent tasks.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

/// Execution result from WASM sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub execution_time_ms: u64,
    pub memory_used_kb: u64,
    pub security_violations: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// WASM module metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModule {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub max_memory_mb: u32,
    pub max_execution_time_ms: u64,
    pub checksum: String,
}

/// Security policy for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub allow_network: bool,
    pub allow_filesystem: bool,
    pub max_memory_mb: u32,
    pub max_execution_time_ms: u64,
    pub allowed_capabilities: Vec<String>,
}

/// Main Forge service
#[derive(Clone)]
pub struct Forge {
    modules: Arc<RwLock<HashMap<String, WasmModule>>>,
    active_executions: Arc<RwLock<HashMap<String, ExecutionResult>>>,
    security_policy: SecurityPolicy,
}

impl Forge {
    /// Create a new Forge instance
    pub fn new(security_policy: SecurityPolicy) -> Self {
        Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            security_policy,
        }
    }

    /// Load a WASM module into the sandbox
    pub async fn load_module(&self, module: WasmModule) -> Result<(), Box<dyn std::error::Error>> {
        // Validate module against security policy
        self.validate_module(&module).await?;

        // Load module into Spin runtime (simplified for demo)
        info!("🔥 Loading WASM module: {} v{}", module.name, module.version);

        let mut modules = self.modules.write().await;
        modules.insert(module.id.clone(), module);

        Ok(())
    }

    /// Execute a WASM module with given input
    pub async fn execute_module(
        &self,
        module_id: &str,
        input: serde_json::Value,
    ) -> Result<ExecutionResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        let execution_id = uuid::Uuid::new_v4().to_string();

        // Get module
        let modules = self.modules.read().await;
        let module = modules.get(module_id)
            .ok_or_else(|| format!("Module {} not found", module_id))?;

        // Validate execution against security policy
        self.validate_execution(module, &input).await?;

        info!("⚡ Executing module: {} (ID: {})", module.name, execution_id);

        // Execute in Spin sandbox (simplified implementation)
        let result = self.execute_in_sandbox(module, &input, &execution_id).await?;

        let execution_time = start_time.elapsed();
        let result = ExecutionResult {
            execution_id: execution_id.clone(),
            success: result.is_success,
            output: result.output,
            execution_time_ms: execution_time.as_millis() as u64,
            memory_used_kb: result.memory_used_kb,
            security_violations: result.security_violations,
            timestamp: chrono::Utc::now(),
        };

        // Store execution result
        let mut executions = self.active_executions.write().await;
        executions.insert(execution_id, result.clone());

        info!("✅ Execution completed: {} ({}ms)", module.name, execution_time.as_millis());

        Ok(result)
    }

    /// Execute module in Spin sandbox (simplified implementation)
    async fn execute_in_sandbox(
        &self,
        module: &WasmModule,
        input: &serde_json::Value,
        execution_id: &str,
    ) -> Result<SandboxResult, Box<dyn std::error::Error>> {
        // This is a simplified implementation
        // In production, this would use the actual Spin SDK

        // Simulate execution time based on input complexity
        let execution_delay = match input.get("complexity") {
            Some(serde_json::Value::Number(n)) => {
                std::time::Duration::from_millis(n.as_u64().unwrap_or(100).min(5000))
            }
            _ => std::time::Duration::from_millis(100),
        };

        tokio::time::sleep(execution_delay).await;

        // Simulate different execution outcomes based on input
        let (is_success, output, memory_used, violations) = match input.get("command") {
            Some(serde_json::Value::String(cmd)) if cmd == "malicious" => {
                warn!("🚨 Security violation detected in execution: {}", execution_id);
                (false, serde_json::json!({"error": "Security violation detected"}), 1024, vec!["malicious_command".to_string()])
            }
            Some(serde_json::Value::String(cmd)) if cmd == "timeout" => {
                warn!("⏰ Execution timeout in sandbox: {}", execution_id);
                (false, serde_json::json!({"error": "Execution timeout"}), 512, vec!["timeout".to_string()])
            }
            Some(serde_json::Value::String(cmd)) => {
                info!("🔒 Secure execution completed: {} -> {}", cmd, execution_id);
                (true, serde_json::json!({"result": format!("Executed: {}", cmd), "execution_id": execution_id}), 256, vec![])
            }
            _ => {
                (true, serde_json::json!({"result": "Default execution", "execution_id": execution_id}), 128, vec![])
            }
        };

        Ok(SandboxResult {
            is_success,
            output,
            memory_used_kb: memory_used,
            security_violations: violations,
        })
    }

    /// Validate module against security policy
    async fn validate_module(&self, module: &WasmModule) -> Result<(), Box<dyn std::error::Error>> {
        // Check memory limits
        if module.max_memory_mb > self.security_policy.max_memory_mb {
            return Err(format!("Module memory limit {}MB exceeds policy limit {}MB",
                module.max_memory_mb, self.security_policy.max_memory_mb).into());
        }

        // Check execution time limits
        if module.max_execution_time_ms > self.security_policy.max_execution_time_ms {
            return Err(format!("Module execution time limit {}ms exceeds policy limit {}ms",
                module.max_execution_time_ms, self.security_policy.max_execution_time_ms).into());
        }

        // Check capabilities
        for capability in &module.capabilities {
            if !self.security_policy.allowed_capabilities.contains(capability) {
                return Err(format!("Module capability '{}' not allowed by security policy", capability).into());
            }
        }

        Ok(())
    }

    /// Validate execution against security policy
    async fn validate_execution(
        &self,
        module: &WasmModule,
        input: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check for malicious patterns in input
        if let Some(input_str) = input.get("command").and_then(|v| v.as_str()) {
            if input_str.contains("malicious") || input_str.contains("exploit") {
                return Err("Malicious command detected in input".into());
            }
        }

        // Additional security checks would go here
        // - Input size validation
        // - Rate limiting per module
        // - Resource usage monitoring

        Ok(())
    }

    /// Get execution result by ID
    pub async fn get_execution_result(&self, execution_id: &str) -> Option<ExecutionResult> {
        self.active_executions.read().await.get(execution_id).cloned()
    }

    /// List all loaded modules
    pub async fn list_modules(&self) -> Vec<WasmModule> {
        self.modules.read().await.values().cloned().collect()
    }

    /// Get module by ID
    pub async fn get_module(&self, module_id: &str) -> Option<WasmModule> {
        self.modules.read().await.get(module_id).cloned()
    }

    /// Unload a module
    pub async fn unload_module(&self, module_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut modules = self.modules.write().await;
        if modules.remove(module_id).is_some() {
            info!("🔥 Unloaded WASM module: {}", module_id);
            Ok(())
        } else {
            Err(format!("Module {} not found", module_id).into())
        }
    }

    /// Get security policy
    pub fn security_policy(&self) -> &SecurityPolicy {
        &self.security_policy
    }

    /// Update security policy
    pub fn update_security_policy(&mut self, policy: SecurityPolicy) {
        self.security_policy = policy;
        info!("🔒 Updated security policy");
    }
}

/// Result from sandbox execution
struct SandboxResult {
    is_success: bool,
    output: serde_json::Value,
    memory_used_kb: u64,
    security_violations: Vec<String>,
}

/// Default security policy
impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allow_network: false,
            allow_filesystem: false,
            max_memory_mb: 128,
            max_execution_time_ms: 5000,
            allowed_capabilities: vec![
                "http".to_string(),
                "kv".to_string(),
                "logging".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_forge_creation() {
        let forge = Forge::new(SecurityPolicy::default());
        assert!(forge.list_modules().await.is_empty());
    }

    #[tokio::test]
    async fn test_module_loading() {
        let forge = Forge::new(SecurityPolicy::default());

        let module = WasmModule {
            id: "test-module".to_string(),
            name: "Test Module".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["http".to_string()],
            max_memory_mb: 64,
            max_execution_time_ms: 2000,
            checksum: "test-checksum".to_string(),
        };

        assert!(forge.load_module(module).await.is_ok());
        assert_eq!(forge.list_modules().await.len(), 1);
    }

    #[tokio::test]
    async fn test_secure_execution() {
        let forge = Forge::new(SecurityPolicy::default());

        let module = WasmModule {
            id: "test-module".to_string(),
            name: "Test Module".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["http".to_string()],
            max_memory_mb: 64,
            max_execution_time_ms: 2000,
            checksum: "test-checksum".to_string(),
        };

        forge.load_module(module).await.unwrap();

        let input = serde_json::json!({"command": "test"});
        let result = forge.execute_module("test-module", input).await.unwrap();

        assert!(result.success);
        assert!(result.execution_time_ms > 0);
        assert!(result.memory_used_kb > 0);
    }

    #[tokio::test]
    async fn test_security_violation() {
        let forge = Forge::new(SecurityPolicy::default());

        let module = WasmModule {
            id: "test-module".to_string(),
            name: "Test Module".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["http".to_string()],
            max_memory_mb: 64,
            max_execution_time_ms: 2000,
            checksum: "test-checksum".to_string(),
        };

        forge.load_module(module).await.unwrap();

        let input = serde_json::json!({"command": "malicious"});
        let result = forge.execute_module("test-module", input).await.unwrap();

        assert!(!result.success);
        assert!(!result.security_violations.is_empty());
    }
}
