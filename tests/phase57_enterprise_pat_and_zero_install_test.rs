// SPDX-License-Identifier: MIT

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

use propylon::control::auth_api::AuthApiDispatcher;
use propylon::core::transport::WireProtocolHandler;
use propylon::policy::pat_service::{get_shared_pat_service, CreatePatRequest, PatService};
use propylon::state::DrainCoordinator;
use propylon::transport::{LiveHttpServer, McpProtocolHandler};
use propylon::PropylonGateway;

fn setup_test_server() -> axum::Router {
    let gateway = Arc::new(PropylonGateway::default());
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let drain = Arc::new(DrainCoordinator::new());
    let server = LiveHttpServer::new(handler, drain, "127.0.0.1", 39650).expect("server init");
    server.router()
}

#[tokio::test]
async fn test_pat_creation_and_cryptographic_format() {
    let pat_service = PatService::new();
    let req = CreatePatRequest {
        name: "Test Claude Code PAT".to_string(),
        ttl_days: Some(30),
        max_idle_days: Some(14),
        scopes: Some(vec!["tools:execute:safe".to_string()]),
    };

    let resp = pat_service.create_pat("developer@bps.go.id", req).expect("PAT creation failed");
    assert!(resp.token.starts_with("prp_pat_"), "Token must have prp_pat_ prefix");
    assert_eq!(resp.name, "Test Claude Code PAT");
    assert_eq!(resp.scopes, vec!["tools:execute:safe"]);

    // Validate token immediately
    let val = pat_service.validate_pat(&resp.token, None).expect("Token validation failed");
    assert_eq!(val.subject, "developer@bps.go.id");
    assert_eq!(val.pat_id, resp.id);
}

#[tokio::test]
async fn test_pat_least_privilege_tool_scoping() {
    let pat_service = PatService::new();
    let req = CreatePatRequest {
        name: "Scoped PAT".to_string(),
        ttl_days: Some(30),
        max_idle_days: Some(14),
        scopes: Some(vec!["tools:execute:safe".to_string()]),
    };

    let resp = pat_service.create_pat("analyst@bps.go.id", req).expect("PAT creation failed");

    // Safe tool must succeed
    let safe_check = pat_service.validate_pat(&resp.token, Some("query_postgresql_ledger"));
    assert!(safe_check.is_ok(), "Safe tool must be permitted");

    // Destructive drop/delete tool must be blocked
    let drop_check = pat_service.validate_pat(&resp.token, Some("destructive_drop_table"));
    assert!(drop_check.is_err(), "Destructive tool must be rejected");
    let err_msg = drop_check.err().unwrap().to_string();
    assert!(err_msg.contains("Destructive operation"), "Error must cite destructive operation");

    let delete_check = pat_service.validate_pat(&resp.token, Some("delete_storage_object"));
    assert!(delete_check.is_err(), "Destructive delete tool must be rejected");
}

#[tokio::test]
async fn test_pat_remote_revocation_kill_switch() {
    let pat_service = PatService::new();
    let req = CreatePatRequest {
        name: "Revocable Token".to_string(),
        ttl_days: Some(30),
        max_idle_days: Some(14),
        scopes: Some(vec!["tools:execute:safe".to_string()]),
    };

    let resp = pat_service.create_pat("user@bps.go.id", req).expect("PAT creation");
    assert!(pat_service.validate_pat(&resp.token, None).is_ok());

    // Revoke token
    pat_service.revoke_pat("user@bps.go.id", &resp.id).expect("Revocation failed");

    // Subsequent validation must fail
    let val = pat_service.validate_pat(&resp.token, None);
    assert!(val.is_err(), "Revoked token must fail validation");
    let err_str = val.err().unwrap().to_string();
    assert!(err_str.contains("revoked"), "Error must state token is revoked");
}

#[tokio::test]
async fn test_pat_sliding_window_inactivity_timeout() {
    let pat_service = PatService::new();
    let req = CreatePatRequest {
        name: "Idle Token".to_string(),
        ttl_days: Some(30),
        max_idle_days: Some(1), // 1 day max idle
        scopes: Some(vec!["tools:execute:safe".to_string()]),
    };

    let resp = pat_service.create_pat("inactive@bps.go.id", req).expect("PAT creation");

    // Artificially modify last_used_at to 2 days ago (exceeding 1 day max idle)
    let two_days_ago = chrono::Utc::now().timestamp() - (2 * 86400);
    pat_service.set_last_used_at_for_test(&resp.id, two_days_ago);

    // Subsequent validation must fail due to sliding window inactivity
    let val_result = pat_service.validate_pat(&resp.token, None);
    assert!(val_result.is_err(), "Inactive token must be rejected");
    let err_str = val_result.err().unwrap().to_string();
    assert!(err_str.contains("inactivity timeout"), "Error must cite inactivity timeout: {}", err_str);
}

#[tokio::test]
async fn test_pat_rest_api_dispatcher_endpoints() {
    // 1. POST /api/v1/auth/pat (Create)
    let create_body = json!({
        "name": "Cursor IDE Remote Token",
        "ttl_days": 60,
        "max_idle_days": 14,
        "scopes": ["tools:execute:safe", "execute_code"]
    });

    let (status, body, _) = AuthApiDispatcher::dispatch(
        "POST",
        "/api/v1/auth/pat",
        &serde_json::to_vec(&create_body).unwrap(),
    ).await;
    assert_eq!(status, 201, "PAT creation must return 201 Created");

    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let pat_id = created["id"].as_str().expect("PAT ID missing").to_string();
    let token = created["token"].as_str().expect("Token missing").to_string();
    assert!(token.starts_with("prp_pat_"));

    // 2. GET /api/v1/auth/pat (List)
    let (status, body, _) = AuthApiDispatcher::dispatch(
        "GET",
        "/api/v1/auth/pat",
        b"",
    ).await;
    assert_eq!(status, 200, "PAT listing must return 200 OK");
    let list: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(list.iter().any(|p| p["id"] == pat_id), "Newly created PAT must be listed");

    // 3. POST /api/v1/auth/pat/revoke (Revoke)
    let revoke_body = json!({ "id": pat_id });
    let (status, body, _) = AuthApiDispatcher::dispatch(
        "POST",
        "/api/v1/auth/pat/revoke",
        &serde_json::to_vec(&revoke_body).unwrap(),
    ).await;
    assert_eq!(status, 200, "PAT revocation must return 200 OK");
    let revoked: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(revoked["status"], "revoked");

    // 4. Validate revoked token is rejected via PAT service
    let val_result = get_shared_pat_service().validate_pat(&token, None);
    assert!(val_result.is_err(), "Revoked token must be rejected");
}

#[tokio::test]
async fn test_zero_install_mcp_post_transport_with_pat() {
    let app = setup_test_server();

    // Create a valid PAT
    let create_resp = get_shared_pat_service().create_pat(
        "ciso_auditor@bps.go.id",
        CreatePatRequest {
            name: "Zero Install Claude Token".to_string(),
            ttl_days: Some(30),
            max_idle_days: Some(14),
            scopes: Some(vec!["tools:execute:safe".to_string()]),
        },
    ).expect("create PAT");

    // 1. Direct MCP JSON-RPC tools/call for safe tool with PAT
    let safe_req_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "gateway_search_tools",
            "arguments": { "query": "auth" }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", create_resp.token))
        .body(Body::from(safe_req_body.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Direct MCP JSON-RPC tools/call for destructive tool with PAT (Must be blocked)
    let destructive_req_body = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "destructive_drop_table",
            "arguments": { "table": "users" }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", create_resp.token))
        .body(Body::from(destructive_req_body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body_json["error"].is_object(), "Destructive tool must return error");
    assert_eq!(body_json["error"]["code"], -32003);
    assert!(body_json["error"]["message"].as_str().unwrap().contains("Access Denied"));
}
