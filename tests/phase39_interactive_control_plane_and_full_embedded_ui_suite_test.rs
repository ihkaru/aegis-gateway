//! Phase 39 Integration Test Suite: Interactive Control Plane and Full Embedded UI Suite
//! Target Gaps: MikkoParkkola/mcp-gateway#2532, docker/mcp-gateway#544

#![deny(unsafe_code)]

use std::sync::Arc;
use aegis_gateway::control::web_ui::EmbeddedAdminServer;
use aegis_gateway::AegisGateway;
use serde_json::json;

#[tokio::test]
async fn test_phase39_full_embedded_control_suite_lifecycle() {
    let gateway = Arc::new(AegisGateway::default());
    let server = EmbeddedAdminServer::new(8485, true).with_gateway(gateway);

    // 1. Verify dynamic backends listing and registration
    let (list_status, list_body, _) = server.handle_request("GET", "/api/v1/backends").await;
    assert_eq!(list_status, 200);
    let backends: Vec<serde_json::Value> = serde_json::from_slice(&list_body).unwrap();
    assert!(!backends.is_empty(), "Default backends should be present");
    assert_eq!(backends[0]["circuit_breaker"], "CLOSED");

    // 2. Register new backend dynamically
    let reg_payload = json!({
        "name": "analytics_worker",
        "backend_type": "Subprocess",
        "command": "python3",
        "args": ["-m", "analytics"]
    });
    let (reg_status, reg_body, _) = server
        .handle_request_with_body("POST", "/api/v1/backends", &serde_json::to_vec(&reg_payload).unwrap())
        .await;
    assert_eq!(reg_status, 200);
    let reg_json: serde_json::Value = serde_json::from_slice(&reg_body).unwrap();
    assert_eq!(reg_json["status"], "registered");
    assert_eq!(reg_json["backend"], "analytics_worker");

    // 3. Deregister backend dynamically
    let (del_status, del_body, _) = server
        .handle_request("DELETE", "/api/v1/backends/analytics_worker")
        .await;
    assert_eq!(del_status, 200);
    let del_json: serde_json::Value = serde_json::from_slice(&del_body).unwrap();
    assert_eq!(del_json["status"], "deregistered");
    assert_eq!(del_json["backend"], "analytics_worker");
}

#[tokio::test]
async fn test_phase39_progressive_disclosure_and_playground_dispatch() {
    let gateway = Arc::new(AegisGateway::default());
    let server = EmbeddedAdminServer::new(8485, true).with_gateway(gateway);

    // 1. Verify progressive disclosure catalog
    let (tools_status, tools_body, _) = server.handle_request("GET", "/api/v1/tools").await;
    assert_eq!(tools_status, 200);
    let tools: Vec<serde_json::Value> = serde_json::from_slice(&tools_body).unwrap();
    assert!(!tools.is_empty());
    let first_tool = &tools[0];
    assert!(first_tool["name"].is_string());
    assert!(first_tool["purpose"].is_string());
    assert!(first_tool["signature"].is_string());
    assert!(first_tool["schema"].is_object());

    // 2. Dispatch interactive tool call with arguments
    let exec_payload = json!({
        "tool": "query_postgresql_ledger",
        "arguments": { "query": "SELECT * FROM audit_logs LIMIT 10" }
    });
    let (exec_status, exec_body, _) = server
        .handle_request_with_body("POST", "/api/v1/tools/call", &serde_json::to_vec(&exec_payload).unwrap())
        .await;
    assert_eq!(exec_status, 200);
    let exec_json: serde_json::Value = serde_json::from_slice(&exec_body).unwrap();
    assert_eq!(exec_json["outcome"], "DISPATCHED");
    assert_eq!(exec_json["tool"], "query_postgresql_ledger");
    assert!(exec_json["result"]["status"] == "success");

    // 3. Verify SOC 2 Report export
    let (soc2_status, soc2_body, _) = server
        .handle_request_with_body("POST", "/api/v1/audit/soc2", &[])
        .await;
    assert_eq!(soc2_status, 200);
    let soc2_json: serde_json::Value = serde_json::from_slice(&soc2_body).unwrap();
    assert!(soc2_json["report_id"].as_str().unwrap().starts_with("SOC2-"));
    assert!(soc2_json["compliance_controls"].is_array());
}
