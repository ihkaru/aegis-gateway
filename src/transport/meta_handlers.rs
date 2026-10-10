// SPDX-License-Identifier: MIT

use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::sandbox::{ExecutionLanguage, SandboxExecutionRequest};
use crate::core::transport::{JsonRpcError, JsonRpcId, JsonRpcResponse, INVALID_PARAMS};
use crate::core::types::{DisclosureTier, ToolDefinition};
use crate::AegisGateway;

pub struct MetaToolDispatcher;

impl MetaToolDispatcher {
    pub async fn handle_search(
        id: JsonRpcId,
        tools: Arc<RwLock<Vec<ToolDefinition>>>,
        params: Option<Value>,
    ) -> JsonRpcResponse {
        let query = params
            .as_ref()
            .and_then(|p| p.get("query").or_else(|| p.get("arguments").and_then(|a| a.get("query"))))
            .and_then(|q| q.as_str())
            .unwrap_or("");

        let read = tools.read().await;
        let mut engine = crate::discovery::HybridSearchEngine::new();
        engine.index_tools("default", read.clone());
        let res = engine.search(query, DisclosureTier::L0, 20);
        JsonRpcResponse::success(
            id,
            json!({
                "tools": res.tools,
                "total_matches": res.total_candidates,
                "lexical_hits": res.lexical_hits,
                "semantic_hits": res.semantic_hits,
            }),
        )
    }

    pub async fn handle_plan(
        id: JsonRpcId,
        tools: Arc<RwLock<Vec<ToolDefinition>>>,
        gateway: &AegisGateway,
        planner: &crate::discovery::ExecutionPlanner,
        params: Option<Value>,
    ) -> JsonRpcResponse {
        let plan_val = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::new(INVALID_PARAMS, "Missing plan definition or goal in arguments"),
                );
            }
        };

        let read = tools.read().await;
        let caller = crate::core::types::CallerContext {
            tenant_id: crate::core::types::TenantId::new("default"),
            subject: "mcp-wire-client".to_string(),
            roles: vec!["admin".to_string()],
            department: None,
            client_ip: None,
            session_id: uuid::Uuid::new_v4().to_string(),
        };
        match crate::discovery::GoalPlanner::process_planning_request(
            &plan_val,
            &read,
            planner,
            gateway.policy(),
            &caller,
        ).await {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::new(INVALID_PARAMS, e.to_string())),
        }
    }

    pub async fn handle_list_servers(
        id: JsonRpcId,
        registry: Option<&Arc<dyn crate::core::backend::BackendRegistry>>,
    ) -> JsonRpcResponse {
        let servers = if let Some(reg) = registry {
            reg.list_backends().await.unwrap_or_default()
        } else {
            vec!["default".to_string()]
        };
        JsonRpcResponse::success(id, json!({ "servers": servers }))
    }

    pub async fn handle_execute_code(
        id: JsonRpcId,
        gateway: &AegisGateway,
        params: Option<Value>,
    ) -> JsonRpcResponse {
        let args = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::new(INVALID_PARAMS, "Missing arguments in execute_code"),
                );
            }
        };

        let code = match args.get("code").and_then(|c| c.as_str()) {
            Some(c) => c.to_string(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::new(INVALID_PARAMS, "Missing 'code' argument in execute_code"),
                );
            }
        };

        let lang_str = args.get("language").and_then(|l| l.as_str()).unwrap_or("python");
        let language = match lang_str.parse::<ExecutionLanguage>() {
            Ok(l) => l,
            Err(e) => {
                return JsonRpcResponse::error(id, JsonRpcError::new(INVALID_PARAMS, e.to_string()));
            }
        };

        let timeout_secs = args.get("timeout_secs").and_then(|t| t.as_u64());
        let services: Vec<String> = args
            .get("services")
            .and_then(|s| s.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let mut env_vars = std::collections::HashMap::new();
        if let Some(envs) = args.get("env_vars").and_then(|e| e.as_object()) {
            for (k, v) in envs {
                if let Some(s) = v.as_str() {
                    env_vars.insert(k.clone(), s.to_string());
                }
            }
        }

        let exec_req = SandboxExecutionRequest {
            language,
            code,
            timeout_secs,
            services,
            env_vars,
            tenant_id: None,
            caller_id: None,
        };

        match gateway.sandbox().execute(&exec_req).await {
            Ok(result) => {
                let content_text = format!(
                    "Exit Code: {}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
                    result.exit_code, result.stdout, result.stderr
                );
                JsonRpcResponse::success(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": content_text }],
                        "isError": !result.success,
                        "structured": result
                    }),
                )
            }
            Err(e) => JsonRpcResponse::error(id, e.to_rpc_error()),
        }
    }
}
