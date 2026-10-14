// SPDX-License-Identifier: MIT

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

use propylon::core::transport::WireProtocolHandler;
use propylon::state::DrainCoordinator;
use propylon::transport::{LiveHttpServer, McpProtocolHandler};
use propylon::PropylonGateway;

fn setup_gateway_server() -> axum::Router {
    let gateway = Arc::new(PropylonGateway::default());
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let drain = Arc::new(DrainCoordinator::new());
    let server = LiveHttpServer::new(handler, drain, "127.0.0.1", 39711).expect("server init");
    server.router()
}

#[tokio::test]
async fn test_phase60_catalog_advertises_gateway_install_tool_and_mise_guidance() {
    let app = setup_gateway_server();

    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(list_req.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let tools = body_json["result"]["tools"].as_array().expect("tools array");

    let install_tool = tools
        .iter()
        .find(|t| t["name"].as_str() == Some("gateway_install_tool"))
        .expect("gateway_install_tool must be advertised");

    assert!(
        install_tool["description"].as_str().unwrap().contains("mise"),
        "gateway_install_tool description must reference mise user-space management"
    );

    let execute_code_tool = tools
        .iter()
        .find(|t| t["name"].as_str() == Some("execute_code"))
        .expect("execute_code must be advertised");

    let exec_desc = execute_code_tool["description"].as_str().unwrap();
    assert!(
        exec_desc.contains("mise"),
        "execute_code description must inform clients about mise tool persistence"
    );
    assert!(
        exec_desc.to_lowercase().contains("infisical"),
        "execute_code description must mention Infisical vault brokering"
    );
}

#[tokio::test]
async fn test_phase60_install_tool_missing_params_rejection() {
    let app = setup_gateway_server();

    let install_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "gateway_install_tool",
            "arguments": {}
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(install_req.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        body_json["error"].is_object(),
        "Missing 'tool' parameter must return JSON-RPC error"
    );
}
