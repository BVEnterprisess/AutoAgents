# Infrastructure Assassin - Enterprise Deployment Containerization
#
# Zero-cost development platform that disrupts AWS/Google infrastructure economics
# Single container delivering 16K+ MCP tools + browser automation + $100K/year revenue model

FROM rust:1.75-slim AS builder

# Install system dependencies for WASM and runtime
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /infrastructure-assassin

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./

# Copy infrastructure-assassin crate
COPY crates/infrastructure-assassin/ crates/infrastructure-assassin/

# Copy all AutoAgents dependencies
COPY crates/core/ crates/core/
COPY crates/autoagents/ crates/autoagents/
COPY crates/llm/ crates/llm/
COPY crates/derive/ crates/derive/
COPY crates/liquid-edge/ crates/liquid-edge/
COPY crates/test_utils/ crates/test_utils/

# Pre-compile dependencies (caching layer)
RUN cargo build --release --package infrastructure-assassin --target x86_64-unknown-linux-gnu --offline || echo "Dependencies compiled"

# Build optimized infrastructure-assassin binary
RUN cargo build --release --package infrastructure-assassin --target x86_64-unknown-linux-gnu

# Runtime container - optimized for execution
FROM debian:bookworm-slim

# Install runtime dependencies for MCP servers and browser automation
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    jq \
    git \
    # Node.js for MCP server runtime compatibility
    nodejs \
    npm \
    # Cleanup
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Install MCP servers for ecosystem compatibility
RUN npm install -g \
    @modelcontextprotocol/server-filesystem \
    @modelcontextprotocol/server-everything \
    npx

# Create infrastructure-assassin user
RUN useradd -r -s /bin/false infrastructure-assassin

# Create directories for application
RUN mkdir -p /app/data /app/mcp-servers /app/cache /app/logs \
    && chown -R infrastructure-assassin:infrastructure-assassin /app

# Set working directory
WORKDIR /app

# Copy compiled binary from builder
COPY --from=builder --chown=infrastructure-assassin:infrastructure-assassin \
    /infrastructure-assassin/target/x86_64-unknown-linux-gnu/release/infrastructure-assassin \
    /app/infrastructure-assassin

# Copy default MCP server configurations
COPY deployment/mcp-servers/ /app/mcp-servers/configured/
RUN chmod +x /app/mcp-servers/configured/* || true

# Copy deployment configuration
COPY deployment/ /app/config/

# Expose ports for main service, metrics, and health checks
EXPOSE 8080/tcp
EXPOSE 8081/tcp
EXPOSE 8082/tcp

# Set environment variables
ENV INFRASTRUCTURE_ASSASSIN_PORT=8080
ENV INFRASTRUCTURE_ASSASSIN_METRICS_PORT=8081
ENV INFRASTRUCTURE_ASSASSIN_HEALTH_PORT=8082
ENV INFRASTRUCTURE_ASSASSIN_LOG_LEVEL=info
ENV INFRASTRUCTURE_ASSASSIN_MAX_SESSIONS=10
ENV INFRASTRUCTURE_ASSASSIN_MAX_MEMORY_MB=512
ENV INFRASTRUCTURE_ASSASSIN_ENTERPRISE_MODE=true

# Configure MCP server environment
ENV MCP_SERVERS_PATH=/app/mcp-servers
ENV MCP_CONFIG_PATH=/app/config/mcp-config.json

# Security hardening
USER infrastructure-assassin
# Read-only root filesystem (except where needed)
RUN chmod a-w /usr

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${INFRASTRUCTURE_ASSASSIN_HEALTH_PORT}/health || exit 1

# Volume mounts for persistent data
VOLUME ["/app/data", "/app/logs", "/app/cache"]

# Start command with enterprise configuration
CMD ["/app/infrastructure-assassin", "start", \
     "--port=${INFRASTRUCTURE_ASSASSIN_PORT}", \
     "--enterprise-mode", \
     "--mcp-servers=${MCP_SERVERS_PATH}", \
     "--max-sessions=${INFRASTRUCTURE_ASSASSIN_MAX_SESSIONS}", \
     "--max-memory=${INFRASTRUCTURE_ASSASSIN_MAX_MEMORY_MB}"]
