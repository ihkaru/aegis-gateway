// SPDX-License-Identifier: MIT

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::core::error::{PropylonError, PropylonResult};
use super::driver::WebSearchDriver;
use super::pool::SearchKeyPool;
use super::types::{MaskedKeyInfo, SearchError, SearchOptions, SearchResults, SelectionStrategy};

/// Enterprise Web Search Manager with Smart Key Pool and Zero-Vendor Lock-in
pub struct WebSearchManager {
    driver: Arc<dyn WebSearchDriver>,
    pool: Arc<RwLock<SearchKeyPool>>,
}

impl WebSearchManager {
    pub fn new(driver: Arc<dyn WebSearchDriver>, pool: Arc<RwLock<SearchKeyPool>>) -> Self {
        Self { driver, pool }
    }

    pub fn driver_name(&self) -> &str {
        self.driver.provider_name()
    }

    pub fn pool(&self) -> Arc<RwLock<SearchKeyPool>> {
        self.pool.clone()
    }

    /// Execute web search with automatic key rotation and resilient failover on 429
    pub async fn search(&self, query: &str, options: &SearchOptions) -> PropylonResult<SearchResults> {
        let max_attempts = {
            let pool_read = self.pool.read().await;
            pool_read.available_key_count().max(1)
        };

        let mut last_err = None;

        for attempt in 0..max_attempts {
            let key_opt = {
                let pool_read = self.pool.read().await;
                pool_read.acquire_key()
            };

            let (key_id, raw_key) = match key_opt {
                Some(k) => k,
                None => {
                    return Err(PropylonError::Internal(
                        "All search API keys in pool are currently rate-limited, inactive, or exhausted. Please add a new key or wait for cooldown.".to_string(),
                    ));
                }
            };

            tracing::debug!(
                query,
                key_id = %key_id,
                attempt = attempt + 1,
                provider = self.driver.provider_name(),
                "Executing web search via key pool"
            );

            match self.driver.search(query, options, &raw_key).await {
                Ok(results) => {
                    let mut pool_write = self.pool.write().await;
                    pool_write.record_success(&key_id);
                    return Ok(results);
                }
                Err(SearchError::RateLimited(msg)) => {
                    tracing::warn!(
                        key_id = %key_id,
                        error = %msg,
                        "Key hit rate-limit or quota exhausted, applying 60s cooldown and rotating"
                    );
                    let mut pool_write = self.pool.write().await;
                    pool_write.record_rate_limit(&key_id, Duration::from_secs(60));
                    last_err = Some(msg);
                }
                Err(SearchError::Unauthorized(msg)) => {
                    tracing::warn!(
                        key_id = %key_id,
                        error = %msg,
                        "Key unauthorized or revoked, disabling key and rotating"
                    );
                    let mut pool_write = self.pool.write().await;
                    pool_write.record_unauthorized(&key_id);
                    last_err = Some(msg);
                }
                Err(e) => {
                    return Err(PropylonError::Internal(format!(
                        "Search provider '{}' error: {}",
                        self.driver.provider_name(),
                        e
                    )));
                }
            }
        }

        Err(PropylonError::Internal(format!(
            "Web search failed after attempting all pool keys: {}",
            last_err.unwrap_or_else(|| "Unknown failure".to_string())
        )))
    }

    pub async fn list_keys(&self) -> Vec<MaskedKeyInfo> {
        let read = self.pool.read().await;
        read.list_keys()
    }

    pub async fn add_key(&self, raw_key: impl Into<String>, label: Option<String>) -> String {
        let mut write = self.pool.write().await;
        write.add_key(raw_key, label)
    }

    pub async fn remove_key(&self, id: &str) -> bool {
        let mut write = self.pool.write().await;
        write.remove_key(id)
    }

    pub async fn toggle_key(&self, id: &str, active: bool) -> bool {
        let mut write = self.pool.write().await;
        write.toggle_key(id, active)
    }

    pub async fn set_strategy(&self, strategy: SelectionStrategy) {
        let mut write = self.pool.write().await;
        write.set_strategy(strategy);
    }

    pub async fn strategy(&self) -> SelectionStrategy {
        let read = self.pool.read().await;
        read.strategy()
    }
}
