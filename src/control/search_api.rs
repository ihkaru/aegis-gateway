// SPDX-License-Identifier: MIT

use std::sync::Arc;
use serde_json::json;

use crate::PropylonGateway;
use crate::search::types::{SearchOptions, SearchType, SelectionStrategy};

/// REST API Dispatcher for Web Search Engine & Key Pool Management
pub struct SearchApiDispatcher;

impl SearchApiDispatcher {
    pub async fn dispatch(
        method: &str,
        path: &str,
        body_bytes: &[u8],
        gateway: Option<&Arc<PropylonGateway>>,
    ) -> (u16, Vec<u8>, &'static str) {
        let gw = match gateway {
            Some(g) => g,
            None => {
                return (
                    503,
                    json!({ "error": "Propylon Gateway orchestrator not available" })
                        .to_string()
                        .into_bytes(),
                    "application/json",
                );
            }
        };

        let search_mgr = gw.search_manager();

        match (method, path) {
            ("GET", "/api/v1/search/keys") => {
                let keys = search_mgr.list_keys().await;
                let strategy = search_mgr.strategy().await;
                let payload = json!({
                    "provider": search_mgr.driver_name(),
                    "strategy": strategy,
                    "total_keys": keys.len(),
                    "active_keys": keys.iter().filter(|k| k.active && !k.in_cooldown).count(),
                    "keys": keys
                });
                (200, payload.to_string().into_bytes(), "application/json")
            }
            ("POST", "/api/v1/search/keys") => {
                let parsed = match serde_json::from_slice::<serde_json::Value>(body_bytes) {
                    Ok(p) => p,
                    Err(_) => {
                        return (400, json!({ "error": "Invalid JSON body" }).to_string().into_bytes(), "application/json");
                    }
                };
                let raw_key = match parsed.get("key").and_then(|k| k.as_str()) {
                    Some(k) if !k.trim().is_empty() => k.trim(),
                    _ => {
                        return (400, json!({ "error": "Field 'key' is required" }).to_string().into_bytes(), "application/json");
                    }
                };
                let label = parsed.get("label").and_then(|l| l.as_str()).map(|s| s.to_string());
                let id = search_mgr.add_key(raw_key, label).await;
                (201, json!({ "status": "created", "id": id }).to_string().into_bytes(), "application/json")
            }
            (method, path) if method == "DELETE" && path.starts_with("/api/v1/search/keys/") => {
                let id = path.trim_start_matches("/api/v1/search/keys/");
                let removed = search_mgr.remove_key(id).await;
                if removed {
                    (200, json!({ "status": "revoked", "id": id }).to_string().into_bytes(), "application/json")
                } else {
                    (404, json!({ "error": "Key ID not found in pool" }).to_string().into_bytes(), "application/json")
                }
            }
            (method, path) if method == "PUT" && path.starts_with("/api/v1/search/keys/") && path.ends_with("/toggle") => {
                let without_suffix = path.trim_end_matches("/toggle");
                let id = without_suffix.trim_start_matches("/api/v1/search/keys/");
                let parsed = serde_json::from_slice::<serde_json::Value>(body_bytes).unwrap_or_default();
                let active = parsed.get("active").and_then(|a| a.as_bool()).unwrap_or(true);
                let toggled = search_mgr.toggle_key(id, active).await;
                if toggled {
                    (200, json!({ "status": "updated", "id": id, "active": active }).to_string().into_bytes(), "application/json")
                } else {
                    (404, json!({ "error": "Key ID not found" }).to_string().into_bytes(), "application/json")
                }
            }
            ("PUT", "/api/v1/search/strategy") => {
                let parsed = serde_json::from_slice::<serde_json::Value>(body_bytes).unwrap_or_default();
                let strat = match parsed.get("strategy").and_then(|s| s.as_str()) {
                    Some("least_used") => SelectionStrategy::LeastUsed,
                    _ => SelectionStrategy::RoundRobin,
                };
                search_mgr.set_strategy(strat).await;
                (200, json!({ "status": "updated", "strategy": strat }).to_string().into_bytes(), "application/json")
            }
            ("POST", "/api/v1/search/query") => {
                let parsed = match serde_json::from_slice::<serde_json::Value>(body_bytes) {
                    Ok(p) => p,
                    Err(_) => {
                        return (400, json!({ "error": "Invalid JSON body" }).to_string().into_bytes(), "application/json");
                    }
                };
                let query = match parsed.get("query").and_then(|q| q.as_str()) {
                    Some(q) if !q.trim().is_empty() => q.trim(),
                    _ => {
                        return (400, json!({ "error": "Field 'query' is required" }).to_string().into_bytes(), "application/json");
                    }
                };
                let limit = parsed.get("limit").and_then(|l| l.as_u64()).unwrap_or(5) as usize;
                let search_type = match parsed.get("search_type").and_then(|s| s.as_str()) {
                    Some("neural") => SearchType::Neural,
                    Some("keyword") => SearchType::Keyword,
                    _ => SearchType::Auto,
                };
                let opts = SearchOptions {
                    limit: limit.clamp(1, 10),
                    search_type,
                    include_contents: true,
                    ..Default::default()
                };
                match search_mgr.search(query, &opts).await {
                    Ok(res) => (200, json!(res).to_string().into_bytes(), "application/json"),
                    Err(e) => (502, json!({ "error": e.to_string() }).to_string().into_bytes(), "application/json"),
                }
            }
            _ => (404, json!({ "error": "Endpoint not found" }).to_string().into_bytes(), "application/json"),
        }
    }
}
