// SPDX-License-Identifier: MIT

#![deny(unsafe_code)]

use std::sync::Arc;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use propylon::cli::args::LoginArgs;
use propylon::cli::commands::run_login;
use propylon::core::transport::WireProtocolHandler;
use propylon::state::DrainCoordinator;
use propylon::transport::{LiveHttpServer, McpProtocolHandler};
use propylon::PropylonGateway;

fn setup_test_server() -> axum::Router {
    let gateway = Arc::new(PropylonGateway::default());
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let drain = Arc::new(DrainCoordinator::new());
    let server = LiveHttpServer::new(handler, drain, "127.0.0.1", 39600).expect("server init");
    server.router()
}

#[tokio::test]
async fn test_real_saml_302_redirect_on_login_endpoint() {
    let app = setup_test_server();

    let req = Request::builder()
        .method("GET")
        .uri("/saml/login")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);

    let location = resp.headers().get("location").expect("location header");
    let loc_str = location.to_str().unwrap();
    assert!(
        loc_str.starts_with("https://auth.dvlpid.my.id/t/bps/saml/sso?SAMLRequest="),
        "Location must point to rIDM IdP with AuthnRequest"
    );
}

#[tokio::test]
async fn test_strict_session_rejection_without_token() {
    let app = setup_test_server();

    // 1. Unauthenticated request without token must be rejected with 401
    let unauth_req = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/session")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(unauth_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["authenticated"], false);

    // 2. Request with valid Authorization header succeeds
    let auth_req = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/session")
        .header("authorization", "Bearer propylon_jwt_ihzakarunia@bps.go.id")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(auth_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["authenticated"], true);
    assert_eq!(json["subject"], "ihzakarunia@bps.go.id");
    assert_eq!(json["tenant"], "bps");
}

#[tokio::test]
async fn test_acs_form_post_302_redirect_to_dashboard() {
    let app = setup_test_server();

    let idp_issuer = "https://auth.dvlpid.my.id/t/bps";
    let sp_entity_id = "https://mcp.dvlpid.my.id/saml/metadata";
    let saml_xml = format!(
        "<samlp:Response xmlns:samlp=\"urn:oasis:names:tc:SAML:2.0:protocol\" Version=\"2.0\">\
           <saml:Issuer xmlns:saml=\"urn:oasis:names:tc:SAML:2.0:assertion\">{idp_issuer}</saml:Issuer>\
           <saml:Assertion xmlns:saml=\"urn:oasis:names:tc:SAML:2.0:assertion\">\
             <saml:Issuer>{idp_issuer}</saml:Issuer>\
             <saml:Subject>\
               <saml:NameID Format=\"urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress\">testuser@bps.go.id</saml:NameID>\
             </saml:Subject>\
             <saml:Conditions NotBefore=\"2026-10-11T00:00:00Z\" NotOnOrAfter=\"2026-10-12T00:00:00Z\">\
               <saml:AudienceRestriction><saml:Audience>{sp_entity_id}</saml:Audience></saml:AudienceRestriction>\
             </saml:Conditions>\
           </saml:Assertion>\
         </samlp:Response>"
    );

    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    let b64_resp = B64.encode(saml_xml.as_bytes());
    let form_body = format!("SAMLResponse={}", b64_resp);

    let req = Request::builder()
        .method("POST")
        .uri("/saml/acs")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(form_body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);

    let location = resp.headers().get("location").expect("location header");
    let loc_str = location.to_str().unwrap();
    assert!(
        loc_str.starts_with("/?token=propylon_jwt_testuser@bps.go.id"),
        "ACS must redirect browser to dashboard with token"
    );
}

#[tokio::test]
async fn test_strict_zero_trust_mcp_data_plane_rejection() {
    let app = setup_test_server();

    // 1. Unauthenticated request to /mcp under strict tier rejected with 401
    let ping_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "2024-11-05")
        .header("x-propylon-policy-tier", "strict")
        .body(Body::from(ping_req.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"]["code"], -32001);
    assert!(json["error"]["message"].as_str().unwrap().contains("Strict Zero Trust requires SAML Bearer token"));

    // 2. Authenticated request to /mcp with Bearer token accepted
    let auth_ping_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "2024-11-05")
        .header("x-propylon-policy-tier", "strict")
        .header("authorization", "Bearer propylon_jwt_engineer@bps.go.id")
        .body(Body::from(ping_req.to_string()))
        .unwrap();

    let resp = app.oneshot(auth_ping_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[test]
fn test_cli_login_token_persistence() {
    let temp_token = "propylon_jwt_cli_test_user@bps.go.id";
    let args = LoginArgs {
        remote: "https://mcp.dvlpid.my.id".to_string(),
        token: Some(temp_token.to_string()),
    };

    let res = run_login(args);
    assert!(res.is_ok());

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let token_path = std::path::Path::new(&home).join(".config").join("propylon").join("token");
    assert!(token_path.exists());
    let read_tok = std::fs::read_to_string(token_path).unwrap();
    assert_eq!(read_tok.trim(), temp_token);
}

#[tokio::test]
async fn test_dynamic_shared_tier_hot_swap_enforces_strict_mcp() {
    let app = setup_test_server();

    // 1. Post to update policy tier to Strict
    let tier_payload = serde_json::json!({ "tier": "Strict" });
    let tier_req = Request::builder()
        .method("POST")
        .uri("/api/v1/policy/tier")
        .header("content-type", "application/json")
        .body(Body::from(tier_payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(tier_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Unauthenticated request to /mcp without any headers now fails with 401
    let ping_req = serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
    let mcp_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "2024-11-05")
        .body(Body::from(ping_req.to_string()))
        .unwrap();

    let mcp_resp = app.clone().oneshot(mcp_req).await.unwrap();
    assert_eq!(mcp_resp.status(), StatusCode::UNAUTHORIZED);

    // Reset back to Hybrid
    let reset_payload = serde_json::json!({ "tier": "Hybrid" });
    let reset_req = Request::builder()
        .method("POST")
        .uri("/api/v1/policy/tier")
        .header("content-type", "application/json")
        .body(Body::from(reset_payload.to_string()))
        .unwrap();
    let _ = app.oneshot(reset_req).await.unwrap();
}
