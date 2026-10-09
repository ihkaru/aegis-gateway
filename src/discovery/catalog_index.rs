// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::types::{DisclosureTier, ProjectedTool, ToolDefinition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendIndexStatus {
    Indexed,
    Initializing,
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCatalogMetadata {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub status: BackendIndexStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogSearchResult {
    pub query: String,
    pub tools: Vec<ProjectedTool>,
    pub total_matches: usize,
    pub indexed_backends: Vec<String>,
    pub unindexed_backends: Vec<String>,
}

/// Dynamic, high-volume deep search index for tools & backends (MikkoParkkola/mcp-gateway#3034)
#[derive(Debug, Clone, Default)]
pub struct CatalogSearchIndex {
    backends: HashMap<String, BackendCatalogMetadata>,
    tools: Vec<ToolDefinition>,
}

impl CatalogSearchIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or update backend metadata
    pub fn register_backend(&mut self, metadata: BackendCatalogMetadata) {
        self.backends.insert(metadata.name.clone(), metadata);
    }

    /// Mark a backend as initializing / pending resolution
    pub fn mark_backend_initializing(&mut self, server: &str, description: &str) {
        self.backends.insert(
            server.to_string(),
            BackendCatalogMetadata {
                name: server.to_string(),
                description: description.to_string(),
                tags: Vec::new(),
                status: BackendIndexStatus::Initializing,
            },
        );
    }

    /// Dynamically register or refresh a full batch of tools for a backend (e.g. dbhub 44 dynamic tools)
    pub fn index_tools_for_backend(&mut self, server: &str, new_tools: Vec<ToolDefinition>) {
        // Remove existing tools for this backend
        self.tools.retain(|t| t.server != server);

        // Append new tools
        self.tools.extend(new_tools);

        // Mark backend as Indexed
        if let Some(meta) = self.backends.get_mut(server) {
            meta.status = BackendIndexStatus::Indexed;
        } else {
            self.backends.insert(
                server.to_string(),
                BackendCatalogMetadata {
                    name: server.to_string(),
                    description: format!("Server {}", server),
                    tags: Vec::new(),
                    status: BackendIndexStatus::Indexed,
                },
            );
        }
    }

    pub fn total_tools(&self) -> usize {
        self.tools.len()
    }

    pub fn backend_tool_count(&self, server: &str) -> usize {
        self.tools.iter().filter(|t| t.server == server).count()
    }

    /// Performs deep search matching over tool names, descriptions, schemas, AND backend descriptions
    pub fn search(&self, query: &str, tier: DisclosureTier, top_k: usize) -> CatalogSearchResult {
        let query_lower = query.to_lowercase();
        let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();

        let mut unindexed = Vec::new();
        let mut indexed = Vec::new();

        for (name, meta) in &self.backends {
            if meta.status == BackendIndexStatus::Initializing {
                unindexed.push(name.clone());
            } else if meta.status == BackendIndexStatus::Indexed {
                indexed.push(name.clone());
            }
        }

        let mut scored_tools: Vec<(f64, &ToolDefinition)> = self
            .tools
            .iter()
            .filter_map(|t| {
                let name_lower = t.name.to_lowercase();
                let desc_lower = t.description.to_lowercase();
                let server_lower = t.server.to_lowercase();
                let schema_str = t.input_schema.to_string().to_lowercase();

                let backend_meta = self.backends.get(&t.server);
                let backend_desc_lower = backend_meta
                    .map(|m| m.description.to_lowercase())
                    .unwrap_or_default();

                let mut score: f64 = 0.0;
                let mut matched = false;

                for token in &query_tokens {
                    let mut token_matched = false;

                    if name_lower.contains(token) {
                        score += 5.0;
                        token_matched = true;
                    }
                    if server_lower.contains(token) {
                        score += 4.0;
                        token_matched = true;
                    }
                    if desc_lower.contains(token) {
                        score += 2.0;
                        token_matched = true;
                    }
                    if backend_desc_lower.contains(token) {
                        score += 3.5;
                        token_matched = true;
                    }
                    if schema_str.contains(token) {
                        score += 1.5;
                        token_matched = true;
                    }
                    for tag in &t.tags {
                        if tag.to_lowercase().contains(token) {
                            score += 2.5;
                            token_matched = true;
                        }
                    }

                    if token_matched {
                        matched = true;
                    }
                }

                if matched || query_tokens.is_empty() {
                    Some((score.max(0.1), t))
                } else {
                    None
                }
            })
            .collect();

        scored_tools.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let total_matches = scored_tools.len();

        let projected = scored_tools
            .into_iter()
            .take(top_k)
            .map(|(score, t)| {
                let summary = if t.description.len() > 120 {
                    format!("{}...", &t.description[..117])
                } else {
                    t.description.clone()
                };

                match tier {
                    DisclosureTier::L0 => ProjectedTool {
                        name: t.name.clone(),
                        server: t.server.clone(),
                        tier: DisclosureTier::L0,
                        summary,
                        signature: None,
                        required_params: None,
                        when_to_use: None,
                        input_schema: None,
                        score,
                    },
                    DisclosureTier::L1 => ProjectedTool {
                        name: t.name.clone(),
                        server: t.server.clone(),
                        tier: DisclosureTier::L1,
                        summary,
                        signature: Some(format!("{}({})", t.name, t.required_params.join(", "))),
                        required_params: Some(t.required_params.clone()),
                        when_to_use: Some(t.when_to_use.clone()),
                        input_schema: None,
                        score,
                    },
                    DisclosureTier::L2 => ProjectedTool {
                        name: t.name.clone(),
                        server: t.server.clone(),
                        tier: DisclosureTier::L2,
                        summary,
                        signature: Some(format!("{}({})", t.name, t.required_params.join(", "))),
                        required_params: Some(t.required_params.clone()),
                        when_to_use: Some(t.when_to_use.clone()),
                        input_schema: Some(t.input_schema.clone()),
                        score,
                    },
                }
            })
            .collect();

        CatalogSearchResult {
            query: query.to_string(),
            tools: projected,
            total_matches,
            indexed_backends: indexed,
            unindexed_backends: unindexed,
        }
    }
}
