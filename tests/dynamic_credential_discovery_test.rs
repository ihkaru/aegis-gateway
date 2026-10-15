// SPDX-License-Identifier: MIT

use serde_json::json;
use std::sync::Arc;

use propylon::core::sandbox::CredentialBroker;
use propylon::core::secrets::SecretStore;
use propylon::core::transport::{JsonRpcId, JsonRpcRequest, WireProtocolHandler};
use propylon::daemon::DaemonSupervisor;
use propylon::policy::secrets::EnvSecretStore;
use propylon::sandbox::VaultCredentialBroker;

#[tokio::test]
async fn test_dynamic_credential_discovery_and_subtoken_extraction() {
    // 1. Setup secret store with diverse keys (no hardcoded vendor assumptions)
    let secret_store = Arc::new(EnvSecretStore::new());
    secret_store
        .set_secret(
            "AINA_GDRIVE_TOKEN_JSON",
            r#"{"access_token":"mock_ya29_token_12345","refresh_token":"mock_1//refresh_token_67890","client_id":"mock_client_id.apps.googleusercontent.com","client_secret":"mock_gdrive_secret"}"#,
        )
        .await
        .unwrap();
    secret_store
        .set_secret("CLOUDFLARE_API_TOKEN", "mock_cf_token_abcdef")
        .await
        .unwrap();
    secret_store
        .set_secret("TALLY_API_KEY", "mock_tally_key_99999")
        .await
        .unwrap();

    // 2. Verify list_secret_keys metadata reflection (zero-knowledge)
    let keys = secret_store.list_secret_keys().await.unwrap();
    assert!(keys.contains(&"AINA_GDRIVE_TOKEN_JSON".to_string()));
    assert!(keys.contains(&"CLOUDFLARE_API_TOKEN".to_string()));
    assert!(keys.contains(&"TALLY_API_KEY".to_string()));

    // 3. Test broker with exact arbitrary key name
    let broker = VaultCredentialBroker::new(secret_store.clone());
    let envs = broker
        .broker_credentials(&["AINA_GDRIVE_TOKEN_JSON".to_string(), "CLOUDFLARE_API_TOKEN".to_string()])
        .await
        .unwrap();

    // Verify exact key is present
    assert!(envs.contains_key("AINA_GDRIVE_TOKEN_JSON"));
    assert_eq!(envs.get("CLOUDFLARE_API_TOKEN").unwrap(), "mock_cf_token_abcdef");

    // Verify automatic JSON subtoken extraction and Google aliasing
    assert_eq!(envs.get("GOOGLE_ACCESS_TOKEN").unwrap(), "mock_ya29_token_12345");
    assert_eq!(envs.get("GOOGLE_REFRESH_TOKEN").unwrap(), "mock_1//refresh_token_67890");
    assert_eq!(envs.get("GOOGLE_CLIENT_ID").unwrap(), "mock_client_id.apps.googleusercontent.com");
    assert_eq!(envs.get("GOOGLE_CLIENT_SECRET").unwrap(), "mock_gdrive_secret");

    // 4. Test broker with generic service name (e.g. "tally" matches "TALLY_API_KEY")
    let tally_envs = broker.broker_credentials(&["tally".to_string()]).await.unwrap();
    assert_eq!(tally_envs.get("TALLY_API_KEY").unwrap(), "mock_tally_key_99999");
}

#[tokio::test]
async fn test_gateway_list_credentials_over_mcp_wire() {
    let (_gateway, handler, _registry) = DaemonSupervisor::bootstrap(None).await.unwrap();

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(1001)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "gateway_list_credentials",
            "arguments": {}
        })),
    };

    let raw_req = serde_json::to_string(&req).unwrap();
    let resp_opt = handler.handle_message(&raw_req).await.unwrap();
    assert!(resp_opt.is_some());

    let resp_str = resp_opt.unwrap();
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
    assert!(resp.get("result").is_some());
    let result = &resp["result"];
    assert!(result.get("credentials").is_some());
    assert!(result["credentials"].is_array());
    assert!(result.get("usage").is_some());
}
