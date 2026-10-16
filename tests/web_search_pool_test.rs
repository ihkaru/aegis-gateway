// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use propylon::core::secrets::SecretStore;
use propylon::policy::secrets::EnvSecretStore;
use propylon::search::driver::WebSearchDriver;
use propylon::search::manager::WebSearchManager;
use propylon::search::pool::{mask_secret, SearchKeyPool};
use propylon::search::types::{
    SearchError, SearchOptions, SearchResultItem, SearchResults, SelectionStrategy,
};

#[test]
fn test_mask_secret() {
    assert_eq!(mask_secret("b436b248-b64d-434d-8db6-e32dcf296a12"), "b436...6a12");
    assert_eq!(mask_secret("short"), "********");
    assert_eq!(mask_secret("12345678"), "********");
    assert_eq!(mask_secret("123456789"), "1234...6789");
}

#[tokio::test]
async fn test_pool_round_robin_selection() {
    let mut pool = SearchKeyPool::new();
    pool.set_strategy(SelectionStrategy::RoundRobin);

    let id1 = pool.add_key("key-alpha-12345678", Some("Alpha".to_string()));
    let id2 = pool.add_key("key-beta-12345678", Some("Beta".to_string()));
    let id3 = pool.add_key("key-gamma-12345678", Some("Gamma".to_string()));

    assert_eq!(pool.key_count(), 3);
    assert_eq!(pool.available_key_count(), 3);

    // Cycle through all 3
    let (k1_id, _) = pool.acquire_key().unwrap();
    let (k2_id, _) = pool.acquire_key().unwrap();
    let (k3_id, _) = pool.acquire_key().unwrap();
    let (k4_id, _) = pool.acquire_key().unwrap();

    assert_eq!(k1_id, id1);
    assert_eq!(k2_id, id2);
    assert_eq!(k3_id, id3);
    assert_eq!(k4_id, id1); // wrapped around
}

#[tokio::test]
async fn test_pool_least_used_selection() {
    let mut pool = SearchKeyPool::new();
    pool.set_strategy(SelectionStrategy::LeastUsed);

    let id1 = pool.add_key("key-111111111111", Some("Key 1".to_string()));
    let id2 = pool.add_key("key-222222222222", Some("Key 2".to_string()));

    // Record 3 successes for key 1
    pool.record_success(&id1);
    pool.record_success(&id1);
    pool.record_success(&id1);

    // Key 2 should be selected because usage is 0
    let (selected_id, _) = pool.acquire_key().unwrap();
    assert_eq!(selected_id, id2);

    pool.record_success(&id2);
    // Key 2 still has usage 1 < 3
    let (selected_id_2, _) = pool.acquire_key().unwrap();
    assert_eq!(selected_id_2, id2);
}

#[tokio::test]
async fn test_pool_rate_limit_cooldown_and_failover() {
    let mut pool = SearchKeyPool::new();
    let id1 = pool.add_key("key-rate-limited-1", Some("Key 1".to_string()));
    let id2 = pool.add_key("key-healthy-22222", Some("Key 2".to_string()));

    // Simulate key 1 hitting 429
    pool.record_rate_limit(&id1, Duration::from_secs(30));

    // Available count drops to 1
    assert_eq!(pool.available_key_count(), 1);

    // Acquire key should skip key 1 and return key 2
    let (selected_id, _) = pool.acquire_key().unwrap();
    assert_eq!(selected_id, id2);

    let keys = pool.list_keys();
    let k1 = keys.iter().find(|k| k.id == id1).unwrap();
    assert!(k1.in_cooldown);
    assert_eq!(k1.failure_count, 1);
}

#[tokio::test]
async fn test_pool_unauthorized_deactivation() {
    let mut pool = SearchKeyPool::new();
    let id1 = pool.add_key("key-invalid-credentials", Some("Bad Key".to_string()));
    let id2 = pool.add_key("key-valid-credentials-2", Some("Good Key".to_string()));

    // Simulate 401
    pool.record_unauthorized(&id1);

    let keys = pool.list_keys();
    let k1 = keys.iter().find(|k| k.id == id1).unwrap();
    assert!(!k1.active);

    // Key 1 is permanently inactive
    let (selected_id, _) = pool.acquire_key().unwrap();
    assert_eq!(selected_id, id2);
}

#[tokio::test]
async fn test_pool_load_from_secret_store() {
    let env_store = EnvSecretStore::new();
    env_store.set_secret("EXA_API_KEY_1", "test-key-number-1-abc").await.unwrap();
    env_store.set_secret("EXA_API_KEY_2", "test-key-number-2-def").await.unwrap();
    env_store.set_secret("EXA_API_KEYS", "test-key-batch-3,test-key-batch-4").await.unwrap();

    let mut pool = SearchKeyPool::new();
    let loaded = pool.load_from_secret_store(&env_store).await;
    assert_eq!(loaded, 4);
    assert_eq!(pool.key_count(), 4);

    let keys = pool.list_keys();
    assert!(keys.iter().any(|k| k.label.contains("Exa Key #1")));
    assert!(keys.iter().any(|k| k.label.contains("Exa Key #2")));
}

/// Channel adapter for testing WebSearchManager automatic failover
struct FailoverTestDriver;

#[async_trait]
impl WebSearchDriver for FailoverTestDriver {
    fn provider_name(&self) -> &str {
        "test-failover"
    }

    async fn search(
        &self,
        query: &str,
        _options: &SearchOptions,
        api_key: &str,
    ) -> Result<SearchResults, SearchError> {
        if api_key.contains("failing") {
            return Err(SearchError::RateLimited("Quota reached".to_string()));
        }
        Ok(SearchResults {
            query: query.to_string(),
            provider: "test-failover".to_string(),
            results: vec![SearchResultItem {
                id: "res-1".to_string(),
                url: "https://example.com/data".to_string(),
                title: "Example Title".to_string(),
                highlights: vec!["Relevant highlight text".to_string()],
                text: None,
                published_date: Some("2026-10-16".to_string()),
                author: None,
                score: Some(0.95),
            }],
            cost_dollars: Some(0.005),
            search_time_ms: 12,
        })
    }

    async fn health_check(&self, _api_key: &str) -> bool {
        true
    }
}

#[tokio::test]
async fn test_web_search_manager_automatic_failover() {
    let mut pool = SearchKeyPool::new();
    let k1 = pool.add_key("failing-key-429", Some("Key 1 (Rate Limited)".to_string()));
    let _k2 = pool.add_key("working-key-200", Some("Key 2 (Healthy)".to_string()));

    let driver = Arc::new(FailoverTestDriver);

    let manager = WebSearchManager::new(driver, Arc::new(RwLock::new(pool)));

    let opts = SearchOptions::default();
    let res = manager.search("economic census 2026", &opts).await.unwrap();

    assert_eq!(res.provider, "test-failover");
    assert_eq!(res.results.len(), 1);
    assert_eq!(res.results[0].title, "Example Title");

    // Key 1 should now be marked in cooldown
    let keys = manager.list_keys().await;
    let k1_meta = keys.iter().find(|k| k.id == k1).unwrap();
    assert!(k1_meta.in_cooldown);
    assert_eq!(k1_meta.failure_count, 1);
}
