#!/bin/bash
# Infrastructure Assassin MCP Server - Filesystem Access
# Zero-trust filesystem operations for tool orchestration

# Configuration
MCP_SERVER_PORT="${MCP_SERVER_PORT:-3001}"
WORKSPACE_PATH="${WORKSPACE_PATH:-${INFRASTRUCTURE_ASSASSIN_WORKSPACE:-/tmp}}"

echo "🚀 Starting Infrastructure Assassin FileSystem MCP Server"
echo "📁 Workspace: ${WORKSPACE_PATH}"
echo "🔌 Port: ${MCP_SERVER_PORT}"

# Start MCP filesystem server
npx @modelcontextprotocol/server-filesystem "${WORKSPACE_PATH}" \
  --port "${MCP_SERVER_PORT}" \
  --allowed-paths "${WORKSPACE_PATH}" \
  --read-only-mode false \
  --log-level "${INFRASTRUCTURE_ASSASSIN_LOG_LEVEL:-info}"
