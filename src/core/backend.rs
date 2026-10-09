// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::core::error::AegisResult;
use crate::core::transport::{JsonRpcRequest, JsonRpcResponse};
use crate::core::types::ToolDefinition;

/// Individual backend MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_timeout() -> u64 {
    30
}

fn default_enabled() -> bool {
    true
}

/// Topology configuration file mapping server names to configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AegisTopologyConfig {
    #[serde(alias = "mcpServers", default)]
    pub mcp_servers: HashMap<String, BackendConfig>,
    #[serde(default)]
    pub default_tenant: Option<String>,
}

/// Upstream backend transport abstraction (Stdio subprocess or Remote HTTP/SSE)
#[async_trait]
pub trait BackendTransport: Send + Sync {
    async fn start(&self) -> AegisResult<()>;
    async fn send_request(&self, request: &JsonRpcRequest) -> AegisResult<JsonRpcResponse>;
    async fn is_healthy(&self) -> bool;
    async fn stop(&self) -> AegisResult<()>;
}

/// Upstream backend registry abstraction managing heterogeneous transports
#[async_trait]
pub trait BackendRegistry: Send + Sync {
    async fn register(&self, name: &str, transport: Arc<dyn BackendTransport>) -> AegisResult<()>;
    async fn get(&self, name: &str) -> AegisResult<Arc<dyn BackendTransport>>;
    async fn list_backends(&self) -> AegisResult<Vec<String>>;
    async fn discover_all_tools(&self) -> AegisResult<Vec<ToolDefinition>>;
}
