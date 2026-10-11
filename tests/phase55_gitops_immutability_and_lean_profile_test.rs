#![deny(unsafe_code)]

use serde_json::json;

use propylon::core::profile::EnterpriseRuntimeProfile;
use propylon::control::web_ui::EmbeddedAdminServer;
use propylon::control::web_api::WebApiDispatcher;
use propylon::control::config_api::ConfigApiDispatcher;

#[tokio::test]
async fn test_enterprise_runtime_profile_builder_and_defaults() {
    let default_prof = EnterpriseRuntimeProfile::default();
    assert!(default_prof.ui_enabled);
    assert!(!default_prof.immutable_config);
    assert!(!default_prof.lean_memory);
    assert!(default_prof.check_mutation_allowed().is_ok());

    let hardened_prof = EnterpriseRuntimeProfile::default()
        .with_ui_enabled(false)
        .with_immutable_config(true)
        .with_lean_memory(true);

    assert!(!hardened_prof.ui_enabled);
    assert!(hardened_prof.immutable_config);
    assert!(hardened_prof.lean_memory);
    assert!(hardened_prof.check_mutation_allowed().is_err());
}

#[tokio::test]
async fn test_headless_ui_opt_out_returns_404_json() {
    let headless_profile = EnterpriseRuntimeProfile::default().with_ui_enabled(false);
    let server = EmbeddedAdminServer::new(8485, true).with_profile(headless_profile);

    // Root index.html request
    let (status, body, mime) = server.handle_request("GET", "/").await;
    assert_eq!(status, 404);
    assert_eq!(mime, "application/json");
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body_json["error"], "not_found");
    assert!(body_json["message"].as_str().unwrap().contains("headless mode"));

    // Static assets request
    let (status_asset, _, _) = server.handle_request("GET", "/assets/index.js").await;
    assert_eq!(status_asset, 404);

    // Enabled UI request returns 200
    let enabled_server = EmbeddedAdminServer::new(8485, true);
    let (enabled_status, _, _) = enabled_server.handle_request("GET", "/").await;
    assert_eq!(enabled_status, 200);
}

#[tokio::test]
async fn test_gitops_immutability_rest_rejection() {
    let immutable_profile = EnterpriseRuntimeProfile::default().with_immutable_config(true);
    let invocations = std::sync::RwLock::new(Vec::new());
    let dlp_events = std::sync::RwLock::new(Vec::new());
    let tier = std::sync::RwLock::new("Hybrid".to_string());

    // 1. POST /api/v1/backends rejected with 403
    let backend_payload = json!({ "name": "rogue_server" }).to_string();
    let (code, body, _) = WebApiDispatcher::dispatch_with_profile(
        "POST", "/api/v1/backends", backend_payload.as_bytes(),
        std::time::Instant::now(), None, None, &invocations, &dlp_events, &tier, &immutable_profile,
    ).await;
    assert_eq!(code, 403);
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp["code"], "IMMUTABLE_GITOPS_ACTIVE");
    assert_eq!(resp["error"], "forbidden");

    // 2. DELETE /api/v1/backends/rogue_server rejected with 403
    let (code_del, body_del, _) = WebApiDispatcher::dispatch_with_profile(
        "DELETE", "/api/v1/backends/rogue_server", &[],
        std::time::Instant::now(), None, None, &invocations, &dlp_events, &tier, &immutable_profile,
    ).await;
    assert_eq!(code_del, 403);
    let resp_del: serde_json::Value = serde_json::from_slice(&body_del).unwrap();
    assert_eq!(resp_del["code"], "IMMUTABLE_GITOPS_ACTIVE");

    // 3. POST /api/v1/policy/tier rejected with 403
    let tier_payload = json!({ "tier": "StrictAirgapped" }).to_string();
    let (code_tier, body_tier, _) = WebApiDispatcher::dispatch_with_profile(
        "POST", "/api/v1/policy/tier", tier_payload.as_bytes(),
        std::time::Instant::now(), None, None, &invocations, &dlp_events, &tier, &immutable_profile,
    ).await;
    assert_eq!(code_tier, 403);
    let resp_tier: serde_json::Value = serde_json::from_slice(&body_tier).unwrap();
    assert_eq!(resp_tier["code"], "IMMUTABLE_GITOPS_ACTIVE");

    // 4. POST /api/v1/config/apply rejected with 403
    let config_payload = json!({ "content": "mcp_servers: {}" }).to_string();
    let (code_apply, body_apply, _) = ConfigApiDispatcher::dispatch_with_profile(
        "POST", "/api/v1/config/apply", config_payload.as_bytes(), None, &immutable_profile,
    ).await;
    assert_eq!(code_apply, 403);
    let resp_apply: serde_json::Value = serde_json::from_slice(&body_apply).unwrap();
    assert_eq!(resp_apply["code"], "IMMUTABLE_GITOPS_ACTIVE");

    // 5. Read-only endpoints remain accessible (200 OK)
    let (code_get, _, _) = WebApiDispatcher::dispatch_with_profile(
        "GET", "/api/v1/overview", &[],
        std::time::Instant::now(), None, None, &invocations, &dlp_events, &tier, &immutable_profile,
    ).await;
    assert_eq!(code_get, 200);

    let (code_backends, _, _) = WebApiDispatcher::dispatch_with_profile(
        "GET", "/api/v1/backends", &[],
        std::time::Instant::now(), None, None, &invocations, &dlp_events, &tier, &immutable_profile,
    ).await;
    assert_eq!(code_backends, 200);

    let (code_cfg, _, _) = ConfigApiDispatcher::dispatch_with_profile(
        "GET", "/api/v1/config/raw", &[], None, &immutable_profile,
    ).await;
    assert_eq!(code_cfg, 200);
}

#[tokio::test]
async fn test_mutable_mode_allows_config_mutations() {
    let mutable_profile = EnterpriseRuntimeProfile::default().with_immutable_config(false);
    let invocations = std::sync::RwLock::new(Vec::new());
    let dlp_events = std::sync::RwLock::new(Vec::new());
    let tier = std::sync::RwLock::new("Hybrid".to_string());

    let backend_payload = json!({ "name": "allowed_server" }).to_string();
    let (code, body, _) = WebApiDispatcher::dispatch_with_profile(
        "POST", "/api/v1/backends", backend_payload.as_bytes(),
        std::time::Instant::now(), None, None, &invocations, &dlp_events, &tier, &mutable_profile,
    ).await;
    assert_eq!(code, 200);
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp["status"], "registered");

    let (code_del, body_del, _) = WebApiDispatcher::dispatch_with_profile(
        "DELETE", "/api/v1/backends/allowed_server", &[],
        std::time::Instant::now(), None, None, &invocations, &dlp_events, &tier, &mutable_profile,
    ).await;
    assert_eq!(code_del, 200);
    let resp_del: serde_json::Value = serde_json::from_slice(&body_del).unwrap();
    assert_eq!(resp_del["status"], "deregistered");
}

#[tokio::test]
async fn test_lean_memory_profile_skips_ui_event_buffers() {
    let lean_profile = EnterpriseRuntimeProfile::default().with_lean_memory(true);
    let server = EmbeddedAdminServer::new(8485, true).with_profile(lean_profile);

    // Call record_invocation and record_dlp_event
    server.record_invocation(json!({ "tool": "test_tool", "latency_ms": 1.2 }));
    server.record_dlp_event(json!({ "event": "PII_MASKED", "field": "email" }));

    // Verify recent calls endpoint returns empty array (memory buffers bypassed)
    let (_, body_calls, _) = server.handle_request("GET", "/api/v1/recent_calls").await;
    let calls: Vec<serde_json::Value> = serde_json::from_slice(&body_calls).unwrap();
    assert_eq!(calls.len(), 0);

    let (_, body_dlp, _) = server.handle_request("GET", "/api/v1/dlp/events").await;
    let evts: Vec<serde_json::Value> = serde_json::from_slice(&body_dlp).unwrap();
    assert_eq!(evts.len(), 0);

    // Non-lean server properly retains records
    let standard_server = EmbeddedAdminServer::new(8485, true)
        .with_profile(EnterpriseRuntimeProfile::default().with_lean_memory(false));
    standard_server.record_invocation(json!({ "tool": "test_tool_2" }));
    let (_, body_std_calls, _) = standard_server.handle_request("GET", "/api/v1/recent_calls").await;
    let std_calls: Vec<serde_json::Value> = serde_json::from_slice(&body_std_calls).unwrap();
    assert_eq!(std_calls.len(), 1);
}
