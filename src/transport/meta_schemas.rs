// SPDX-License-Identifier: MIT

use serde_json::{json, Value};

/// Catalog of built-in first-class meta-tools advertised over MCP wire
pub fn get_built_in_meta_tools() -> Vec<Value> {
    vec![
        json!({
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
        }),
        json!({
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
        }),
        json!({
            "name": "evaluate_data_egress",
            "description": "Preflight evaluation of data export/download against enterprise policy tiers (Developer, Hybrid, StrictAirgapped) and size thresholds",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "resource_id": { "type": "string", "description": "Identifier or filename of the resource (e.g. 'export_parquet.zip')" },
                    "resource_size_bytes": { "type": "integer", "description": "Size in bytes" },
                    "mime_type": { "type": "string", "description": "MIME type (e.g. 'application/zip')" },
                    "classification": { "type": "string", "description": "Data classification: 'RawRestricted', 'DerivedArtifact', or 'PublicResource'" }
                },
                "required": ["resource_id", "resource_size_bytes"]
            }
        }),
        json!({
            "name": "execute_in_situ_query",
            "description": "Execute high-performance analytical queries (DuckDB/Polars/Python) in-situ inside the secure server sandbox without egressing raw data",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "SQL or analytical query string" },
                    "resource_path": { "type": "string", "description": "Path to dataset within server sandbox" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "resolve_approval",
            "description": "Cryptographically resolve and resume a suspended Human-In-The-Loop approval ticket using HMAC signature",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "ticket_id": { "type": "string", "description": "The approval ticket ID" },
                    "signature": { "type": "string", "description": "HMAC-SHA256 signature token" },
                    "decision": { "type": "string", "description": "'approve' or 'deny'" },
                    "approver_id": { "type": "string", "description": "Identifier of the approving human" }
                },
                "required": ["ticket_id", "signature", "decision"]
            }
        }),
        json!({
            "name": "gateway_search_tools",
            "description": "Progressive tool discovery across active MCP backend servers",
            "inputSchema": { "type": "object", "properties": { "query": { "type": "string" } }, "required": ["query"] }
        }),
        json!({
            "name": "gateway_plan_tasks",
            "description": "Formulate and preflight multi-step execution plans across tools against zero-trust policy",
            "inputSchema": { "type": "object", "properties": { "plan_id": { "type": "string" }, "steps": { "type": "array" } }, "required": ["steps"] }
        }),
        json!({
            "name": "gateway_list_servers",
            "description": "List connected upstream backend MCP servers and operational status",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gateway_register_tools",
            "description": "Dynamically register or update tool definitions into gateway catalog",
            "inputSchema": { "type": "object", "properties": { "tools": { "type": "array" } }, "required": ["tools"] }
        }),
    ]
}
