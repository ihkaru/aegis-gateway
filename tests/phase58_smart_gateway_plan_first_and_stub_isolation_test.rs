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

fn setup_clean_gateway_server() -> axum::Router {
    let gateway = Arc::new(PropylonGateway::default());
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let drain = Arc::new(DrainCoordinator::new());
    let server = LiveHttpServer::new(handler, drain, "127.0.0.1", 39700).expect("server init");
    server.router()
}

#[tokio::test]
async fn test_phase58_clean_wire_catalog_has_zero_synthetic_stubs() {
    let app = setup_clean_gateway_server();

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

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();

    // 1. Zero synthetic stubs must leak into default wire catalog
    assert!(!tool_names.contains(&"stripe_create_customer"), "Stripe stub must not be advertised");
    assert!(!tool_names.contains(&"stripe_charge_card"), "Stripe charge stub must not be advertised");
    assert!(!tool_names.contains(&"query_postgresql_ledger"), "Postgres stub must not be advertised");
    assert!(!tool_names.contains(&"destructive_drop_table"), "Drop table stub must not be advertised");
    assert!(!tool_names.contains(&"tax_calculate_vat"), "Tax stub must not be advertised");
    assert!(!tool_names.contains(&"dispatch_slack_alert"), "Slack stub must not be advertised");
    assert!(!tool_names.contains(&"sendgrid_send_invoice_email"), "Sendgrid stub must not be advertised");

    // 2. Redundancy check: gateway_execute_code duplicate alias must be removed from advertised list
    assert!(!tool_names.contains(&"gateway_execute_code"), "Redundant duplicate alias must be omitted");
    assert!(tool_names.contains(&"execute_code"), "Core execute_code must be present");

    // 3. Plan-First gate must be present as first advertised meta-tool
    assert_eq!(tool_names[0], "gateway_plan_tasks", "gateway_plan_tasks must be the prime plan-first tool");
    assert!(tool_names.contains(&"gateway_search_tools"), "Search tools must be present");
    assert!(tool_names.contains(&"execute_in_situ_query"), "In-situ query must be present");
    assert!(tool_names.contains(&"resolve_approval"), "Approval gate must be present");
}

#[tokio::test]
async fn test_phase58_plan_first_natural_language_goal_decomposition() {
    let app = setup_clean_gateway_server();

    // Natural language compound goal in Indonesian
    let plan_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "gateway_plan_tasks",
            "arguments": {
                "goal": "Analisis data dari tabel transaksi kemudian jalankan analisis statistik dan simpan output"
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(plan_req.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let result = &body_json["result"];
    assert!(result["valid"].as_bool().unwrap_or(false), "Plan formulation must be valid");
    assert!(result["plan_id"].is_string(), "Plan ID must be generated");
    assert!(result["step_count"].as_u64().unwrap_or(0) >= 1, "Plan must generate sequential steps");

    let steps = result["plan"]["steps"].as_array().expect("steps array");
    assert!(!steps.is_empty(), "Steps must be populated from goal segmentation");
}

#[tokio::test]
async fn test_phase58_backward_compatibility_for_legacy_alias() {
    let app = setup_clean_gateway_server();

    // Calling legacy alias gateway_execute_code must still succeed transparently
    let exec_req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "gateway_execute_code",
            "arguments": {
                "code": "print('hello from legacy alias')",
                "language": "python"
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(exec_req.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body_json["result"].is_object(), "Execution via legacy alias must succeed");
}

#[tokio::test]
async fn test_phase58_progressive_tool_discovery_on_demand() {
    let app = setup_clean_gateway_server();

    let search_req = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "gateway_search_tools",
            "arguments": {
                "query": "code execution"
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(search_req.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body_json["result"]["tools"].is_array(), "Search must return matching tools array");
}
