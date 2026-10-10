// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::backend::BackendRegistry;
use crate::core::error::{AegisError, AegisResult};
use crate::core::transport::{
    JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, WireProtocolHandler,
    INVALID_PARAMS, METHOD_NOT_FOUND, PARSE_ERROR,
};
use crate::core::types::{
    CallerContext, TenantId, ToolCallRequest, ToolDefinition,
};
use crate::discovery::ExecutionPlanner;
use crate::transport::meta_handlers::MetaToolDispatcher;
use crate::AegisGateway;

/// Core MCP wire protocol engine handling standard JSON-RPC 2.0 lifecycle
pub struct McpProtocolHandler {
    gateway: Arc<AegisGateway>,
    registry: Option<Arc<dyn BackendRegistry>>,
    planner: ExecutionPlanner,
    tools: Arc<RwLock<Vec<ToolDefinition>>>,
    server_name: String,
    server_version: String,
}

impl McpProtocolHandler {
    pub fn new(gateway: Arc<AegisGateway>) -> Self {
        Self::with_registry(gateway, None)
    }

    pub fn with_registry(
        gateway: Arc<AegisGateway>,
        registry: Option<Arc<dyn BackendRegistry>>,
    ) -> Self {
        Self {
            gateway,
            registry,
            planner: ExecutionPlanner::new(),
            tools: Arc::new(RwLock::new(Vec::new())),
            server_name: "aegis-gateway".to_string(),
            server_version: "0.9.0".to_string(),
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
        let mut tool_list: Vec<Value> = read
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema
                })
            })
            .collect();

        // Advertise first-class built-in meta-tools and sandbox
        tool_list.push(json!({
            "name": "execute_code",
            "description": "Execute context-agnostic Python/Bash/Node code in hermetic sandbox with zero-knowledge credential broker and egress firewall",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "The script source code to execute" },
                    "language": { "type": "string", "description": "Execution language: python (default), bash, javascript" },
                    "services": { "type": "array", "items": { "type": "string" }, "description": "Services requiring brokered credentials (e.g. ['google', 'github', 'aws'])" },
                    "timeout_secs": { "type": "integer", "description": "Maximum execution time in seconds (default 30)" },
                    "env_vars": { "type": "object", "description": "Optional environment variables" }
                },
                "required": ["code"]
            }
        }));
        tool_list.push(json!({
            "name": "gateway_execute_code",
            "description": "Execute context-agnostic code in hermetic sandbox (alias for execute_code)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "string" },
                    "language": { "type": "string" },
                    "services": { "type": "array", "items": { "type": "string" } },
                    "timeout_secs": { "type": "integer" }
                },
                "required": ["code"]
            }
        }));
        tool_list.push(json!({
            "name": "gateway_search_tools",
            "description": "Progressive tool discovery across active MCP backend servers",
            "inputSchema": { "type": "object", "properties": { "query": { "type": "string" } }, "required": ["query"] }
        }));
        tool_list.push(json!({
            "name": "gateway_plan_tasks",
            "description": "Formulate and preflight multi-step execution plans across tools against zero-trust policy",
            "inputSchema": { "type": "object", "properties": { "plan_id": { "type": "string" }, "steps": { "type": "array" } }, "required": ["steps"] }
        }));
        tool_list.push(json!({
            "name": "gateway_list_servers",
            "description": "List connected upstream backend MCP servers and operational status",
            "inputSchema": { "type": "object", "properties": {} }
        }));
        tool_list.push(json!({
            "name": "gateway_register_tools",
            "description": "Dynamically register or update tool definitions into gateway catalog",
            "inputSchema": { "type": "object", "properties": { "tools": { "type": "array" } }, "required": ["tools"] }
        }));

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

        // Handle meta-tools and sandbox directly
        if tool_name == "execute_code" || tool_name == "gateway_execute_code" {
            return MetaToolDispatcher::handle_execute_code(id, &self.gateway, Some(arguments)).await;
        }
        if tool_name == "gateway_search_tools" {
            return MetaToolDispatcher::handle_search(id, self.tools.clone(), Some(arguments)).await;
        }
        if tool_name == "gateway_plan_tasks" {
            return MetaToolDispatcher::handle_plan(
                id,
                self.tools.clone(),
                &self.gateway,
                &self.planner,
                Some(arguments),
            )
            .await;
        }
        if tool_name == "gateway_list_servers" {
            return MetaToolDispatcher::handle_list_servers(id, self.registry.as_ref()).await;
        }
        if tool_name == "gateway_register_tools" {
            return self.handle_meta_register(id, Some(arguments)).await;
        }

        let tool_info = {
            let read = self.tools.read().await;
            read.iter().find(|t| t.name == tool_name).cloned()
        };

        let server_name = tool_info
            .as_ref()
            .map(|t| t.server.clone())
            .unwrap_or_else(|| "default".to_string());

        let req = ToolCallRequest {
            tool: tool_name.clone(),
            server: server_name.clone(),
            arguments: arguments.clone(),
            caller: Self::default_caller(),
        };

        let reg_opt = self.registry.clone();
        let srv = server_name.clone();
        let t_name = tool_name.clone();
        let args = arguments.clone();
        let id_cloned = id.clone();

        let exec_closure = move || async move {
            if let Some(reg) = reg_opt {
                let backend = reg.get(&srv).await?;
                let backend_req = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    id: Some(id_cloned),
                    method: "tools/call".to_string(),
                    params: Some(json!({
                        "name": t_name,
                        "arguments": args
                    })),
                };
                let resp = backend.send_request(&backend_req).await?;
                if let Some(err) = resp.error {
                    Err(AegisError::Internal(format!("Backend error: {}", err.message)))
                } else {
                    Ok(resp.result.unwrap_or_else(|| json!({})))
                }
            } else {
                Ok(json!({ "result": "completed", "tool": t_name, "server": srv }))
            }
        };

        match self.gateway.execute_tool_async(req, exec_closure).await {
            Ok(resp) => {
                let content_val = if let Some(content) = resp.output.get("content") {
                    content.clone()
                } else {
                    json!([{
                        "type": "text",
                        "text": serde_json::to_string(&resp.output).unwrap_or_default()
                    }])
                };
                JsonRpcResponse::success(
                    id,
                    json!({
                        "content": content_val,
                        "isError": !resp.success
                    }),
                )
            }
            Err(e) => JsonRpcResponse::error(id, e.to_rpc_error()),
        }
    }

    async fn handle_meta_register(&self, id: JsonRpcId, params: Option<Value>) -> JsonRpcResponse {
        let tools_val = params.and_then(|p| p.get("tools").cloned()).unwrap_or(Value::Array(Vec::new()));
        let new_tools: Vec<ToolDefinition> = serde_json::from_value(tools_val).unwrap_or_default();
        let count = new_tools.len();
        self.register_tools(new_tools).await;
        JsonRpcResponse::success(id, json!({ "status": "registered", "tools_count": count }))
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
            "resources/list" => JsonRpcResponse::success(id, json!({ "resources": [] })),
            "prompts/list" => JsonRpcResponse::success(id, json!({ "prompts": [] })),
            "execute_code" | "gateway_execute_code" => {
                MetaToolDispatcher::handle_execute_code(id, &self.gateway, req.params).await
            }
            "gateway_search_tools" => {
                MetaToolDispatcher::handle_search(id, self.tools.clone(), req.params).await
            }
            "gateway_plan_tasks" => {
                MetaToolDispatcher::handle_plan(
                    id,
                    self.tools.clone(),
                    &self.gateway,
                    &self.planner,
                    req.params,
                )
                .await
            }
            "gateway_list_servers" => {
                MetaToolDispatcher::handle_list_servers(id, self.registry.as_ref()).await
            }
            "gateway_register_tools" => self.handle_meta_register(id, req.params).await,
            other => JsonRpcResponse::error(
                id,
                JsonRpcError::new(METHOD_NOT_FOUND, format!("Method '{other}' not implemented")),
            ),
        };

        Ok(Some(serde_json::to_string(&resp)?))
    }
}
