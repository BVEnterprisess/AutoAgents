# CNCF AutoAgents Implementation Plan

## Overview
Implement a streamlined, production-ready CNCF-based infrastructure stack following the "Fortress, Forge, Launchpad, Conductor" architecture. Focus on minimal viable components with maximum efficiency, integrating BVEnterprisess MCP registry alongside existing MCP servers while removing dead code and excess LLM providers.

## Types

### Core Infrastructure Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FortressConfig {
    pub linkerd: LinkerdConfig,
    pub mcp_registry: McpRegistryConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeConfig {
    pub spin: SpinConfig,
    pub wasm_sandbox: WasmSandboxConfig,
    pub execution_limits: ExecutionLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorConfig {
    pub agent_core: AgentCoreConfig,
    pub orchestration: OrchestrationConfig,
    pub redis: RedisConfig,
}
```

### MCP Integration Types
```rust
#[derive(Debug, Clone)]
pub struct McpRegistry {
    pub bv_enterprise_servers: HashMap<String, BvServer>,
    pub awesome_servers: HashMap<String, AwesomeServer>,
    pub health_checks: HealthMonitor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BvServer {
    pub name: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub auth_required: bool,
}
```

## Files

### New Files - Core Infrastructure
- `crates/fortress/src/lib.rs` - Main gateway/proxy service using Linkerd2-proxy
- `crates/fortress/src/mcp_registry.rs` - BVEnterprisess registry integration
- `crates/forge/src/lib.rs` - WASM execution sandbox using Fermyon Spin
- `crates/forge/src/sandbox.rs` - Secure WASM runtime environment
- `crates/conductor/src/lib.rs` - Core agent orchestration engine
- `crates/conductor/src/agent_core.rs` - Streamlined agent framework
- `infrastructure/nixos/fortress.nix` - NixOS module for gateway
- `infrastructure/nixos/forge.nix` - NixOS module for WASM runtime
- `infrastructure/nixos/conductor.nix` - NixOS module for orchestration
- `infrastructure/docker-compose.cncf.yml` - CNCF stack deployment

### Modified Files - Cleanup & Optimization
- `Cargo.toml` - Remove excess LLM provider dependencies, keep core + infrastructure
- `crates/core/src/lib.rs` - Streamline to essential agent functionality
- `crates/llm/src/lib.rs` - Reduce to Ollama + one API provider only
- `cline_mcp_settings.json` - Add BVEnterprisess registry servers
- `flake.nix` - Update with optimized CNCF components

### Files to Remove - Dead Code Cleanup
- `crates/liquid-edge/` - Remove entire crate (excess complexity)
- `examples/wasm_tool/` - Remove (excluded in workspace)
- `crates/llm/src/backends/` - Remove all except Ollama + one API provider
- `examples/image_chat/` - Remove (non-essential)
- `examples/providers/` - Remove (excess LLM provider examples)

## Functions

### New Functions - Core Services
```rust
// Fortress (Gateway)
pub async fn init_fortress(config: FortressConfig) -> Result<FortressService, Error>
pub async fn route_with_mcp_registry(req: Request) -> Result<Response, Error>

// Forge (WASM Runtime)
pub async fn init_forge(config: ForgeConfig) -> Result<ForgeService, Error>
pub async fn execute_wasm_secure(module: WasmModule, input: Value) -> Result<Value, Error>

// Conductor (Orchestration)
pub async fn init_conductor(config: ConductorConfig) -> Result<ConductorService, Error>
pub async fn orchestrate_agent_task(task: AgentTask) -> Result<TaskResult, Error>
```

### Modified Functions - Streamlined Core
- `Agent::execute()` - Simplify to essential orchestration logic
- `LLMProvider::chat()` - Reduce to Ollama + one API provider
- `Tool::invoke()` - Streamline to core MCP integration

## Classes

### New Classes - CNCF Architecture
```rust
pub struct FortressService {
    linkerd_proxy: LinkerdProxy,
    mcp_registry: McpRegistry,
    security_enforcer: SecurityEnforcer,
}

pub struct ForgeService {
    spin_runtime: SpinRuntime,
    wasm_sandbox: WasmSandbox,
    resource_monitor: ResourceMonitor,
}

pub struct ConductorService {
    agent_core: AgentCore,
    redis_cache: RedisCache,
    task_orchestrator: TaskOrchestrator,
}
```

### Modified Classes - Optimization
- `InfrastructureAssassin` - Rename to `ConductorService` and streamline
- `LLMProvider` - Reduce to essential interface with 2 implementations
- `Agent` - Focus on core execution logic, remove excess features

## Dependencies

### New Dependencies - CNCF Stack
- `linkerd2-proxy = "0.1"` - Core proxy service
- `spin-sdk = "2.0"` - WASM runtime
- `redis = "0.23"` - Distributed caching
- `bv-enterprise-mcp = "0.1"` - MCP registry client

### Removed Dependencies - Cleanup
- Remove all LLM provider crates except Ollama + one API
- Remove `liquid-edge` dependencies
- Remove `minijinja`, `base64`, `ndarray` (non-essential)
- Remove `tempfile`, `getrandom` (dev-only)

### Optimized Dependencies - Core Only
```toml
[dependencies]
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
serde = { version = "1.0", features = ["derive"] }
reqwest = { version = "0.12", features = ["json"] }
```

## Testing

### New Tests - CNCF Integration
- `tests/fortress_integration_test.rs` - Gateway + MCP registry
- `tests/forge_execution_test.rs` - WASM sandbox security
- `tests/conductor_orchestration_test.rs` - Agent task flow
- `tests/mcp_registry_test.rs` - BVEnterprisess integration

### Streamlined Tests - Essential Only
- Remove tests for removed LLM providers
- Focus on core agent functionality
- Add integration tests for CNCF components

## Implementation Order

1. **Phase 1: Infrastructure Foundation (Days 1-2)**
   - Clean up Cargo.toml, remove excess dependencies
   - Set up NixOS modules for CNCF components
   - Initialize BVEnterprisess MCP registry integration

2. **Phase 2: Fortress Construction (Days 3-4)**
   - Implement Linkerd2-proxy based gateway
   - Integrate MCP registry with existing servers
   - Add security and routing middleware

3. **Phase 3: Forge Assembly (Days 5-6)**
   - Set up Fermyon Spin WASM runtime
   - Implement secure sandbox environment
   - Add resource monitoring and limits

4. **Phase 4: Conductor Integration (Day 7)**
   - Build streamlined agent orchestration
   - Integrate Redis for caching/messaging
   - Connect all components into cohesive system

5. **Phase 5: Testing & Optimization (Ongoing)**
   - End-to-end integration testing
   - Performance optimization
   - Documentation and deployment guides
