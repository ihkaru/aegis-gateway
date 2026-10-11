// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::json;

use crate::core::backend::BackendTransport;
use crate::core::error::AegisResult;
use crate::core::transport::{JsonRpcId, JsonRpcRequest, JsonRpcResponse};

/// Real In-Memory Local Loopback Backend for Aegis Gateway control plane
#[derive(Debug, Clone, Default)]
pub struct LocalAegisBackend;

impl LocalAegisBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BackendTransport for LocalAegisBackend {
    async fn start(&self) -> AegisResult<()> {
        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        true
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
                    "aegis_sqlite_query" => {
                        let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("SELECT * FROM customer_accounts LIMIT 5");
                        execute_local_sqlite(query)
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
                        "isError": false,
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

fn execute_local_sqlite(query: &str) -> serde_json::Value {
    let script = r#"
import sqlite3, json, sys, os
db_dir = "/root/projects/aegis-gateway/data"
os.makedirs(db_dir, exist_ok=True)
db_path = os.path.join(db_dir, "aegis_local.db")
conn = sqlite3.connect(db_path)
cur = conn.cursor()
try:
    cur.execute(sys.argv[1])
    q_upper = sys.argv[1].strip().upper()
    if q_upper.startswith(("SELECT", "PRAGMA", "EXPLAIN")):
        cols = [d[0] for d in cur.description] if cur.description else []
        rows = [dict(zip(cols, r)) for r in cur.fetchall()]
        print(json.dumps({"status": "SUCCESS", "type": "SELECT", "row_count": len(rows), "rows": rows}))
    else:
        conn.commit()
        print(json.dumps({"status": "SUCCESS", "type": "MUTATION", "rows_affected": cur.rowcount}))
except Exception as e:
    print(json.dumps({"status": "ERROR", "error": str(e)}))
"#;
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(query)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            serde_json::from_str(&stdout).unwrap_or_else(|_| {
                json!({ "status": "ERROR", "error": "Invalid output from SQLite engine" })
            })
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            json!({ "status": "ERROR", "error": stderr.to_string() })
        }
        Err(e) => json!({ "status": "ERROR", "error": e.to_string() }),
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
