// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

//! Phase 9 Integration Test Suite: Production CLI & Runtime Daemon
//! Verifies CLI commands (`init`, `validate`, `doctor`, `audit`) and
//! live Axum HTTP server routing, stats, and health probes.

use std::sync::Arc;
use serde_json::json;

use aegis_gateway::cli::commands::{run_audit, run_doctor, run_init, run_validate};
use aegis_gateway::cli::args::{AuditArgs, DoctorArgs, InitArgs, ValidateArgs};
use aegis_gateway::core::backend::BackendRegistry;
use aegis_gateway::core::transport::WireProtocolHandler;
use aegis_gateway::daemon::DaemonSupervisor;
use aegis_gateway::state::DrainCoordinator;
use aegis_gateway::transport::{LiveHttpServer, McpProtocolHandler};
use aegis_gateway::AegisGateway;

#[tokio::test]
async fn test_phase9_init_and_validate_workflow() {
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let config_path = tmp_dir.path().join("aegis_test.yaml");
    let config_str = config_path.to_str().unwrap().to_string();

    // 1. Run init command to scaffold configuration
    let init_res = run_init(InitArgs {
        output: config_str.clone(),
        force: true,
    });
    assert!(init_res.is_ok(), "Init command must succeed");
    assert!(config_path.exists(), "Configuration file must be created on disk");

    // 2. Run validate command on the generated configuration
    let val_res = run_validate(ValidateArgs {
        config: config_str,
    });
    assert!(val_res.is_ok(), "Generated configuration must pass validation");
}

#[test]
fn test_phase9_doctor_preflight() {
    // Human-readable output
    let doc_res = run_doctor(DoctorArgs { json: false });
    assert!(doc_res.is_ok(), "Doctor command must succeed");

    // JSON output format
    let doc_json_res = run_doctor(DoctorArgs { json: true });
    assert!(doc_json_res.is_ok(), "Doctor JSON mode must succeed");
}

#[tokio::test]
async fn test_phase9_audit_soc2_report_export() {
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let report_path = tmp_dir.path().join("soc2_report.json");
    let report_str = report_path.to_str().unwrap().to_string();

    let audit_res = run_audit(AuditArgs {
        output: Some(report_str),
    })
    .await;
    assert!(audit_res.is_ok(), "Audit export must succeed");
    assert!(report_path.exists(), "Report file must exist on disk");

    let content = std::fs::read_to_string(report_path).expect("read report");
    let val: serde_json::Value = serde_json::from_str(&content).expect("parse json");
    assert_eq!(val["standard"], "SOC 2 Type II / ISO 27001");
}

#[tokio::test]
async fn test_phase9_daemon_supervisor_bootstrap() {
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let config_path = tmp_dir.path().join("test_topology.yaml");
    std::fs::write(
        &config_path,
        r#"
mcpServers:
  echo_server:
    command: "echo"
    args: ["ready"]
    timeout_secs: 10
"#,
    )
    .expect("write yaml");

    let (gateway, handler, registry) = DaemonSupervisor::bootstrap(config_path.to_str())
        .await
        .expect("bootstrap must succeed");

    assert!(gateway.skills().list_skills().await.is_ok());
    assert!(registry.list_backends().await.expect("list").contains(&"echo_server".to_string()));

    // Verify handler responds
    let ping_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping"
    });
    let ping_resp = handler
        .handle_message(&ping_req.to_string())
        .await
        .expect("handle ping")
        .expect("ping response");
    assert!(ping_resp.contains("\"result\":{}"));
}

#[tokio::test]
async fn test_phase9_live_http_server_router_endpoints() {
    let gateway = Arc::new(AegisGateway::default());
    let handler = Arc::new(McpProtocolHandler::new(gateway));
    let drain = Arc::new(DrainCoordinator::new());

    let server = LiveHttpServer::new(handler, drain, "127.0.0.1", 39450)
        .expect("construct server");

    assert_eq!(server.listen_addr().port(), 39450);
    let router = server.router();

    // Verify router handles requests using tower::ServiceExt
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    // 1. GET /healthz
    let req = Request::builder()
        .uri("/healthz")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    // 2. GET /stats
    let req2 = Request::builder()
        .uri("/stats")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp2 = router.clone().oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), axum::http::StatusCode::OK);

    // 3. POST /mcp ping
    let ping_payload = json!({
        "jsonrpc": "2.0",
        "id": "http-test-1",
        "method": "ping"
    })
    .to_string();
    let req3 = Request::builder()
        .uri("/mcp")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(ping_payload))
        .unwrap();
    let resp3 = router.oneshot(req3).await.unwrap();
    assert_eq!(resp3.status(), axum::http::StatusCode::OK);
}
