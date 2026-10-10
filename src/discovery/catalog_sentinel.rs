// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use crate::core::types::ToolDefinition;

/// Outcome of Catalog Sync evaluation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatalogSyncOutcome {
    /// Sync accepted and applied
    Accepted { server: String, tool_count: usize },
    /// Sync rejected because incoming catalog is empty but existing catalog is non-empty
    RefusedEmptySync {
        server: String,
        preserved_tool_count: usize,
        reason: String,
    },
    /// Explicit purge requested by administrative action
    ExplicitPurge { server: String },
}

/// Interface-First Trait for Catalog Drift and Zero-Tool Drop Protection (resolves docker/mcp-gateway#561)
pub trait CatalogSyncSentinel: Send + Sync {
    /// Evaluate an incoming batch of tools against currently indexed tools
    fn evaluate_sync(
        &self,
        server: &str,
        current_tools: &[ToolDefinition],
        incoming_tools: &[ToolDefinition],
        is_explicit_purge: bool,
    ) -> CatalogSyncOutcome;
}

/// Default Enterprise Catalog Sync Sentinel Implementation
#[derive(Debug, Clone)]
pub struct AegisCatalogSyncSentinel {
    allow_empty_sync: bool,
}

impl Default for AegisCatalogSyncSentinel {
    fn default() -> Self {
        Self {
            allow_empty_sync: false,
        }
    }
}

impl AegisCatalogSyncSentinel {
    pub fn new(allow_empty_sync: bool) -> Self {
        Self { allow_empty_sync }
    }
}

impl CatalogSyncSentinel for AegisCatalogSyncSentinel {
    fn evaluate_sync(
        &self,
        server: &str,
        current_tools: &[ToolDefinition],
        incoming_tools: &[ToolDefinition],
        is_explicit_purge: bool,
    ) -> CatalogSyncOutcome {
        if is_explicit_purge {
            return CatalogSyncOutcome::ExplicitPurge {
                server: server.to_string(),
            };
        }

        if incoming_tools.is_empty() && !current_tools.is_empty() && !self.allow_empty_sync {
            return CatalogSyncOutcome::RefusedEmptySync {
                server: server.to_string(),
                preserved_tool_count: current_tools.len(),
                reason: format!(
                    "Sentinel refused zero-tool drop for backend '{}': received 0 tools, preserving {} healthy tools",
                    server,
                    current_tools.len()
                ),
            };
        }

        CatalogSyncOutcome::Accepted {
            server: server.to_string(),
            tool_count: incoming_tools.len(),
        }
    }
}
