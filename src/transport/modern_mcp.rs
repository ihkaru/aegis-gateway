// SPDX-License-Identifier: MIT

use serde_json::Value;

use crate::core::protocol_negotiation::{
    AntiDesyncValidationResult, HeaderRoutingMetadata, McpProtocolNegotiator,
    McpProtocolVersion, NegotiatedMcpContext,
};

/// Production Dual-Stack MCP Protocol Negotiator supporting MCP 2026-07-28 & 2024-11-05
pub struct StatelessProtocolNegotiator {
    default_ttl_ms: u64,
}

impl Default for StatelessProtocolNegotiator {
    fn default() -> Self {
        Self::new(300_000) // Default 5 minutes cache TTL
    }
}

impl StatelessProtocolNegotiator {
    pub fn new(default_ttl_ms: u64) -> Self {
        Self { default_ttl_ms }
    }

    /// Injects RFC/MCP 2026-07-28 cache metadata into tools/list response
    pub fn inject_cache_ttl(response_val: &mut Value, ttl_ms: u64) {
        if let Some(obj) = response_val.as_object_mut() {
            if let Some(result) = obj.get_mut("result").and_then(|r| r.as_object_mut()) {
                result.insert("ttlMs".into(), Value::from(ttl_ms));
            }
        }
    }
}

impl McpProtocolNegotiator for StatelessProtocolNegotiator {
    fn detect_protocol_version(&self, header_value: Option<&str>) -> McpProtocolVersion {
        match header_value {
            Some(v) if v.contains("2026-07-28") => McpProtocolVersion::Modern2026_07_28,
            Some(v) if v.contains("2024-11-05") => McpProtocolVersion::Legacy2024_11_05,
            Some(other) if other.trim().is_empty() => McpProtocolVersion::Modern2026_07_28,
            Some(other) => McpProtocolVersion::Unknown(other.to_string()),
            None => McpProtocolVersion::Modern2026_07_28, // Modern stateless by default
        }
    }

    fn validate_anti_desync(
        &self,
        headers: &HeaderRoutingMetadata,
        body_method: &str,
        body_tool_name: Option<&str>,
    ) -> AntiDesyncValidationResult {
        // 1. Validate Method Match
        if let Some(ref header_method) = headers.method {
            if header_method != body_method {
                return AntiDesyncValidationResult {
                    is_valid: false,
                    reason: Some(format!(
                        "Protocol desync detected: HTTP header Mcp-Method '{}' does not match JSON-RPC body method '{}'",
                        header_method, body_method
                    )),
                };
            }
        }

        // 2. Validate Tool Name Match (for tools/call)
        if let Some(ref header_name) = headers.name {
            if let Some(tool_name) = body_tool_name {
                if header_name != tool_name {
                    return AntiDesyncValidationResult {
                        is_valid: false,
                        reason: Some(format!(
                            "Protocol desync detected: HTTP header Mcp-Name '{}' does not match JSON-RPC payload name '{}'",
                            header_name, tool_name
                        )),
                    };
                }
            }
        }

        AntiDesyncValidationResult {
            is_valid: true,
            reason: None,
        }
    }

    fn negotiate_context(&self, version: &McpProtocolVersion) -> NegotiatedMcpContext {
        match version {
            McpProtocolVersion::Modern2026_07_28 => NegotiatedMcpContext {
                version: McpProtocolVersion::Modern2026_07_28,
                is_stateless: true,
                emit_cache_ttl_ms: Some(self.default_ttl_ms),
            },
            McpProtocolVersion::Legacy2024_11_05 => NegotiatedMcpContext {
                version: McpProtocolVersion::Legacy2024_11_05,
                is_stateless: false,
                emit_cache_ttl_ms: None,
            },
            McpProtocolVersion::Unknown(raw) => NegotiatedMcpContext {
                version: McpProtocolVersion::Unknown(raw.clone()),
                is_stateless: true,
                emit_cache_ttl_ms: None,
            },
        }
    }
}
