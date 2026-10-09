//! Phase 13 Integration Test Suite: Stream Buffer Reuse OAuth Resource Metadata and Process Reaper
//! Target Gaps: microsoft/mcp-gateway#48, microsoft/mcp-gateway#17, docker/mcp-gateway#483, docker/mcp-gateway#453

#![deny(unsafe_code)]

use std::sync::Arc;
use axum::http::Request;
use serde_json::json;
use tower::ServiceExt;

use aegis_gateway::backend::reaper::ProcessGroupReaper;
use aegis_gateway::core::transport::WireProtocolHandler;
use aegis_gateway::policy::oauth_metadata::ProtectedResourceMetadata;
use aegis_gateway::state::DrainCoordinator;
use aegis_gateway::transport::buffer::ReusableStreamBuffer;
use aegis_gateway::transport::server::LiveHttpServer;
use aegis_gateway::transport::signer::McpMessageSigner;

#[tokio::test]
async fn test_microsoft_issue_48_reusable_stream_buffer_multi_read() {
    let raw_payload = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "calculate_tax",
            "arguments": { "income": 75000 }
        },
        "id": "req-999"
    });

    let payload_bytes = serde_json::to_vec(&raw_payload).expect("Serialize to bytes");
    let buffer = ReusableStreamBuffer::new(payload_bytes.clone());

    // 1. Multiple sequential JSON reads without stream consumption errors
    let pass1 = buffer.read_json().expect("First read passes");
    let pass2 = buffer.read_json().expect("Second read passes (no stream exhausted error)");
    let pass3 = buffer.read_json().expect("Third read passes (no stream exhausted error)");

    assert_eq!(pass1, raw_payload);
    assert_eq!(pass2, raw_payload);
    assert_eq!(pass3, raw_payload);

    // 2. Multi-pass consumption with independent forked cursors
    let results = buffer
        .multi_pass_consume(5, |cursor, idx| {
            use std::io::Read;
            let mut read_buf = Vec::new();
            cursor.read_to_end(&mut read_buf).map_err(|e| {
                aegis_gateway::core::error::AegisError::StateError(e.to_string())
            })?;
            assert_eq!(read_buf.len(), buffer.len());
            Ok(idx)
        })
        .expect("Multi-pass reader execution succeeds");

    assert_eq!(results, vec![0, 1, 2, 3, 4]);

    // 3. Buffer capacity limit boundary check
    let small_limit_res = ReusableStreamBuffer::with_max_capacity(payload_bytes, 10);
    assert!(small_limit_res.is_err(), "Exceeding max capacity must return error");
}

#[tokio::test]
async fn test_microsoft_issue_17_and_docker_474_oauth_protected_resource_metadata() {
    let metadata = ProtectedResourceMetadata::new(
        "http://localhost:39400/mcp",
        "https://login.microsoftonline.com/common/v2.0",
    )
    .with_scopes(vec!["mcp:read".to_string(), "mcp:write".to_string()])
    .with_documentation("https://aegis-gateway.dev/docs/oauth");

    // 1. Validate metadata attributes
    assert_eq!(metadata.resource, "http://localhost:39400/mcp");
    assert!(metadata.validate_audience("http://localhost:39400/mcp"));
    assert!(metadata.validate_request_scope("mcp:read"));
    assert!(!metadata.validate_request_scope("mcp:superadmin"));

    let json_val = metadata.to_json();
    assert_eq!(json_val["resource"], "http://localhost:39400/mcp");
    assert_eq!(json_val["bearer_methods_supported"][0], "header");

    // 2. Test live HTTP endpoint integration
    struct DummyHandler;
    #[async_trait::async_trait]
    impl WireProtocolHandler for DummyHandler {
        async fn handle_message(&self, _msg: &str) -> aegis_gateway::core::error::AegisResult<Option<String>> {
            Ok(None)
        }
    }

    let drain = Arc::new(DrainCoordinator::new());
    let server = LiveHttpServer::new(Arc::new(DummyHandler), drain, "127.0.0.1", 39400).expect("Server init");
    let router = server.router();

    let req = Request::builder()
        .uri("/.well-known/oauth-protected-resource")
        .method("GET")
        .body(axum::body::Body::empty())
        .expect("Request builder");

    let response = router.oneshot(req).await.expect("Oneshot call succeeds");
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Read response bytes");
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("Valid JSON");
    assert_eq!(resp_json["resource"], "http://localhost:39400/mcp");
    assert!(resp_json["scopes_supported"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn test_docker_issue_483_concurrency_process_reaper() {
    let reaper = ProcessGroupReaper::new();

    // 1. Concurrently spawn and track 40 child PIDs across tasks
    let mut handles = Vec::new();
    for i in 1000..1040 {
        let r = reaper.clone();
        handles.push(tokio::spawn(async move {
            r.register(i).await;
        }));
    }

    for h in handles {
        h.await.expect("Spawn task succeeds");
    }

    assert_eq!(reaper.active_count().await, 40);
    assert!(reaper.is_tracked(1015).await);

    // 2. Terminate a single specific PID cleanly
    let terminated = reaper.terminate_pid(1015).await;
    assert!(terminated);
    assert_eq!(reaper.active_count().await, 39);
    assert!(!reaper.is_tracked(1015).await);

    // 3. Clean exit deregistration
    reaper.deregister(1016).await;
    assert_eq!(reaper.active_count().await, 38);

    // 4. Batch reap all remaining lingering child processes (zero zombie guarantee)
    let reaped_count = reaper.reap_all().await;
    assert_eq!(reaped_count, 38);
    assert_eq!(reaper.active_count().await, 0);
}

#[tokio::test]
async fn test_docker_issue_453_mcp_message_signer_and_verification() {
    let secret = "enterprise-hmac-sha256-signing-secret";
    let gateway_id = "aegis-gw-us-east-1a";
    let signer = McpMessageSigner::new(secret, gateway_id);

    let payload = r#"{"jsonrpc":"2.0","result":{"temperature":22.5},"id":1}"#;

    // 1. Sign message payload
    let header = signer.sign_message(payload);
    assert!(header.starts_with("t="));
    assert!(header.contains(",v1="));

    // 2. Verify legitimate signature
    assert!(signer.verify_signature(payload, &header, 300));

    // 3. Tampered payload must fail verification
    let tampered_payload = r#"{"jsonrpc":"2.0","result":{"temperature":99.9},"id":1}"#;
    assert!(!signer.verify_signature(tampered_payload, &header, 300));

    // 4. Malformed signature header must fail safely without panic
    assert!(!signer.verify_signature(payload, "invalid-header-syntax", 300));

    // 5. Expired timestamp (skew > limit) must fail
    assert!(!signer.verify_signature(payload, "t=100000,v1=abcdef12345", 10));
}
