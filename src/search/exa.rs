// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Instant;

use super::driver::WebSearchDriver;
use super::types::{SearchError, SearchOptions, SearchResultItem, SearchResults, SearchType};

const EXA_SEARCH_URL: &str = "https://api.exa.ai/search";

/// Production Exa AI Search Driver
pub struct ExaSearchDriver {
    client: reqwest::Client,
    base_url: String,
}

impl ExaSearchDriver {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .unwrap_or_default(),
            base_url: EXA_SEARCH_URL.to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl Default for ExaSearchDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebSearchDriver for ExaSearchDriver {
    fn provider_name(&self) -> &str {
        "exa"
    }

    async fn search(
        &self,
        query: &str,
        options: &SearchOptions,
        api_key: &str,
    ) -> Result<SearchResults, SearchError> {
        let start = Instant::now();
        let search_type_str = match options.search_type {
            SearchType::Auto => "auto",
            SearchType::Neural => "neural",
            SearchType::Keyword => "keyword",
        };

        let mut payload = json!({
            "query": query,
            "type": search_type_str,
            "numResults": options.limit.clamp(1, 20),
        });

        if options.include_contents {
            payload["contents"] = json!({
                "highlights": true
            });
        }

        if let Some(cat) = &options.category {
            payload["category"] = json!(cat);
        }
        if !options.include_domains.is_empty() {
            payload["includeDomains"] = json!(options.include_domains);
        }
        if !options.exclude_domains.is_empty() {
            payload["excludeDomains"] = json!(options.exclude_domains);
        }

        let resp = self
            .client
            .post(&self.base_url)
            .header("x-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("Accept-Encoding", "gzip, deflate, identity")
            .json(&payload)
            .send()
            .await
            .map_err(|e| SearchError::Network(e.to_string()))?;

        let status = resp.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(SearchError::RateLimited(
                "Exa API rate limit or monthly quota reached (HTTP 429)".to_string(),
            ));
        } else if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(SearchError::Unauthorized(
                "Exa API key is invalid or revoked (HTTP 401/403)".to_string(),
            ));
        } else if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(SearchError::Provider(format!(
                "Exa API returned HTTP {status}: {err_text}"
            )));
        }

        let raw_bytes = resp
            .bytes()
            .await
            .map_err(|e| SearchError::Provider(format!("Failed to read Exa response bytes: {e}")))?;

        let body: Value = if raw_bytes.starts_with(&[0x1f, 0x8b]) {
            use std::io::Read;
            let mut decoder = flate2::read::GzDecoder::new(&raw_bytes[..]);
            let mut decompressed = String::new();
            decoder.read_to_string(&mut decompressed).map_err(|e| {
                SearchError::Provider(format!("Failed to decompress gzip Exa response: {e}"))
            })?;
            serde_json::from_str(&decompressed).map_err(|e| {
                SearchError::Provider(format!("Failed to parse decompressed Exa JSON: {e}"))
            })?
        } else {
            serde_json::from_slice(&raw_bytes)
                .map_err(|e| SearchError::Provider(format!("Failed to parse Exa JSON response: {e}")))?
        };

        let mut items = Vec::new();
        if let Some(raw_results) = body.get("results").and_then(|r| r.as_array()) {
            for r in raw_results {
                let id = r.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let url = r.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let title = r.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let published_date = r
                    .get("publishedDate")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let author = r.get("author").and_then(|v| v.as_str()).map(|s| s.to_string());
                let score = r.get("score").and_then(|v| v.as_f64());

                let mut highlights = Vec::new();
                if let Some(hl_arr) = r.get("highlights").and_then(|h| h.as_array()) {
                    for h in hl_arr {
                        if let Some(s) = h.as_str() {
                            highlights.push(s.to_string());
                        }
                    }
                }
                let text = r.get("text").and_then(|v| v.as_str()).map(|s| s.to_string());

                items.push(SearchResultItem {
                    id,
                    url,
                    title,
                    highlights,
                    text,
                    published_date,
                    author,
                    score,
                });
            }
        }

        let cost_dollars = body
            .get("costDollars")
            .and_then(|c| c.get("total"))
            .and_then(|t| t.as_f64());

        let search_time_ms = start.elapsed().as_millis() as u64;

        Ok(SearchResults {
            query: query.to_string(),
            provider: "exa".to_string(),
            results: items,
            cost_dollars,
            search_time_ms,
        })
    }

    async fn health_check(&self, api_key: &str) -> bool {
        let dummy_opts = SearchOptions {
            limit: 1,
            include_contents: false,
            ..Default::default()
        };
        self.search("health check", &dummy_opts, api_key).await.is_ok()
    }
}
