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
    let server = LiveHttpServer::new(handler, drain, "127.0.0.1", 39710).expect("server init");
    server.router()
}

#[tokio::test]
async fn test_phase59_wire_catalog_advertises_plan_executor_and_call_tool() {
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

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();

    // Verify Phase 59 high-accuracy meta-tools are advertised
    assert!(tool_names.contains(&"gateway_plan_tasks"), "gateway_plan_tasks must be advertised");
    assert!(tool_names.contains(&"gateway_execute_plan"), "gateway_execute_plan must be advertised");
    assert!(tool_names.contains(&"gateway_call_tool"), "gateway_call_tool must be advertised");
    assert!(tool_names.contains(&"gateway_search_tools"), "gateway_search_tools must be advertised");
}

#[tokio::test]
async fn test_phase59_plan_first_caching_and_autonomous_execution() {
    let app = setup_gateway_server();

    // Step 1: Formulate multi-step goal plan via gateway_plan_tasks
    let plan_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "gateway_plan_tasks",
            "arguments": {
                "goal": "Analisis data dengan Python lalu lakukan kalkulasi statistik"
            }
        }
    });

    let req1 = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(plan_req.to_string()))
        .unwrap();

    let resp1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    let bytes1 = axum::body::to_bytes(resp1.into_body(), usize::MAX).await.unwrap();
    let body1: serde_json::Value = serde_json::from_slice(&bytes1).unwrap();
    let result1 = &body1["result"];

    assert_eq!(result1["valid"], true, "Formulated plan must be valid");
    let plan_id = result1["plan_id"].as_str().expect("plan_id present");
    assert!(!plan_id.is_empty());

    // Step 2: Autonomously execute the cached plan via gateway_execute_plan
    let exec_req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "gateway_execute_plan",
            "arguments": {
                "plan_id": plan_id
            }
        }
    });

    let req2 = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(exec_req.to_string()))
        .unwrap();

    let resp2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);

    let bytes2 = axum::body::to_bytes(resp2.into_body(), usize::MAX).await.unwrap();
    let body2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    let structured = &body2["result"]["structured"];
    assert_eq!(structured["success"], true, "Autonomous DAG execution must succeed");
    assert_eq!(structured["plan_id"], plan_id);
    let completed = structured["completed_steps"].as_u64().unwrap();
    let total = structured["total_steps"].as_u64().unwrap();
    assert_eq!(completed, total, "All steps in plan must be completed");
    assert!(total >= 1, "Must have executed at least 1 step");
}

#[tokio::test]
async fn test_phase59_autonomous_plan_execution_with_inline_plan_object() {
    let app = setup_gateway_server();

    // Direct invocation with explicit plan structure (DAG dependency interpolation)
    let direct_plan_req = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "gateway_execute_plan",
            "arguments": {
                "plan": {
                    "plan_id": "inline_plan_001",
                    "title": "Inline Multi-Step Transformation",
                    "steps": [
                        {
                            "id": "step_1",
                            "tool": "execute_code",
                            "arguments": {
                                "code": "print('step 1 payload generated')"
                            },
                            "depends_on": []
                        },
                        {
                            "id": "step_2",
                            "tool": "execute_code",
                            "arguments": {
                                "code": "print('step 2 execution complete')"
                            },
                            "depends_on": ["step_1"]
                        }
                    ]
                }
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(direct_plan_req.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let structured = &body["result"]["structured"];

    assert_eq!(structured["success"], true);
    assert_eq!(structured["completed_steps"], 2);
    assert_eq!(structured["total_steps"], 2);
}

#[tokio::test]
async fn test_phase59_dynamic_tool_invoker_gateway_call_tool() {
    let app = setup_gateway_server();

    // Call sandbox dynamically via gateway_call_tool
    let call_req = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "gateway_call_tool",
            "arguments": {
                "name": "execute_code",
                "arguments": {
                    "code": "print('DYNAMIC_INVOKER_SUCCESS_SENTINEL')"
                }
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(call_req.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let content_text = body["result"]["content"][0]["text"].as_str().unwrap();

    assert!(
        content_text.contains("DYNAMIC_INVOKER_SUCCESS_SENTINEL"),
        "Dynamic tool invoker must execute target tool and return stdout"
    );
}
