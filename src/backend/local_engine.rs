// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

use crate::core::backend::BackendTransport;
use crate::core::database::SqlEngine;
use crate::core::error::AegisResult;
use crate::core::transport::{JsonRpcId, JsonRpcRequest, JsonRpcResponse};

/// Real In-Memory Local Loopback Backend with Dependency Injection (DIP & ISP)
#[derive(Clone)]
pub struct LocalAegisBackend {
    sql_engine: Arc<dyn SqlEngine>,
}

impl Default for LocalAegisBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalAegisBackend {
    pub fn new() -> Self {
        Self {
            sql_engine: Arc::new(crate::backend::LocalSqliteEngine::with_default_path()),
        }
    }

    pub fn with_sql_engine(sql_engine: Arc<dyn SqlEngine>) -> Self {
        Self { sql_engine }
    }
}

#[async_trait]
impl BackendTransport for LocalAegisBackend {
    async fn start(&self) -> AegisResult<()> {
        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        self.sql_engine.is_healthy().await
    }

    async fn send_request(&self, request: &JsonRpcRequest) -> AegisResult<JsonRpcResponse> {
        let req_id = request.id.clone().unwrap_or(JsonRpcId::Number(1));
        match request.method.as_str() {
            "tools/list" => {
                let tools = json!({
                    "tools": [
                        {
                            "name": "aegis_system_health",
                            "description": "Inspect live gateway health and resource metrics",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "aegis_ping",
                            "description": "Ping local loopback backend for roundtrip latency test",
                            "inputSchema": { "type": "object", "properties": { "echo": { "type": "string" } } }
                        },
                        {
                            "name": "aegis_sqlite_query",
                            "description": "Execute live SQL query on local persistent SQLite database",
                            "inputSchema": { "type": "object", "properties": { "query": { "type": "string" } }, "required": ["query"] }
                        }
                    ]
                });
                Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req_id,
                    result: Some(tools),
                    error: None,
                })
            }
            "tools/call" => {
                let params = request.params.clone().unwrap_or(json!({}));
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                let data = match tool_name {
                    "aegis_system_health" => json!({
                        "status": "HEALTHY",
                        "memory_mb": read_rss_mb(),
                        "mtls_enforced": true
                    }),
                    "aegis_ping" => json!({
                        "status": "pong",
                        "echo": args.get("echo").and_then(|e| e.as_str()).unwrap_or("hello")
                    }),
                    "aegis_sqlite_query" | "sqlite_query" => {
                        let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("SELECT * FROM customer_accounts LIMIT 5");
                        match self.sql_engine.execute(query).await {
                            Ok(res) => res.to_json_value(),
                            Err(e) => json!({ "status": "ERROR", "error": e.to_string() }),
                        }
                    }
                    "sqlite_schema" => {
                        let table = args.get("table").and_then(|t| t.as_str()).unwrap_or("customer_accounts");
                        match self.sql_engine.execute(&format!("PRAGMA table_info({table})")).await {
                            Ok(res) => res.to_json_value(),
                            Err(e) => json!({ "status": "ERROR", "error": e.to_string() }),
                        }
                    }
                    _ => json!({
                        "status": "unattached",
                        "execution_mode": "PREFLIGHT_GUARDRAIL_ONLY",
                        "upstream_status": "NOT_CONNECTED",
                        "message": format!("Guardrail ABAC & DLP verified for '{tool_name}'. No live upstream backend connected. Register a live backend in Backends tab to execute live transactions."),
                        "tool": tool_name,
                        "arguments": args
                    }),
                };

                Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req_id,
                    result: Some(json!({
                        "content": [{ "type": "text", "text": data.to_string() }],
                        "isError": data.get("status") == Some(&json!("ERROR")),
                        "data": data
                    })),
                    error: None,
                })
            }
            _ => Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req_id,
                result: Some(json!({ "status": "acknowledged" })),
                error: None,
            }),
        }
    }

    async fn stop(&self) -> AegisResult<()> {
        Ok(())
    }
}

fn read_rss_mb() -> f64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return (kb / 1024.0 * 100.0).round() / 100.0;
                    }
                }
            }
        }
    }
    0.0
}
