// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::backend::BackendRegistry;
use crate::core::error::{PropylonError, PropylonResult};
use crate::core::transport::{
    JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, WireProtocolHandler,
    INVALID_PARAMS, METHOD_NOT_FOUND, PARSE_ERROR,
};
use crate::core::identity::GatewayServerIdentity;
use crate::core::types::{
    CallerContext, TenantId, ToolCallRequest, ToolDefinition,
};
use crate::discovery::ExecutionPlanner;
use crate::transport::meta_handlers::MetaToolDispatcher;
use crate::PropylonGateway;

/// Core MCP wire protocol engine handling standard JSON-RPC 2.0 lifecycle
pub struct McpProtocolHandler {
    gateway: Arc<PropylonGateway>,
    registry: Option<Arc<dyn BackendRegistry>>,
    planner: ExecutionPlanner,
    tools: Arc<RwLock<Vec<ToolDefinition>>>,
    server_identity: Arc<RwLock<GatewayServerIdentity>>,
}

impl McpProtocolHandler {
    pub fn new(gateway: Arc<PropylonGateway>) -> Self {
        Self::with_registry(gateway, None)
    }

    pub fn with_identity(
        gateway: Arc<PropylonGateway>,
        registry: Option<Arc<dyn BackendRegistry>>,
        identity: GatewayServerIdentity,
    ) -> Self {
        Self {
            gateway,
            registry,
            planner: ExecutionPlanner::new(),
            tools: Arc::new(RwLock::new(Vec::new())),
            server_identity: Arc::new(RwLock::new(identity)),
        }
    }

    pub fn with_registry(
        gateway: Arc<PropylonGateway>,
        registry: Option<Arc<dyn BackendRegistry>>,
    ) -> Self {
        Self::with_identity(gateway, registry, GatewayServerIdentity::default())
    }

    pub async fn set_server_identity(&self, identity: GatewayServerIdentity) {
        let mut id = self.server_identity.write().await;
        *id = identity;
    }

    pub async fn server_identity(&self) -> GatewayServerIdentity {
        self.server_identity.read().await.clone()
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
        let identity = self.server_identity().await;
        let result = json!({
            "protocolVersion": identity.protocol_version,
            "capabilities": {
                "tools": { "listChanged": true },
                "resources": {},
                "prompts": {}
            },
            "serverInfo": {
                "name": identity.name,
                "version": identity.version
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
        tool_list.extend(crate::transport::meta_schemas::get_built_in_meta_tools());

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

        // Handle meta-tools, plan execution, and dynamic tool invoker
        if let Some(resp) = self.dispatch_meta_or_plan(&tool_name, id.clone(), Some(arguments.clone())).await {
            return resp;
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
                    Err(PropylonError::Internal(format!("Backend error: {}", err.message)))
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
        let profile = crate::core::profile::EnterpriseRuntimeProfile::from_env();
        if profile.immutable_config {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::new(
                    -32003,
                    "Dynamic tool registration is prohibited under enterprise GitOps immutability policy. Tools must be declared via propylon.yaml.",
                ),
            );
        }
        let tools_val = params.and_then(|p| p.get("tools").cloned()).unwrap_or(Value::Array(Vec::new()));
        let new_tools: Vec<ToolDefinition> = serde_json::from_value(tools_val).unwrap_or_default();
        let count = new_tools.len();
        self.register_tools(new_tools).await;
        JsonRpcResponse::success(id, json!({ "status": "registered", "tools_count": count }))
    }

    async fn dispatch_meta_or_plan(
        &self,
        tool: &str,
        id: JsonRpcId,
        args: Option<Value>,
    ) -> Option<JsonRpcResponse> {
        match tool {
            "execute_code" | "gateway_execute_code" => {
                Some(MetaToolDispatcher::handle_execute_code(id, &self.gateway, args).await)
            }
            "evaluate_data_egress" | "gateway_evaluate_data_egress" => {
                Some(MetaToolDispatcher::handle_evaluate_data_egress(id, &self.gateway, args).await)
            }
            "execute_in_situ_query" | "gateway_execute_in_situ_query" => {
                Some(MetaToolDispatcher::handle_execute_in_situ_query(id, &self.gateway, args).await)
            }
            "resolve_approval" | "gateway_resolve_approval" => {
                Some(MetaToolDispatcher::handle_resolve_approval(id, &self.gateway, args).await)
            }
            "gateway_search_tools" => {
                Some(MetaToolDispatcher::handle_search(id, self.tools.clone(), args).await)
            }
            "gateway_plan_tasks" => {
                Some(MetaToolDispatcher::handle_plan(id, self.tools.clone(), &self.gateway, &self.planner, args).await)
            }
            "gateway_execute_plan" => {
                Some(crate::transport::plan_executor::PlanExecutorDispatcher::handle_execute_plan(
                    id, &self.gateway, self.tools.clone(), self.registry.clone(), args,
                ).await)
            }
            "gateway_call_tool" => {
                Some(crate::transport::plan_executor::PlanExecutorDispatcher::handle_call_tool(
                    id, &self.gateway, self.tools.clone(), self.registry.clone(), args,
                ).await)
            }
            "gateway_install_tool" => {
                Some(MetaToolDispatcher::handle_install_tool(id, &self.gateway, args).await)
            }
            "gateway_list_credentials" => {
                Some(MetaToolDispatcher::handle_list_credentials(id, &self.gateway).await)
            }
            "gateway_list_servers" => {
                Some(MetaToolDispatcher::handle_list_servers(id, self.registry.as_ref()).await)
            }
            "gateway_web_search" | "web_search" => {
                Some(crate::transport::search_handler::SearchToolDispatcher::handle_web_search(id, &self.gateway, args).await)
            }
            "gateway_register_tools" => Some(self.handle_meta_register(id, args).await),
            _ => None,
        }
    }
}

#[async_trait]
impl WireProtocolHandler for McpProtocolHandler {
    async fn handle_message(&self, raw_json: &str) -> PropylonResult<Option<String>> {
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
            other => {
                if let Some(resp) = self.dispatch_meta_or_plan(other, id.clone(), req.params).await {
                    resp
                } else {
                    JsonRpcResponse::error(
                        id,
                        JsonRpcError::new(METHOD_NOT_FOUND, format!("Method '{other}' not implemented")),
                    )
                }
            }
        };

        Ok(Some(serde_json::to_string(&resp)?))
    }
}
