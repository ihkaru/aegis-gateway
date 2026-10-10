// SPDX-License-Identifier: MIT

use std::sync::Arc;

use aegis_gateway::core::oauth_connect::{AuthProviderConfig, OAuthConnectEngine};
use aegis_gateway::policy::delegation::UserIdentityDelegationBroker;
use aegis_gateway::policy::oauth_connect::VendorAgnosticOAuthRouter;

#[tokio::test]
async fn test_oauth_connect_pkce_initiation_and_callback() {
    let delegation_broker = Arc::new(UserIdentityDelegationBroker::new());
    let router = VendorAgnosticOAuthRouter::new(delegation_broker)
        .with_provider(AuthProviderConfig {
            provider_name: "google".to_string(),
            auth_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
            client_id: "google-test-client-id-123.apps.googleusercontent.com".to_string(),
            default_scopes: vec![
                "https://www.googleapis.com/auth/drive".to_string(),
                "https://www.googleapis.com/auth/spreadsheets".to_string(),
            ],
        });

    let caller_id = "alice@company.com";
    let redirect_uri = "https://mcp.dvlpid.my.id/oauth/callback";

    // 1. Initial check: user does not have active token
    assert!(!router.has_active_token(caller_id, "google").await.unwrap());

    // 2. Initiate PKCE connect flow
    let (auth_url, session) = router
        .initiate_connect(caller_id, "google", redirect_uri)
        .await
        .unwrap();

    assert!(auth_url.starts_with("https://accounts.google.com/o/oauth2/v2/auth"));
    assert!(auth_url.contains("code_challenge="));
    assert!(auth_url.contains("code_challenge_method=S256"));
    assert!(auth_url.contains(&format!("state={}", session.state_nonce)));
    assert_eq!(session.caller_id, caller_id);
    assert_eq!(session.provider, "google");

    // 3. Handle OAuth callback with code and valid state
    let authorized_user = router
        .handle_callback("4/0Aean...auth_code_from_google", &session.state_nonce)
        .await
        .unwrap();

    assert_eq!(authorized_user, caller_id);

    // 4. Verification: user now has active token
    assert!(router.has_active_token(caller_id, "google").await.unwrap());
}

#[tokio::test]
async fn test_oauth_connect_csrf_state_nonce_rejection() {
    let delegation_broker = Arc::new(UserIdentityDelegationBroker::new());
    let router = VendorAgnosticOAuthRouter::new(delegation_broker);

    // Callback with bogus state must fail closed
    let bogus_state = "forged-csrf-state-nonce";
    let res = router.handle_callback("sample_code", bogus_state).await;

    assert!(res.is_err());
    let err_msg = res.err().unwrap().to_string();
    assert!(err_msg.contains("CSRF violation"));
}
