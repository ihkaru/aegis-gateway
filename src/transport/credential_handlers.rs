// SPDX-License-Identifier: MIT

use serde_json::json;
use serde_json::Value;

use crate::core::transport::{JsonRpcError, JsonRpcId, JsonRpcResponse, INVALID_PARAMS};
use crate::PropylonGateway;

pub struct CredentialAndToolInstallerDispatcher;

impl CredentialAndToolInstallerDispatcher {
    /// Zero-Knowledge Credential Discovery handler
    pub async fn handle_list_credentials(
        id: JsonRpcId,
        gateway: &PropylonGateway,
    ) -> JsonRpcResponse {
        let keys = gateway.secret_store().list_secret_keys().await.unwrap_or_default();
        JsonRpcResponse::success(id, json!({
            "credentials": keys,
            "count": keys.len(),
            "usage": "Specify any discovered key name in execute_code's 'services' parameter (e.g. ['AINA_GDRIVE_TOKEN_JSON', 'GH_TOKEN']) to automatically inject authenticated sessions into the sandbox."
        }))
    }

    /// On-demand persistent package/tool installer via mise
    pub async fn handle_install_tool(
        id: JsonRpcId,
        gateway: &PropylonGateway,
        params: Option<Value>,
    ) -> JsonRpcResponse {
        let args = match params {
            Some(p) => p,
            None => return JsonRpcResponse::error(id, JsonRpcError::new(INVALID_PARAMS, "Missing tool parameter")),
        };
        let tool_name = match args.get("tool").and_then(|t| t.as_str()) {
            Some(t) => t.trim(),
            None => return JsonRpcResponse::error(id, JsonRpcError::new(INVALID_PARAMS, "Missing 'tool' field")),
        };
        let bin_name = tool_name.split('@').next().unwrap_or(tool_name);
        let code = format!("mise use -g {tool_name} && mise reshim && (which {bin_name} || true) 2>&1");
        let req = crate::core::sandbox::SandboxExecutionRequest {
            code,
            language: crate::core::sandbox::ExecutionLanguage::Bash,
            services: vec![],
            timeout_secs: Some(120),
            env_vars: std::collections::HashMap::new(),
            tenant_id: None,
            caller_id: None,
        };
        match gateway.sandbox().execute(&req).await {
            Ok(outcome) => JsonRpcResponse::success(id, json!({
                "tool": tool_name,
                "installed": outcome.exit_code == 0,
                "output": outcome.stdout.trim(),
                "error": outcome.stderr.trim(),
                "persistent_path": format!("/app/data/mise/bin/{tool_name}")
            })),
            Err(e) => JsonRpcResponse::error(id, e.to_rpc_error()),
        }
    }
}
