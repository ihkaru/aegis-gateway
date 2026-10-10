// SPDX-License-Identifier: MIT

use serde_json::json;

use aegis_gateway::core::protocol_negotiation::{
    HeaderRoutingMetadata, McpProtocolNegotiator, McpProtocolVersion,
};
use aegis_gateway::transport::StatelessProtocolNegotiator;

#[test]
fn test_detect_protocol_version_header() {
    let negotiator = StatelessProtocolNegotiator::default();

    // 1. Modern MCP 2026-07-28 detection
    let v_modern = negotiator.detect_protocol_version(Some("2026-07-28"));
    assert_eq!(v_modern, McpProtocolVersion::Modern2026_07_28);
    assert_eq!(v_modern.as_str(), "2026-07-28");

    // 2. Legacy MCP 2024-11-05 backward compatibility
    let v_legacy = negotiator.detect_protocol_version(Some("2024-11-05"));
    assert_eq!(v_legacy, McpProtocolVersion::Legacy2024_11_05);
    assert_eq!(v_legacy.as_str(), "2024-11-05");

    // 3. Omitted header defaults to modern stateless
    let v_default = negotiator.detect_protocol_version(None);
    assert_eq!(v_default, McpProtocolVersion::Modern2026_07_28);
}

#[test]
fn test_anti_desync_header_body_validation() {
    let negotiator = StatelessProtocolNegotiator::default();

    // 1. Matching headers and payload -> Valid
    let valid_headers = HeaderRoutingMetadata {
        protocol_version: McpProtocolVersion::Modern2026_07_28,
        method: Some("tools/call".into()),
        name: Some("execute_code".into()),
    };
    let res_valid = negotiator.validate_anti_desync(&valid_headers, "tools/call", Some("execute_code"));
    assert!(res_valid.is_valid);
    assert!(res_valid.reason.is_none());

    // 2. Method mismatch -> Desync rejection
    let desync_method_headers = HeaderRoutingMetadata {
        protocol_version: McpProtocolVersion::Modern2026_07_28,
        method: Some("tools/list".into()),
        name: None,
    };
    let res_method_mismatch = negotiator.validate_anti_desync(&desync_method_headers, "tools/call", Some("execute_code"));
    assert!(!res_method_mismatch.is_valid);
    assert!(res_method_mismatch.reason.unwrap().contains("Protocol desync detected"));

    // 3. Tool name mismatch -> Desync rejection
    let desync_name_headers = HeaderRoutingMetadata {
        protocol_version: McpProtocolVersion::Modern2026_07_28,
        method: Some("tools/call".into()),
        name: Some("read_safe_file".into()),
    };
    let res_name_mismatch = negotiator.validate_anti_desync(&desync_name_headers, "tools/call", Some("destructive_delete"));
    assert!(!res_name_mismatch.is_valid);
    assert!(res_name_mismatch.reason.unwrap().contains("Mcp-Name 'read_safe_file' does not match"));
}

#[test]
fn test_negotiate_context_and_ttl_cache_injection() {
    let negotiator = StatelessProtocolNegotiator::new(300_000);

    // 1. Modern context negotiation
    let modern_ctx = negotiator.negotiate_context(&McpProtocolVersion::Modern2026_07_28);
    assert!(modern_ctx.is_stateless);
    assert_eq!(modern_ctx.emit_cache_ttl_ms, Some(300_000));

    // 2. Cache metadata injection into JSON-RPC tools/list
    let mut response_val = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "tools": [
                { "name": "search_gdrive", "description": "Search Drive files" }
            ]
        }
    });

    StatelessProtocolNegotiator::inject_cache_ttl(&mut response_val, 300_000);
    assert_eq!(response_val["result"]["ttlMs"], 300_000);

    // 3. Legacy context negotiation
    let legacy_ctx = negotiator.negotiate_context(&McpProtocolVersion::Legacy2024_11_05);
    assert!(!legacy_ctx.is_stateless);
    assert_eq!(legacy_ctx.emit_cache_ttl_ms, None);
}
