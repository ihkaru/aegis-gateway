// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use super::types::{SearchError, SearchOptions, SearchResults};

/// Interface-first abstract contract for search engines (Exa, Tavily, Brave, Google, etc.)
/// Guarantees zero vendor lock-in per DIP (Dependency Inversion Principle).
#[async_trait]
pub trait WebSearchDriver: Send + Sync {
    /// Canonical provider name (e.g. "exa", "tavily", "brave")
    fn provider_name(&self) -> &str;

    /// Execute search query with options against this driver using the supplied API key
    async fn search(
        &self,
        query: &str,
        options: &SearchOptions,
        api_key: &str,
    ) -> Result<SearchResults, SearchError>;

    /// Check driver health and key validity
    async fn health_check(&self, api_key: &str) -> bool;
}
