// SPDX-License-Identifier: MIT

use std::sync::Arc;
use aegis_gateway::control::web_ui::EmbeddedAdminServer;
use aegis_gateway::AegisGateway;
use serde_json::json;

#[tokio::test]
async fn test_phase34_backends_discovery_and_registration() {
    let server = EmbeddedAdminServer::new(8485, true);

    // 1. List backends
    let (status, body, _) = server.handle_request("GET", "/api/v1/backends").await;
    assert_eq!(status, 200);
    let list: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!list.is_empty());
    assert_eq!(list[0]["circuit_breaker"], "CLOSED");

    // 2. Register dynamic backend
    let reg_payload = json!({
        "name": "dynamic_vault_backend",
        "command": "python3",
        "args": ["-m", "vault_mcp"]
    });
    let (reg_status, reg_body, _) = server
        .handle_request_with_body("POST", "/api/v1/backends", &serde_json::to_vec(&reg_payload).unwrap())
        .await;
    assert_eq!(reg_status, 200);
    let reg_json: serde_json::Value = serde_json::from_slice(&reg_body).unwrap();
    assert_eq!(reg_json["status"], "registered");
}

#[tokio::test]
async fn test_phase34_tools_catalog_and_playground_execution() {
    let gateway = Arc::new(AegisGateway::default());
    let server = EmbeddedAdminServer::new(8485, true).with_gateway(gateway);

    // 1. Discover tools
    let (status, body, _) = server.handle_request("GET", "/api/v1/tools").await;
    assert_eq!(status, 200);
    let tools: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!tools.is_empty());

    // 2. Execute playground tool call with DLP diff
    let call_payload = json!({
        "tool": "fetch_weather",
        "arguments": { "city": "Jakarta" }
    });
    let (call_status, call_body, _) = server
        .handle_request_with_body("POST", "/api/v1/tools/call", &serde_json::to_vec(&call_payload).unwrap())
        .await;
    assert_eq!(call_status, 200);
    let call_json: serde_json::Value = serde_json::from_slice(&call_body).unwrap();
    assert!(call_json["raw"].is_object());
    assert!(call_json["sanitized"].is_object());
}

#[tokio::test]
async fn test_phase34_policy_tier_hot_swap() {
    let server = EmbeddedAdminServer::new(8485, true);

    // Initial tier is Hybrid
    let (status, body, _) = server.handle_request("GET", "/api/v1/policy/tier").await;
    assert_eq!(status, 200);
    let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["tier"], "Hybrid");

    // Hot-swap to Strict
    let swap_payload = json!({ "tier": "Strict" });
    let (swap_status, swap_body, _) = server
        .handle_request_with_body("POST", "/api/v1/policy/tier", &serde_json::to_vec(&swap_payload).unwrap())
        .await;
    assert_eq!(swap_status, 200);
    let swap_json: serde_json::Value = serde_json::from_slice(&swap_body).unwrap();
    assert_eq!(swap_json["tier"], "Strict");

    // Verify overview reflects new tier
    let (_, ov_body, _) = server.handle_request("GET", "/api/v1/overview").await;
    let ov_json: serde_json::Value = serde_json::from_slice(&ov_body).unwrap();
    assert_eq!(ov_json["policy_tier"], "Strict");
}

#[tokio::test]
async fn test_phase34_siem_soc2_report_export() {
    let server = EmbeddedAdminServer::new(8485, true);

    let (status, body, _) = server.handle_request("GET", "/api/v1/audit/soc2").await;
    assert_eq!(status, 200);
    let report: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(report["standard"].as_str().unwrap_or_default().contains("SOC 2 Type II"));
    assert!(report["cryptographic_integrity_verified"].as_bool().unwrap_or(false));
}
