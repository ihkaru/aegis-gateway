// SPDX-License-Identifier: MIT

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

use propylon::cli::args::LoginArgs;
use propylon::cli::commands::run_login;
use propylon::control::auth_api::AuthApiDispatcher;
use propylon::control::saml_api::SamlApiDispatcher;
use propylon::core::transport::WireProtocolHandler;
use propylon::policy::{get_shared_jwt_service, JwtTokenService};
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
async fn test_enterprise_cryptographic_jwt_generation_and_validation() {
    let jwt_service = JwtTokenService::new("ciso_super_secret_test_key_2026", "https://mcp.dvlpid.my.id", "propylon-gateway");
    let token = jwt_service.generate_token(
        "chief_security_officer@bps.go.id",
        vec!["ciso".to_string(), "admin".to_string()],
        Some("Information Security Directorate".to_string()),
        Some("bps".to_string()),
        3600,
    );

    // 1. Must be standard RFC 7519 3-part dot-separated JWT
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT must contain header.payload.signature");

    // 2. Cryptographic signature and claim verification
    let claims = jwt_service.validate_token(&token).expect("Valid token must verify");
    assert_eq!(claims.sub, "chief_security_officer@bps.go.id");
    assert_eq!(claims.roles, vec!["ciso".to_string(), "admin".to_string()]);
    assert_eq!(claims.department.as_deref(), Some("Information Security Directorate"));
    assert_eq!(claims.tenant.as_deref(), Some("bps"));
    assert!(!claims.jti.is_empty(), "Unique JTI required for replay protection");

    // 3. Signature tampering detection
    let tampered_token = format!("{}.{}.invalidsignature123", parts[0], parts[1]);
    let err = jwt_service.validate_token(&tampered_token);
    assert!(err.is_err(), "Tampered signature must be rejected");
}

#[tokio::test]
async fn test_jwt_expiration_rejection() {
    let jwt_service = JwtTokenService::new("ciso_super_secret_test_key_2026", "https://mcp.dvlpid.my.id", "propylon-gateway");
    let expired_token = jwt_service.generate_token(
        "analyst@bps.go.id",
        vec!["analyst".to_string()],
        None,
        None,
        -10, // Expired 10 seconds ago
    );

    let err = jwt_service.validate_token(&expired_token);
    assert!(err.is_err(), "Expired token must be rejected");
    let err_str = err.err().unwrap().to_string();
    assert!(err_str.contains("expired"), "Error must state token expired");
}

#[tokio::test]
async fn test_token_rolling_refresh_and_replay_protection() {
    let jwt_service = get_shared_jwt_service();
    let token_a = jwt_service.generate_token(
        "dev_agent@bps.go.id",
        vec!["developer".to_string()],
        Some("IPDS".to_string()),
        Some("bps".to_string()),
        3600,
    );

    // 1. Roll token via POST /api/v1/auth/refresh
    let req_body = json!({ "token": token_a }).to_string();
    let (status, resp_bytes, _) = AuthApiDispatcher::dispatch("POST", "/api/v1/auth/refresh", req_body.as_bytes()).await;
    assert_eq!(status, 200, "Token refresh must succeed");

    let resp_json: serde_json::Value = serde_json::from_slice(&resp_bytes).unwrap();
    assert_eq!(resp_json["status"], "rolled");
    let token_b = resp_json["access_token"].as_str().expect("access_token string");

    // 2. Token B is valid and active
    let claims_b = jwt_service.validate_token(token_b).expect("New rolled token must be valid");
    assert_eq!(claims_b.sub, "dev_agent@bps.go.id");

    // 3. Token A is revoked immediately upon refresh (prevent replay attack)
    let replayed = jwt_service.validate_token(&token_a);
    assert!(replayed.is_err(), "Old token A must be revoked and rejected upon rolling");
}

#[tokio::test]
async fn test_token_revocation_killswitch() {
    let jwt_service = get_shared_jwt_service();
    let token = jwt_service.generate_token(
        "compromised_agent@bps.go.id",
        vec!["developer".to_string()],
        None,
        None,
        3600,
    );

    assert!(jwt_service.validate_token(&token).is_ok());

    // Kill switch via POST /api/v1/auth/revoke
    let req_body = json!({ "token": token }).to_string();
    let (status, resp_bytes, _) = AuthApiDispatcher::dispatch("POST", "/api/v1/auth/revoke", req_body.as_bytes()).await;
    assert_eq!(status, 200);
    let resp_json: serde_json::Value = serde_json::from_slice(&resp_bytes).unwrap();
    assert_eq!(resp_json["status"], "revoked");

    // Must be rejected across policy validator and session introspection
    assert!(jwt_service.validate_token(&token).is_err());
    let (sess_status, _, _) = SamlApiDispatcher::dispatch("GET", "/api/v1/auth/session", token.as_bytes()).await;
    assert_eq!(sess_status, 401, "Revoked token must be rejected in session endpoint");
}

#[tokio::test]
async fn test_strict_zero_trust_mcp_with_real_cryptographic_jwt() {
    let app = setup_test_server();
    let jwt_service = get_shared_jwt_service();
    let valid_jwt = jwt_service.generate_token(
        "ciso_verified@bps.go.id",
        vec!["admin".to_string()],
        Some("Security".to_string()),
        Some("bps".to_string()),
        3600,
    );

    let ping_req = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });

    // Request with valid cryptographic JWT allowed
    let auth_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "2024-11-05")
        .header("x-propylon-policy-tier", "strict")
        .header("authorization", format!("Bearer {}", valid_jwt))
        .body(Body::from(ping_req.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(auth_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Revoke token and verify /mcp immediately denies with 401
    jwt_service.revoke_token(&valid_jwt).expect("Revoke token");
    let denied_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "2024-11-05")
        .header("x-propylon-policy-tier", "strict")
        .header("authorization", format!("Bearer {}", valid_jwt))
        .body(Body::from(ping_req.to_string()))
        .unwrap();

    let denied_resp = app.oneshot(denied_req).await.unwrap();
    assert_eq!(denied_resp.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_cli_login_posix_0600_file_permissions() {
    let temp_token = "propylon_enterprise_test_token_ciso_verified";
    let args = LoginArgs {
        remote: "https://mcp.dvlpid.my.id".to_string(),
        token: Some(temp_token.to_string()),
    };

    let res = run_login(args);
    assert!(res.is_ok());

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let token_path = std::path::Path::new(&home).join(".config").join("propylon").join("token");
    assert!(token_path.exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&token_path).expect("Token metadata");
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "CISO NIST SP 800-63B requirement: mode must be 0600");
    }
}
