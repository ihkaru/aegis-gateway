// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::AegisResult;
use crate::core::transport::{
    JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, WireProtocolHandler,
    INTERNAL_ERROR, INVALID_PARAMS, METHOD_NOT_FOUND, PARSE_ERROR,
};
use crate::core::types::{
    CallerContext, DisclosureTier, TenantId, ToolCallRequest, ToolDefinition,
};
use crate::AegisGateway;

/// Core MCP wire protocol engine handling standard JSON-RPC 2.0 lifecycle
pub struct McpProtocolHandler {
    gateway: Arc<AegisGateway>,
    tools: Arc<RwLock<Vec<ToolDefinition>>>,
    server_name: String,
    server_version: String,
}

impl McpProtocolHandler {
    pub fn new(gateway: Arc<AegisGateway>) -> Self {
        Self {
            gateway,
            tools: Arc::new(RwLock::new(Vec::new())),
            server_name: "aegis-gateway".to_string(),
            server_version: "0.8.0".to_string(),
        }
    }

    pub async fn register_tools(&self, new_tools: Vec<ToolDefinition>) {
        let mut write = self.tools.write().await;
        for tool in new_tools {
            write.retain(|t| t.name != tool.name);
            write.push(tool);
        }
    }

    fn default_caller() -> CallerContext {
        CallerContext {
            tenant_id: TenantId::new("default-tenant"),
            subject: "mcp-client-session".to_string(),
            roles: vec!["developer".to_string(), "agent".to_string()],
            department: Some("Engineering".to_string()),
            client_ip: None,
            session_id: "mcp-wire-session-1".to_string(),
        }
    }

    async fn handle_initialize(&self, id: JsonRpcId) -> JsonRpcResponse {
        let result = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": true },
                "resources": {},
                "prompts": {}
            },
            "serverInfo": {
                "name": self.server_name,
                "version": self.server_version
            }
        });
        JsonRpcResponse::success(id, result)
    }

    async fn handle_tools_list(&self, id: JsonRpcId) -> JsonRpcResponse {
        let read = self.tools.read().await;
        let tool_list: Vec<Value> = read
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema
                })
            })
            .collect();

        JsonRpcResponse::success(id, json!({ "tools": tool_list }))
    }

    async fn handle_tools_call(&self, id: JsonRpcId, params: Option<Value>) -> JsonRpcResponse {
        let params_val = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::new(INVALID_PARAMS, "Missing 'params' in tools/call"),
                );
            }
        };

        let tool_name = match params_val.get("name").and_then(|n| n.as_str()) {
            Some(n) => n.to_string(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::new(INVALID_PARAMS, "Missing 'name' field in tools/call params"),
                );
            }
        };

        let arguments = params_val
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let req = ToolCallRequest {
            tool: tool_name.clone(),
            server: "backend".to_string(),
            arguments: arguments.clone(),
            caller: Self::default_caller(),
        };

        match self.gateway.execute_tool(req, || Ok(json!({ "status": "executed", "tool": tool_name }))).await {
            Ok(resp) => {
                let content_text = serde_json::to_string(&resp.output).unwrap_or_default();
                JsonRpcResponse::success(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": content_text
                        }],
                        "isError": !resp.success
                    }),
                )
            }
            Err(e) => JsonRpcResponse::error(
                id,
                JsonRpcError::new(INTERNAL_ERROR, format!("Aegis policy or pipeline denial: {e}")),
            ),
        }
    }

    async fn handle_meta_search(&self, id: JsonRpcId, params: Option<Value>) -> JsonRpcResponse {
        let query = params
            .as_ref()
            .and_then(|p| p.get("query").or_else(|| p.get("arguments").and_then(|a| a.get("query"))))
            .and_then(|q| q.as_str())
            .unwrap_or("");

        let read = self.tools.read().await;
        let projected = self.gateway.discover_tools(&read, query, DisclosureTier::L0).await;
        JsonRpcResponse::success(id, json!({ "tools": projected }))
    }
}

#[async_trait]
impl WireProtocolHandler for McpProtocolHandler {
    async fn handle_message(&self, raw_json: &str) -> AegisResult<Option<String>> {
        let req: JsonRpcRequest = match serde_json::from_str(raw_json) {
            Ok(r) => r,
            Err(_) => {
                let err_resp = JsonRpcResponse::error(
                    JsonRpcId::Null,
                    JsonRpcError::new(PARSE_ERROR, "Parse error: invalid JSON-RPC 2.0"),
                );
                return Ok(Some(serde_json::to_string(&err_resp)?));
            }
        };

        // Notifications (e.g. notifications/initialized or any message without id) do not expect a response
        let id = match req.id {
            Some(id) => id,
            None => return Ok(None),
        };

        if req.method.starts_with("notifications/") {
            return Ok(None);
        }

        let resp = match req.method.as_str() {
            "initialize" => self.handle_initialize(id).await,
            "ping" => JsonRpcResponse::success(id, json!({})),
            "tools/list" => self.handle_tools_list(id).await,
            "tools/call" => self.handle_tools_call(id, req.params).await,
            "gateway_search_tools" => self.handle_meta_search(id, req.params).await,
            other => JsonRpcResponse::error(
                id,
                JsonRpcError::new(METHOD_NOT_FOUND, format!("Method '{other}' not implemented")),
            ),
        };

        Ok(Some(serde_json::to_string(&resp)?))
    }
}
