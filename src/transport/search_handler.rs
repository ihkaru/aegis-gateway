// SPDX-License-Identifier: MIT

use serde_json::{json, Value};

use crate::core::transport::{JsonRpcError, JsonRpcId, JsonRpcResponse, INVALID_PARAMS};
use crate::search::types::{SearchOptions, SearchType};
use crate::PropylonGateway;

pub struct SearchToolDispatcher;

impl SearchToolDispatcher {
    pub async fn handle_web_search(
        id: JsonRpcId,
        gateway: &PropylonGateway,
        params: Option<Value>,
    ) -> JsonRpcResponse {
        let args = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::new(INVALID_PARAMS, "Missing arguments for gateway_web_search"),
                );
            }
        };

        let query = match args.get("query").and_then(|q| q.as_str()) {
            Some(q) if !q.trim().is_empty() => q.trim(),
            _ => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::new(INVALID_PARAMS, "Parameter 'query' is required and must not be empty"),
                );
            }
        };

        let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(5) as usize;
        let category = args.get("category").and_then(|c| c.as_str()).map(|s| s.to_string());
        let search_type = match args.get("search_type").and_then(|s| s.as_str()) {
            Some("neural") => SearchType::Neural,
            Some("keyword") => SearchType::Keyword,
            _ => SearchType::Auto,
        };

        let options = SearchOptions {
            limit: limit.clamp(1, 10),
            category,
            search_type,
            include_contents: true,
            ..Default::default()
        };

        match gateway.search_manager().search(query, &options).await {
            Ok(results) => {
                let mut md = format!(
                    "### Web Search Results for \"{}\" (Engine: {}, Latency: {}ms)\n\n",
                    results.query, results.provider, results.search_time_ms
                );

                if results.results.is_empty() {
                    md.push_str("_No relevant web results found._\n");
                } else {
                    for (idx, item) in results.results.iter().enumerate() {
                        md.push_str(&format!("{}. **[{}]({})**\n", idx + 1, item.title, item.url));
                        if let Some(date) = &item.published_date {
                            md.push_str(&format!("   *Published: {}*\n", date));
                        }
                        for highlight in &item.highlights {
                            md.push_str(&format!("   > {}\n", highlight.trim()));
                        }
                        md.push('\n');
                    }
                }

                JsonRpcResponse::success(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": md.trim() }],
                        "structured": results
                    }),
                )
            }
            Err(e) => JsonRpcResponse::error(id, e.to_rpc_error()),
        }
    }
}
