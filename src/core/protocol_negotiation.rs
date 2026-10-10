// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpProtocolVersion {
    Legacy2024_11_05,
    Modern2026_07_28,
    Unknown(String),
}

impl McpProtocolVersion {
    pub fn as_str(&self) -> &str {
        match self {
            McpProtocolVersion::Legacy2024_11_05 => "2024-11-05",
            McpProtocolVersion::Modern2026_07_28 => "2026-07-28",
            McpProtocolVersion::Unknown(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderRoutingMetadata {
    pub protocol_version: McpProtocolVersion,
    pub method: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AntiDesyncValidationResult {
    pub is_valid: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiatedMcpContext {
    pub version: McpProtocolVersion,
    pub is_stateless: bool,
    pub emit_cache_ttl_ms: Option<u64>,
}

/// Interface-First abstraction for Model Context Protocol 2026-07-28 & Dual-Stack Negotiation
#[async_trait]
pub trait McpProtocolNegotiator: Send + Sync {
    fn detect_protocol_version(&self, header_value: Option<&str>) -> McpProtocolVersion;
    fn validate_anti_desync(
        &self,
        headers: &HeaderRoutingMetadata,
        body_method: &str,
        body_tool_name: Option<&str>,
    ) -> AntiDesyncValidationResult;
    fn negotiate_context(&self, version: &McpProtocolVersion) -> NegotiatedMcpContext;
}
