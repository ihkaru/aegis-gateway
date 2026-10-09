// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

//! E2E Tier 4: Enterprise DevOps & Cloud-Native HA Test Suite
//! Persona: SRE / Platform Engineer deploying Aegis Gateway in Kubernetes clusters
//! behind AWS ALB / Ingress Controller with health probes and zero-downtime draining.

use std::sync::Arc;
use serde_json::json;

use aegis_gateway::core::transport::WireProtocolHandler;
use aegis_gateway::state::DrainCoordinator;
use aegis_gateway::transport::{McpProtocolHandler, StreamableHttpTransport};
use aegis_gateway::AegisGateway;

#[tokio::test]
async fn test_enterprise_ha_streamable_http_rfc_compliance() {
    let gateway = Arc::new(AegisGateway::default());
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let http = StreamableHttpTransport::new(handler, "0.0.0.0:8443");

    // 1. Modern unified POST /mcp RFC 2025-03-26
    let req = json!({
        "jsonrpc": "2.0",
        "id": "alb-req-1",
        "method": "ping"
    });
    let (status, content_type, body) = http
        .process_http_request("/mcp", "POST", &req.to_string())
        .await
        .expect("http post mcp");
    assert_eq!(status, 200);
    assert_eq!(content_type, "application/json");
    let val: serde_json::Value = serde_json::from_str(&body).expect("parse response");
    assert_eq!(val["id"], "alb-req-1");

    // 2. Initialized Notification over POST /mcp -> 204 No Content
    let notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let (notif_status, _, notif_body) = http
        .process_http_request("/mcp", "POST", &notif.to_string())
        .await
        .expect("http post notification");
    assert_eq!(notif_status, 204);
    assert!(notif_body.is_empty());
}

#[tokio::test]
async fn test_enterprise_ha_k8s_health_probes() {
    let gateway = Arc::new(AegisGateway::default());
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let http = StreamableHttpTransport::new(handler, "0.0.0.0:8080");

    // Kubernetes liveness probe: /healthz
    let (code, content_type, body) = http
        .process_http_request("/healthz", "GET", "")
        .await
        .expect("healthz check");
    assert_eq!(code, 200);
    assert_eq!(content_type, "application/json");
    assert!(body.contains("healthy"));

    // Legacy /health probe
    let (code2, _, body2) = http
        .process_http_request("/health", "GET", "")
        .await
        .expect("health check");
    assert_eq!(code2, 200);
    assert!(body2.contains("healthy"));
}

#[tokio::test]
async fn test_enterprise_ha_graceful_drain_coordination() {
    use std::time::Duration;

    let drain = Arc::new(DrainCoordinator::new());

    // 1. Simulate 2 concurrent inflight requests entering the gateway
    let slot1 = drain.acquire_slot().expect("acquire slot 1");
    let slot2 = drain.acquire_slot().expect("acquire slot 2");
    assert_eq!(drain.inflight_count(), 2);
    assert!(!drain.is_draining());

    // 2. K8s sends SIGTERM -> initiate drain with 2-second budget
    let drain_clone = drain.clone();
    let drain_handle = tokio::spawn(async move {
        drain_clone.wait_drain(Duration::from_millis(2000)).await
    });

    // Verify drain mode is active
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(drain.is_draining());

    // 3. New requests are rejected while draining
    assert!(drain.acquire_slot().is_err());

    // 4. Inflight requests complete their work and drop their guards
    drop(slot1);
    assert_eq!(drain.inflight_count(), 1);

    drop(slot2);
    assert_eq!(drain.inflight_count(), 0);

    // 5. Drain coordinator completes cleanly with 0 dropped requests
    let drain_result = drain_handle.await.expect("join drain");
    assert!(drain_result.is_ok(), "Drain coordinator must cleanly finish");
}
