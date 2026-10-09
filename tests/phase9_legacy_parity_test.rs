// SPDX-License-Identifier: MIT

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tower::ServiceExt;

use aegis_gateway::backend::HermeticSubprocessBackend;
use aegis_gateway::cli::args::{AddArgs, ListArgs, RemoveArgs};
use aegis_gateway::cli::commands::{run_add, run_list, run_remove};
use aegis_gateway::core::types::{DisclosureTier, ToolDefinition};
use aegis_gateway::discovery::ProgressiveProjector;
use aegis_gateway::state::DrainCoordinator;
use aegis_gateway::transport::command_split::{split_command_unix, split_command_windows};
use aegis_gateway::transport::{LiveHttpServer, McpProtocolHandler};
use aegis_gateway::AegisGateway;

#[tokio::test]
async fn test_issue_2317_search_tool_by_backend_name() {
    let projector = ProgressiveProjector::new();

    let tools = vec![
        ToolDefinition {
            name: "query_sql".to_string(),
            server: "postgres".to_string(),
            description: "Run arbitrary read-only queries against tables".to_string(),
            input_schema: json!({ "type": "object" }),
            required_params: vec!["sql".to_string()],
            when_to_use: "Database inspection".to_string(),
            tags: vec!["postgres".to_string(), "rdbms".to_string()],
        },
        ToolDefinition {
            name: "find_symbol".to_string(),
            server: "codesearch".to_string(),
            description: "Locate classes and function symbols".to_string(),
            input_schema: json!({ "type": "object" }),
            required_params: vec!["symbol".to_string()],
            when_to_use: "Source code exploration".to_string(),
            tags: vec!["codesearch".to_string()],
        },
    ];

    // Query specifically for the backend name "postgres" (not in tool name or description)
    let pg_results = projector.filter_by_context(&tools, "postgres", DisclosureTier::L0, 5);
    assert!(!pg_results.is_empty(), "Searching by backend name 'postgres' must match");
    assert_eq!(pg_results[0].name, "query_sql");
    assert_eq!(pg_results[0].server, "postgres");

    // Query specifically for "codesearch"
    let cs_results = projector.filter_by_context(&tools, "codesearch", DisclosureTier::L0, 5);
    assert!(!cs_results.is_empty(), "Searching by backend name 'codesearch' must match");
    assert_eq!(cs_results[0].name, "find_symbol");
    assert_eq!(cs_results[0].server, "codesearch");
}

#[tokio::test]
async fn test_issue_622_per_backend_package_manager_cache_isolation() {
    // 1. Two backends running the same npx package command must not share a cache dir
    let cmd_a = HermeticSubprocessBackend::build_command_for_backend(
        "cal-personal",
        "npx -y caldav-mcp",
        &[],
        &HashMap::new(),
    );
    let cmd_b = HermeticSubprocessBackend::build_command_for_backend(
        "cal-work",
        "npx -y caldav-mcp",
        &[],
        &HashMap::new(),
    );

    let str_a = format!("{cmd_a:?}");
    let str_b = format!("{cmd_b:?}");
    assert!(str_a.contains("npm_config_cache"), "npx command must have npm_config_cache injected");
    assert!(str_b.contains("npm_config_cache"), "npx command must have npm_config_cache injected");
    assert_ne!(str_a, str_b, "Distinct backends must have distinct cache paths");

    // 2. An operator-provided npm_config_cache must be preserved
    let mut custom_env = HashMap::new();
    custom_env.insert("npm_config_cache".to_string(), "/custom/cache/dir".to_string());
    let cmd_custom = HermeticSubprocessBackend::build_command_for_backend(
        "custom-srv",
        "npx -y pkg",
        &[],
        &custom_env,
    );
    let str_custom = format!("{cmd_custom:?}");
    assert!(str_custom.contains("/custom/cache/dir"), "Operator-set cache must not be overwritten");

    // 3. Python uvx commands must get UV_CACHE_DIR isolation
    let cmd_uv = HermeticSubprocessBackend::build_command_for_backend(
        "meili",
        "uvx meilisearch-mcp",
        &[],
        &HashMap::new(),
    );
    let str_uv = format!("{cmd_uv:?}");
    assert!(str_uv.contains("UV_CACHE_DIR"), "uvx command must receive UV_CACHE_DIR");

    // 4. Directory traversal attempt in backend name must be sanitized
    let cmd_hostile = HermeticSubprocessBackend::build_command_for_backend(
        "../../etc/passwd",
        "npx -y pkg",
        &[],
        &HashMap::new(),
    );
    let str_hostile = format!("{cmd_hostile:?}");
    assert!(!str_hostile.contains("../"), "Hostile path traversal in backend name must be sanitized");
}

#[test]
fn test_issue_523_windows_and_unix_command_split() {
    // Windows CommandLineToArgvW rules preserve backslashes as path separators
    let win_cmd = r#"C:\Windows\System32\py.exe -3.11 -m markitdown_mcp"#;
    let win_parts = split_command_windows(win_cmd).expect("Windows split failed");
    assert_eq!(
        win_parts,
        vec![
            r"C:\Windows\System32\py.exe".to_string(),
            "-3.11".to_string(),
            "-m".to_string(),
            "markitdown_mcp".to_string(),
        ]
    );

    // Windows quoted path with spaces
    let win_quoted = r#""C:\Program Files\Node\node.exe" index.js"#;
    let win_q_parts = split_command_windows(win_quoted).expect("Windows quoted split failed");
    assert_eq!(
        win_q_parts,
        vec![r"C:\Program Files\Node\node.exe".to_string(), "index.js".to_string()]
    );

    // Unix POSIX shell rules
    let unix_cmd = "/usr/local/bin/node index.js --option 'value with spaces'";
    let unix_parts = split_command_unix(unix_cmd).expect("Unix split failed");
    assert_eq!(
        unix_parts,
        vec![
            "/usr/local/bin/node".to_string(),
            "index.js".to_string(),
            "--option".to_string(),
            "value with spaces".to_string(),
        ]
    );
}

#[tokio::test]
async fn test_issue_540_mcp_protocol_version_header_validation() {
    let gateway = Arc::new(AegisGateway::default());
    let handler = Arc::new(McpProtocolHandler::new(Arc::clone(&gateway)));
    let drain = Arc::new(DrainCoordinator::new());
    let server = LiveHttpServer::new(handler, drain, "127.0.0.1", 39999).unwrap();
    let app = server.router();

    // 1. Unsupported protocol version header returns HTTP 400 Bad Request with code -32022
    let bad_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "1999-01-01")
        .body(Body::from(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping"
        }).to_string()))
        .unwrap();

    let resp = app.clone().oneshot(bad_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["error"]["code"], -32022);
    assert!(val["error"]["message"].as_str().unwrap().contains("Unsupported MCP protocol version"));

    // 2. Supported protocol version header returns HTTP 200 OK
    let ok_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "2024-11-05")
        .body(Body::from(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "ping"
        }).to_string()))
        .unwrap();

    let resp_ok = app.clone().oneshot(ok_req).await.unwrap();
    assert_eq!(resp_ok.status(), StatusCode::OK);

    // 3. Absent protocol version header also returns HTTP 200 OK (stateless legacy fallback)
    let absent_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "ping"
        }).to_string()))
        .unwrap();

    let resp_absent = app.oneshot(absent_req).await.unwrap();
    assert_eq!(resp_absent.status(), StatusCode::OK);
}

#[test]
fn test_cli_add_remove_list_dynamic_topology_lifecycle() {
    let tmp = NamedTempFile::new().unwrap();
    let tmp_path = tmp.path().to_string_lossy().to_string();

    // 1. Add server "postgres-local"
    let add_res = run_add(AddArgs {
        name: "postgres-local".to_string(),
        command: Some("npx".to_string()),
        args: vec!["-y".to_string(), "@modelcontextprotocol/server-postgres".to_string()],
        url: None,
        env: vec!["DATABASE_URL=postgres://localhost/test".to_string()],
        config: tmp_path.clone(),
    });
    assert!(add_res.is_ok(), "run_add should succeed");

    // 2. Add second remote server "github-remote"
    let add_remote_res = run_add(AddArgs {
        name: "github-remote".to_string(),
        command: None,
        args: vec![],
        url: Some("http://localhost:8080/mcp".to_string()),
        env: vec![],
        config: tmp_path.clone(),
    });
    assert!(add_remote_res.is_ok(), "run_add remote should succeed");

    // 3. Inspect listing via run_list
    let list_res = run_list(ListArgs {
        config: tmp_path.clone(),
        json: true,
    });
    assert!(list_res.is_ok(), "run_list should succeed");

    // 4. Remove "postgres-local"
    let remove_res = run_remove(RemoveArgs {
        name: "postgres-local".to_string(),
        config: tmp_path.clone(),
    });
    assert!(remove_res.is_ok(), "run_remove should succeed");

    // 5. Verify removal reflected in configuration
    let content = std::fs::read_to_string(&tmp_path).unwrap();
    assert!(!content.contains("postgres-local"), "postgres-local must be removed");
    assert!(content.contains("github-remote"), "github-remote must still exist");
}
