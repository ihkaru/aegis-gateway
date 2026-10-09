// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::backend::{BackendRegistry, BackendTransport};
use crate::core::error::{AegisError, AegisResult};
use crate::core::transport::{JsonRpcId, JsonRpcRequest};
use crate::core::types::ToolDefinition;

/// In-memory backend registry coordinating multiple local and remote MCP servers
#[derive(Clone, Default)]
pub struct SubprocessBackendRegistry {
    backends: Arc<RwLock<HashMap<String, Arc<dyn BackendTransport>>>>,
}

impl SubprocessBackendRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl BackendRegistry for SubprocessBackendRegistry {
    async fn register(&self, name: &str, transport: Arc<dyn BackendTransport>) -> AegisResult<()> {
        let mut write = self.backends.write().await;
        write.insert(name.to_string(), transport);
        Ok(())
    }

    async fn get(&self, name: &str) -> AegisResult<Arc<dyn BackendTransport>> {
        let read = self.backends.read().await;
        read.get(name)
            .cloned()
            .ok_or_else(|| AegisError::Internal(format!("Backend '{name}' not found in registry")))
    }

    async fn list_backends(&self) -> AegisResult<Vec<String>> {
        let read = self.backends.read().await;
        Ok(read.keys().cloned().collect())
    }

    async fn discover_all_tools(&self) -> AegisResult<Vec<ToolDefinition>> {
        let read = self.backends.read().await;
        let mut all_tools = Vec::new();

        for (name, transport) in read.iter() {
            if !transport.is_healthy().await {
                continue;
            }

            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(JsonRpcId::Number(100)),
                method: "tools/list".to_string(),
                params: Some(json!({})),
            };

            if let Ok(resp) = transport.send_request(&req).await {
                if let Some(result) = resp.result {
                    if let Some(tools_arr) = result.get("tools").and_then(|t| t.as_array()) {
                        for t in tools_arr {
                            if let Some(tool_name) = t.get("name").and_then(|n| n.as_str()) {
                                let desc = t.get("description").and_then(|d| d.as_str()).unwrap_or("");
                                let schema = t.get("inputSchema").cloned().unwrap_or_else(|| json!({}));
                                all_tools.push(ToolDefinition {
                                    name: tool_name.to_string(),
                                    server: name.clone(),
                                    description: desc.to_string(),
                                    input_schema: schema,
                                    required_params: Vec::new(),
                                    when_to_use: format!("When using backend {name}"),
                                    tags: vec![name.clone()],
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(all_tools)
    }
}
