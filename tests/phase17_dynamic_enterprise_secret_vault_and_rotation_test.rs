// SPDX-License-Identifier: MIT

use std::sync::Arc;
use std::time::Duration;

use aegis_gateway::core::secrets::SecretStore;
use aegis_gateway::policy::infisical::{
    ChainedSecretStore, InfisicalSecretStore, LocalInfisicalTransport,
};
use aegis_gateway::policy::secrets::EnvSecretStore;

#[tokio::test]
async fn test_infisical_secret_store_cache_and_fetch() {
    let transport = Arc::new(
        LocalInfisicalTransport::new()
            .with_secret("AINA_GDRIVE_TOKEN_JSON", "{\"token\":\"google_secret_val\"}")
            .with_secret("GITHUB_TOKEN", "ghp_secure_pat_12345"),
    );

    let store = InfisicalSecretStore::new(
        "https://secrets.dvlpid.my.id/api",
        "mach_id_client_1",
        "mach_id_sec_1",
        "f13379e0-9661-4f8e-81ef-0e81d1502da1",
        "dev",
        transport.clone(),
    )
    .with_ttl(Duration::from_millis(50));

    // 1. First fetch triggers transport login & retrieval
    let secret = store.get_secret("AINA_GDRIVE_TOKEN_JSON").await.unwrap();
    assert_eq!(
        secret,
        Some("{\"token\":\"google_secret_val\"}".to_string())
    );

    // 2. Second fetch hits cache immediately
    let cached = store.get_secret("AINA_GDRIVE_TOKEN_JSON").await.unwrap();
    assert_eq!(cached, secret);

    // 3. Unknown key returns None
    let missing = store.get_secret("NON_EXISTENT_KEY").await.unwrap();
    assert!(missing.is_none());

    // 4. Invalidation purges cached entry
    store.invalidate("AINA_GDRIVE_TOKEN_JSON").await.unwrap();
    let re_fetched = store.get_secret("AINA_GDRIVE_TOKEN_JSON").await.unwrap();
    assert_eq!(re_fetched, secret);
}

#[tokio::test]
async fn test_infisical_cache_ttl_expiration() {
    let transport = Arc::new(
        LocalInfisicalTransport::new().with_secret("ROTATING_KEY", "initial_value"),
    );

    let store = InfisicalSecretStore::new(
        "https://secrets.dvlpid.my.id/api",
        "mach_id_client_1",
        "mach_id_sec_1",
        "proj_1",
        "dev",
        transport,
    )
    .with_ttl(Duration::from_millis(10));

    let val1 = store.get_secret("ROTATING_KEY").await.unwrap();
    assert_eq!(val1, Some("initial_value".to_string()));

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Should still resolve after expiration
    let val2 = store.get_secret("ROTATING_KEY").await.unwrap();
    assert_eq!(val2, Some("initial_value".to_string()));
}

#[tokio::test]
async fn test_chained_secret_store_fallback_hierarchy() {
    // Primary store has only GITHUB_TOKEN
    let primary_transport = Arc::new(
        LocalInfisicalTransport::new().with_secret("GITHUB_TOKEN", "ghp_primary"),
    );
    let primary = Arc::new(InfisicalSecretStore::new(
        "https://secrets.dvlpid.my.id/api",
        "cid",
        "csec",
        "proj",
        "dev",
        primary_transport,
    ));

    // Secondary fallback has AWS_KEY
    let secondary = Arc::new(EnvSecretStore::new());
    secondary
        .set_secret("AWS_ACCESS_KEY_ID", "AKIA_FALLBACK_VAL")
        .await
        .unwrap();

    let chained = ChainedSecretStore::new(vec![primary.clone(), secondary.clone()]);

    // Primary hit
    let gh = chained.get_secret("GITHUB_TOKEN").await.unwrap();
    assert_eq!(gh, Some("ghp_primary".to_string()));

    // Fallback hit
    let aws = chained.get_secret("AWS_ACCESS_KEY_ID").await.unwrap();
    assert_eq!(aws, Some("AKIA_FALLBACK_VAL".to_string()));

    // Completely missing key
    let unknown = chained.get_secret("MISSING_ANYWHERE").await.unwrap();
    assert!(unknown.is_none());

    // Health check returns true if at least one is healthy
    assert!(chained.health_check().await.unwrap());
}
