// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

/// Search execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    #[default]
    Auto,
    Neural,
    Keyword,
}

/// Parameters for web search invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub category: Option<String>,
    #[serde(default)]
    pub include_domains: Vec<String>,
    #[serde(default)]
    pub exclude_domains: Vec<String>,
    #[serde(default)]
    pub search_type: SearchType,
    #[serde(default = "default_true")]
    pub include_contents: bool,
}

fn default_limit() -> usize {
    5
}

fn default_true() -> bool {
    true
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            category: None,
            include_domains: Vec::new(),
            exclude_domains: Vec::new(),
            search_type: SearchType::Auto,
            include_contents: true,
        }
    }
}

/// An individual search hit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub id: String,
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub highlights: Vec<String>,
    pub text: Option<String>,
    pub published_date: Option<String>,
    pub author: Option<String>,
    pub score: Option<f64>,
}

/// Aggregated search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub query: String,
    pub provider: String,
    pub results: Vec<SearchResultItem>,
    pub cost_dollars: Option<f64>,
    pub search_time_ms: u64,
}

/// Strategy for selecting next active key from the pool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SelectionStrategy {
    #[default]
    RoundRobin,
    LeastUsed,
}

/// Zero-knowledge metadata for keys displayed via UI/API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskedKeyInfo {
    pub id: String,
    pub masked_key: String,
    pub label: String,
    pub active: bool,
    pub usage_count: u64,
    pub failure_count: u32,
    pub last_used_at: Option<String>,
    pub in_cooldown: bool,
}

/// Errors occurring during search driver or pool operations
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("API Rate limit exceeded or quota exhausted: {0}")]
    RateLimited(String),

    #[error("Authentication error (invalid or revoked key): {0}")]
    Unauthorized(String),

    #[error("Network connection error: {0}")]
    Network(String),

    #[error("Upstream search provider error: {0}")]
    Provider(String),

    #[error("No operational search keys available: {0}")]
    NoKeysAvailable(String),
}
