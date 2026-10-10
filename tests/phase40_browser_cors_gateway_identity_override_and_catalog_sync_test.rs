// SPDX-License-Identifier: MIT

//! Phase 40 Integration Test Suite: Browser CORS, Gateway Identity Override & Catalog Sync
//! Resolves docker/mcp-gateway#544, docker/mcp-gateway#583, docker/mcp-gateway#561

#![deny(unsafe_code)]

use std::sync::Arc;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use aegis_gateway::core::identity::GatewayServerIdentity;
use aegis_gateway::core::transport::WireProtocolHandler;
use aegis_gateway::core::types::ToolDefinition;
use aegis_gateway::discovery::{
    AegisCatalogSyncSentinel, CatalogSearchIndex, CatalogSyncOutcome,
};
use aegis_gateway::state::DrainCoordinator;
use aegis_gateway::transport::{
    AegisCorsPolicy, LiveHttpServer, McpProtocolHandler,
};
use aegis_gateway::AegisGateway;

#[tokio::test]
async fn test_phase40_browser_cors_preflight_and_origin_whitelist() {
    let gateway = Arc::new(AegisGateway::default());
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let drain = Arc::new(DrainCoordinator::new());

    // Test default policy with wildcard origin
    let server = LiveHttpServer::new(handler.clone(), drain.clone(), "127.0.0.1", 39401)
        .expect("Failed to initialize LiveHttpServer");
    let app = server.router();

    // 1. Send OPTIONS preflight from a browser client
    let preflight_req = Request::builder()
        .method("OPTIONS")
        .uri("/mcp")
        .header("origin", "http://localhost:5173")
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "content-type, mcp-protocol-version")
        .body(Body::empty())
        .expect("Failed to build request");

    let preflight_resp = app.clone().oneshot(preflight_req).await.unwrap();
    assert_eq!(preflight_resp.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        preflight_resp.headers().get("access-control-allow-origin").unwrap(),
        "*"
    );
    assert!(
        preflight_resp
            .headers()
            .get("access-control-allow-methods")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("POST")
    );

    // 2. Send POST /mcp and verify CORS headers in response
    let init_payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "browser-client", "version": "1.0.0" }
        }
    })
    .to_string();

    let post_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("origin", "http://localhost:5173")
        .body(Body::from(init_payload))
        .expect("Failed to build request");

    let post_resp = app.clone().oneshot(post_req).await.unwrap();
    assert_eq!(post_resp.status(), StatusCode::OK);
    assert_eq!(
        post_resp.headers().get("access-control-allow-origin").unwrap(),
        "*"
    );

    // 3. Test strict CORS policy with restricted whitelist
    let strict_cors = Arc::new(
        AegisCorsPolicy::new(vec!["https://trusted-dashboard.internal".to_string()])
            .with_allow_credentials(true),
    );
    let strict_server = LiveHttpServer::with_cors(
        handler,
        drain,
        strict_cors,
        "127.0.0.1",
        39402,
    )
    .expect("Failed to create strict server");
    let strict_app = strict_server.router();

    // Permitted origin
    let allowed_req = Request::builder()
        .method("OPTIONS")
        .uri("/mcp")
        .header("origin", "https://trusted-dashboard.internal")
        .body(Body::empty())
        .unwrap();
    let allowed_resp = strict_app.clone().oneshot(allowed_req).await.unwrap();
    assert_eq!(allowed_resp.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        allowed_resp.headers().get("access-control-allow-origin").unwrap(),
        "https://trusted-dashboard.internal"
    );
    assert_eq!(
        allowed_resp.headers().get("access-control-allow-credentials").unwrap(),
        "true"
    );

    // Disallowed origin should be rejected with 403 Forbidden
    let blocked_req = Request::builder()
        .method("OPTIONS")
        .uri("/mcp")
        .header("origin", "https://malicious-site.com")
        .body(Body::empty())
        .unwrap();
    let blocked_resp = strict_app.oneshot(blocked_req).await.unwrap();
    assert_eq!(blocked_resp.status(), StatusCode::FORBIDDEN);
    assert!(blocked_resp.headers().contains_key("x-aegis-cors-rejection"));
}

#[tokio::test]
async fn test_phase40_server_info_identity_override() {
    let gateway = Arc::new(AegisGateway::default());

    // 1. Initialize with custom enterprise identity
    let custom_identity = GatewayServerIdentity::new("enterprise-corp-gateway", "2.40.0")
        .with_protocol_version("2024-11-05");
    let handler = McpProtocolHandler::with_identity(gateway, None, custom_identity);

    let init_rpc = json!({
        "jsonrpc": "2.0",
        "id": 101,
        "method": "initialize",
        "params": {}
    })
    .to_string();

    let resp_opt = handler.handle_message(&init_rpc).await.unwrap();
    assert!(resp_opt.is_some());
    let resp_val: serde_json::Value = serde_json::from_str(&resp_opt.unwrap()).unwrap();
    assert_eq!(resp_val["result"]["serverInfo"]["name"], "enterprise-corp-gateway");
    assert_eq!(resp_val["result"]["serverInfo"]["version"], "2.40.0");
    assert_eq!(resp_val["result"]["protocolVersion"], "2024-11-05");

    // 2. Dynamic runtime update without server restart
    handler
        .set_server_identity(GatewayServerIdentity::new("sovereign-ai-nexus", "3.0.0-rc1"))
        .await;

    let init_rpc_updated = json!({
        "jsonrpc": "2.0",
        "id": 102,
        "method": "initialize",
        "params": {}
    })
    .to_string();

    let resp_updated = handler.handle_message(&init_rpc_updated).await.unwrap();
    let resp_up_val: serde_json::Value = serde_json::from_str(&resp_updated.unwrap()).unwrap();
    assert_eq!(resp_up_val["result"]["serverInfo"]["name"], "sovereign-ai-nexus");
    assert_eq!(resp_up_val["result"]["serverInfo"]["version"], "3.0.0-rc1");
}

#[tokio::test]
async fn test_phase40_catalog_zero_tool_drop_protection() {
    let mut index = CatalogSearchIndex::new();
    let sentinel = AegisCatalogSyncSentinel::default();

    // 1. Initial healthy tool registration (3 tools)
    let initial_tools = vec![
        ToolDefinition {
            name: "query_database".to_string(),
            description: "Run SQL select queries".to_string(),
            server: "pg-db-cluster".to_string(),
            required_params: vec!["sql".to_string()],
            when_to_use: "querying records".to_string(),
            tags: vec!["db".to_string(), "sql".to_string()],
            input_schema: json!({}),
        },
        ToolDefinition {
            name: "list_tables".to_string(),
            description: "List all database schema tables".to_string(),
            server: "pg-db-cluster".to_string(),
            required_params: vec![],
            when_to_use: "schema discovery".to_string(),
            tags: vec!["db".to_string()],
            input_schema: json!({}),
        },
        ToolDefinition {
            name: "describe_schema".to_string(),
            description: "Describe column definitions".to_string(),
            server: "pg-db-cluster".to_string(),
            required_params: vec!["table".to_string()],
            when_to_use: "schema structure".to_string(),
            tags: vec!["schema".to_string()],
            input_schema: json!({}),
        },
    ];

    let outcome1 = index.safe_index_tools_for_backend(
        "pg-db-cluster",
        initial_tools,
        &sentinel,
        false,
    );
    assert_eq!(outcome1, CatalogSyncOutcome::Accepted {
        server: "pg-db-cluster".to_string(),
        tool_count: 3,
    });
    assert_eq!(index.backend_tool_count("pg-db-cluster"), 3);

    // 2. Erroneous upstream sync with 0 tools (transient error / drop)
    let outcome2 = index.safe_index_tools_for_backend(
        "pg-db-cluster",
        Vec::new(),
        &sentinel,
        false,
    );

    // Sentinel must refuse the zero-tool drop and preserve all 3 existing tools!
    match outcome2 {
        CatalogSyncOutcome::RefusedEmptySync {
            server,
            preserved_tool_count,
            reason,
        } => {
            assert_eq!(server, "pg-db-cluster");
            assert_eq!(preserved_tool_count, 3);
            assert!(reason.contains("refused zero-tool drop"));
        }
        _ => panic!("Expected RefusedEmptySync but got {:?}", outcome2),
    }

    assert_eq!(
        index.backend_tool_count("pg-db-cluster"),
        3,
        "Tools must not be wiped out on empty sync"
    );

    // 3. Legitimate explicit administrative purge
    let outcome3 = index.safe_index_tools_for_backend(
        "pg-db-cluster",
        Vec::new(),
        &sentinel,
        true,
    );
    assert_eq!(outcome3, CatalogSyncOutcome::ExplicitPurge {
        server: "pg-db-cluster".to_string(),
    });
    assert_eq!(
        index.backend_tool_count("pg-db-cluster"),
        0,
        "Tools must be purged only when explicitly requested"
    );
}
