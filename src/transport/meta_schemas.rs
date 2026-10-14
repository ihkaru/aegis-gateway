// SPDX-License-Identifier: MIT

use serde_json::{json, Value};
use crate::core::types::ToolDefinition;

/// Catalog of built-in first-class meta-tools advertised over MCP wire.
/// Structured following the Plan-First Doctrine and Progressive Tool Discovery.
pub fn get_built_in_meta_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "gateway_plan_tasks",
            "description": "Smart Plan-First Gate: Formulate, decompose, and preflight multi-step execution plans before taking action. Pass 'goal' in natural language (Indonesian or English) to decompose into ordered execution steps with ABAC security validation and risk scoring.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "goal": { "type": "string", "description": "High-level user objective or goal in natural language (Indonesian or English) to decompose and preflight" },
                    "plan_id": { "type": "string", "description": "Optional custom plan identifier" },
                    "steps": { "type": "array", "description": "Optional explicit plan steps if pre-formulated" }
                }
            }
        }),
        json!({
            "name": "gateway_search_tools",
            "description": "Progressive Tool Discovery: Search and find specific tools on-demand across connected backend services using BM25 and semantic matching. Reduces context bloat by looking up tools only when needed.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search keyword or capability requirement" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "execute_code",
            "description": "Hermetic Code Sandbox: Execute context-agnostic Python/Bash/Node code in an isolated sandbox with zero-knowledge credential broker and egress firewall.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "The script source code to execute" },
                    "language": { "type": "string", "description": "Execution language: python (default), bash, javascript" },
                    "services": { "type": "array", "items": { "type": "string" }, "description": "Services requiring brokered credentials (e.g. ['google', 'github', 'aws'])" },
                    "timeout_secs": { "type": "integer", "description": "Maximum execution time in seconds (default 30)" },
                    "env_vars": { "type": "object", "description": "Optional environment variables" },
                    "plan_id": { "type": "string", "description": "Optional plan ID from gateway_plan_tasks for correlation" }
                },
                "required": ["code"]
            }
        }),
        json!({
            "name": "execute_in_situ_query",
            "description": "In-situ Analytical Query: Execute high-performance analytical queries (DuckDB/Polars/Python) inside the secure server sandbox without egressing raw data.",
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
            "description": "Cryptographically resolve and resume a suspended Human-In-The-Loop approval ticket using HMAC signature.",
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
            "name": "evaluate_data_egress",
            "description": "Preflight evaluation of data export/download against enterprise policy tiers (Developer, Hybrid, StrictAirgapped) and size thresholds.",
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
            "name": "gateway_list_servers",
            "description": "List connected upstream backend MCP servers and operational status.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gateway_register_tools",
            "description": "Dynamically register or update tool definitions into gateway catalog (governed by GitOps immutability policy).",
            "inputSchema": { "type": "object", "properties": { "tools": { "type": "array" } }, "required": ["tools"] }
        }),
    ]
}

/// Provides ToolDefinition structures for native meta-tools so GoalPlanner and HybridSearchEngine
/// can synthesize plans and discover tools even before any external backends are registered.
pub fn get_native_meta_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "execute_code".to_string(),
            server: "gateway".to_string(),
            description: "Execute Python/Bash/Node code in hermetic sandbox with egress firewall".to_string(),
            input_schema: json!({ "type": "object", "properties": { "code": { "type": "string" } }, "required": ["code"] }),
            required_params: vec!["code".to_string()],
            when_to_use: "When running scripts, calculations, transformations, or data processing".to_string(),
            tags: vec!["code".to_string(), "python".to_string(), "bash".to_string(), "script".to_string(), "eksekusi".to_string(), "analisis".to_string(), "simpan".to_string()],
        },
        ToolDefinition {
            name: "execute_in_situ_query".to_string(),
            server: "gateway".to_string(),
            description: "Execute high-performance analytical queries in-situ inside secure sandbox".to_string(),
            input_schema: json!({ "type": "object", "properties": { "query": { "type": "string" } }, "required": ["query"] }),
            required_params: vec!["query".to_string()],
            when_to_use: "When analyzing internal datasets or SQL tables without exporting raw data".to_string(),
            tags: vec!["query".to_string(), "sql".to_string(), "duckdb".to_string(), "data".to_string(), "tabel".to_string(), "transaksi".to_string(), "analisis".to_string()],
        },
        ToolDefinition {
            name: "resolve_approval".to_string(),
            server: "gateway".to_string(),
            description: "Resolve and resume a suspended Human-In-The-Loop approval ticket".to_string(),
            input_schema: json!({ "type": "object", "properties": { "ticket_id": { "type": "string" } }, "required": ["ticket_id"] }),
            required_params: vec!["ticket_id".to_string()],
            when_to_use: "When resolving pending human authorization".to_string(),
            tags: vec!["approval".to_string(), "persetujuan".to_string(), "izin".to_string()],
        },
        ToolDefinition {
            name: "gateway_search_tools".to_string(),
            server: "gateway".to_string(),
            description: "Search and discover available tools across backends on demand".to_string(),
            input_schema: json!({ "type": "object", "properties": { "query": { "type": "string" } }, "required": ["query"] }),
            required_params: vec!["query".to_string()],
            when_to_use: "When looking for capabilities or tools".to_string(),
            tags: vec!["search".to_string(), "cari".to_string(), "tool".to_string()],
        },
    ]
}
