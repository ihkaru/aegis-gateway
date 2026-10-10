// SPDX-License-Identifier: MIT

use std::sync::Arc;

use aegis_gateway::core::proxy::CredentialProxyEngine;
use aegis_gateway::core::secrets::SecretStore;
use aegis_gateway::policy::secrets::EnvSecretStore;
use aegis_gateway::sandbox::LoopbackCredentialProxy;

#[tokio::test]
async fn test_loopback_proxy_binding_and_teardown() {
    let secret_store: Arc<dyn SecretStore> = Arc::new(EnvSecretStore::new());
    let proxy = LoopbackCredentialProxy::new(secret_store).with_base_port(19000);

    let session_id = "test-session-uuid-1";
    let binding = proxy.bind_proxy(session_id).await.unwrap();

    assert_eq!(binding.session_id, session_id);
    assert_eq!(binding.port, 19000);
    assert_eq!(binding.proxy_url, "http://127.0.0.1:19000");

    // Second session gets distinct port offset
    let binding2 = proxy.bind_proxy("test-session-uuid-2").await.unwrap();
    assert_eq!(binding2.port, 19001);

    // Teardown cleanly removes session
    proxy.teardown_proxy(session_id).await.unwrap();
}

#[tokio::test]
async fn test_loopback_proxy_transport_header_injection() {
    let secret_store: Arc<dyn SecretStore> = Arc::new(EnvSecretStore::new());
    secret_store
        .set_secret("GOOGLE_ACCESS_TOKEN", "ya29.secret_google_access_token")
        .await
        .unwrap();
    secret_store
        .set_secret("GITHUB_TOKEN", "ghp_secure_github_pat")
        .await
        .unwrap();

    let proxy = LoopbackCredentialProxy::new(secret_store);

    // Test Google header transformation
    let raw_headers = vec![
        ("Host".to_string(), "sheets.googleapis.com".to_string()),
        ("User-Agent".to_string(), "AegisSandbox/1.0".to_string()),
    ];

    let transformed = proxy
        .transform_request_headers("google", &raw_headers)
        .await
        .unwrap();

    let auth_header = transformed
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("authorization"));

    assert!(auth_header.is_some());
    assert_eq!(
        auth_header.unwrap().1,
        "Bearer ya29.secret_google_access_token"
    );

    // Ensure preexisting auth headers are stripped and replaced
    let spoofed_headers = vec![
        ("Authorization".to_string(), "Bearer fake_token".to_string()),
        ("Host".to_string(), "api.github.com".to_string()),
    ];

    let transformed_gh = proxy
        .transform_request_headers("github", &spoofed_headers)
        .await
        .unwrap();

    let auth_gh = transformed_gh
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("authorization"));
    assert_eq!(
        auth_gh.unwrap().1,
        "Bearer ghp_secure_github_pat"
    );
}

#[tokio::test]
async fn test_loopback_proxy_missing_credentials_fails_closed() {
    let secret_store: Arc<dyn SecretStore> = Arc::new(EnvSecretStore::new());
    let proxy = LoopbackCredentialProxy::new(secret_store);

    let raw_headers = vec![("Host".to_string(), "api.github.com".to_string())];
    let res = proxy.transform_request_headers("github", &raw_headers).await;

    assert!(res.is_err());
    let err_msg = res.err().unwrap().to_string();
    assert!(err_msg.contains("No credentials available"));
}
