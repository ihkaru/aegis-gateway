// SPDX-License-Identifier: MIT

use std::sync::Arc;
use aegis_gateway::control::web_ui::EmbeddedAdminServer;
use aegis_gateway::core::approval::ApprovalDecision;
use aegis_gateway::backend::SubprocessBackendRegistry;
use aegis_gateway::AegisGateway;
use serde_json::json;

#[tokio::test]
async fn test_phase35_live_overview_telemetry_binding() {
    let gateway = Arc::new(AegisGateway::default());
    let registry = Arc::new(SubprocessBackendRegistry::new());
    let server = EmbeddedAdminServer::new(8485, true)
        .with_gateway(gateway)
        .with_registry(registry);

    let (status, body, mime) = server.handle_request("GET", "/api/v1/overview").await;
    assert_eq!(status, 200);
    assert_eq!(mime, "application/json");

    let val: serde_json::Value = serde_json::from_slice(&body).expect("Valid JSON");
    assert_eq!(val["status"], "HEALTHY");
    assert_eq!(val["mtls_enforced"], true);
    assert_eq!(val["policy_tier"], "Hybrid");
    assert!(val["memory_rss_mb"].as_f64().unwrap_or(0.0) >= 0.0);
    assert_eq!(val["active_backends"], 0);
    assert_eq!(val["total_tools"], 0);
}

#[tokio::test]
async fn test_phase35_live_hitl_queue_and_cryptographic_resolve() {
    let gateway = Arc::new(AegisGateway::default());
    let server = EmbeddedAdminServer::new(8485, true).with_gateway(Arc::clone(&gateway));

    // Initially HITL queue is empty
    let (status_empty, body_empty, _) = server.handle_request("GET", "/api/v1/hitl/queue").await;
    assert_eq!(status_empty, 200);
    assert_eq!(body_empty, b"[]".to_vec());

    // Intercept a critical action requiring HITL authorization
    let decision = gateway
        .approval_gate()
        .evaluate_action(
            "agent.sec.bot",
            "sess_9001",
            "drop_database_table",
            &json!({"target": "production_users", "cascade": true}),
        )
        .await
        .expect("Evaluation ok");

    let ticket = match decision {
        ApprovalDecision::RequireApproval(t) => t,
        _ => panic!("Expected RequireApproval decision"),
    };

    // Queue now reflects the intercepted ticket
    let (status_q, body_q, _) = server.handle_request("GET", "/api/v1/hitl/queue").await;
    assert_eq!(status_q, 200);
    let q_list: Vec<serde_json::Value> = serde_json::from_slice(&body_q).unwrap();
    assert_eq!(q_list.len(), 1);
    assert_eq!(q_list[0]["ticketId"], ticket.ticket_id);
    assert_eq!(q_list[0]["action"], "drop_database_table");

    // Resolve ticket via live HTTP endpoint with valid HMAC signature
    let resolve_payload = json!({
        "ticket_id": ticket.ticket_id,
        "signature": ticket.hmac_signature,
        "approved": true
    });
    let (res_status, res_body, _) = server
        .handle_request_with_body("POST", "/api/v1/hitl/resolve", &serde_json::to_vec(&resolve_payload).unwrap())
        .await;
    assert_eq!(res_status, 200);
    let res_json: serde_json::Value = serde_json::from_slice(&res_body).unwrap();
    assert_eq!(res_json["status"], "resolved");
    assert_eq!(res_json["approved"], true);

    // Queue is now empty after cryptographic resolution
    let (_, body_resolved, _) = server.handle_request("GET", "/api/v1/hitl/queue").await;
    assert_eq!(body_resolved, b"[]".to_vec());
}

#[tokio::test]
async fn test_phase35_live_recent_calls_and_dlp_event_stream() {
    let server = EmbeddedAdminServer::new(8485, true);

    server.record_invocation(json!({
        "time": "14:02:11.901",
        "agent": "agent.gemini.dev",
        "tool": "read_internal_wiki",
        "duration": "0.45ms",
        "status": "SUCCESS"
    }));

    server.record_dlp_event(json!({
        "id": "dlp_evt_live_1092",
        "time": "14:02:12.100",
        "policy": "PCI-DSS v4.0",
        "pattern": "Primary Account Number",
        "action": "REDACTED_LUHN_MASK"
    }));

    let (_, calls_body, _) = server.handle_request("GET", "/api/v1/recent_calls").await;
    let calls: Vec<serde_json::Value> = serde_json::from_slice(&calls_body).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["tool"], "read_internal_wiki");

    let (_, dlp_body, _) = server.handle_request("GET", "/api/v1/dlp/events").await;
    let evts: Vec<serde_json::Value> = serde_json::from_slice(&dlp_body).unwrap();
    assert_eq!(evts.len(), 1);
    assert_eq!(evts[0]["action"], "REDACTED_LUHN_MASK");
}
