// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

//! E2E Tier 2: The Clumsy Developer / Negative & Boundary Test Suite
//! Persona: Inexperienced or careless developer who sends invalid JSON, truncates streams,
//! drops connections mid-request, or specifies broken configurations.
//! Invariant: System NEVER panics; standard JSON-RPC 2.0 error taxonomy is preserved.

use std::sync::Arc;
use serde_json::json;
use tokio::io::AsyncWriteExt;

use aegis_gateway::backend::TopologyConfigLoader;
use aegis_gateway::core::transport::{
    WireProtocolHandler, INVALID_PARAMS, METHOD_NOT_FOUND, PARSE_ERROR,
};
use aegis_gateway::transport::{McpProtocolHandler, StdioTransport};
use aegis_gateway::AegisGateway;

#[tokio::test]
async fn test_negative_malformed_json_inputs() {
    let gateway = Arc::new(AegisGateway::default());
    let handler = McpProtocolHandler::new(gateway);

    let garbage_cases = vec![
        "",                       // Empty string
        "   ",                    // Whitespace
        "NOT JSON AT ALL",        // Plain text
        "{\"jsonrpc\": \"2.0\"",  // Incomplete JSON object
        "{\"jsonrpc\": 2.0}",     // jsonrpc not a string
        "{[]}",                   // Syntax error
    ];

    for case in garbage_cases {
        let resp = handler.handle_message(case).await.expect("handler must not fail");
        assert!(resp.is_some(), "Malformed input should return a JSON-RPC error response");
        let val: serde_json::Value = serde_json::from_str(&resp.unwrap()).expect("parse error resp");
        assert_eq!(
            val["error"]["code"], PARSE_ERROR,
            "Must return standard PARSE_ERROR (-32700) for case: '{case}'"
        );
    }
}

#[tokio::test]
async fn test_negative_method_and_params_taxonomy() {
    let gateway = Arc::new(AegisGateway::default());
    let handler = McpProtocolHandler::new(gateway);

    // 1. Unknown Method -> METHOD_NOT_FOUND (-32601)
    let unknown_req = json!({
        "jsonrpc": "2.0",
        "id": 999,
        "method": "hack_the_gibson_now"
    });
    let resp = handler
        .handle_message(&unknown_req.to_string())
        .await
        .expect("handle unknown method")
        .expect("response expected");
    let val: serde_json::Value = serde_json::from_str(&resp).expect("parse val");
    assert_eq!(val["error"]["code"], METHOD_NOT_FOUND);
    assert_eq!(val["id"], 999);

    // 2. tools/call with missing params -> INVALID_PARAMS (-32602)
    let missing_params_req = json!({
        "jsonrpc": "2.0",
        "id": 1000,
        "method": "tools/call"
    });
    let resp2 = handler
        .handle_message(&missing_params_req.to_string())
        .await
        .expect("handle missing params")
        .expect("response expected");
    let val2: serde_json::Value = serde_json::from_str(&resp2).expect("parse val2");
    assert_eq!(val2["error"]["code"], INVALID_PARAMS);

    // 3. tools/call with params missing "name" field -> INVALID_PARAMS (-32602)
    let missing_name_req = json!({
        "jsonrpc": "2.0",
        "id": 1001,
        "method": "tools/call",
        "params": {
            "arguments": { "foo": "bar" }
        }
    });
    let resp3 = handler
        .handle_message(&missing_name_req.to_string())
        .await
        .expect("handle missing name")
        .expect("response expected");
    let val3: serde_json::Value = serde_json::from_str(&resp3).expect("parse val3");
    assert_eq!(val3["error"]["code"], INVALID_PARAMS);
}

#[tokio::test]
async fn test_negative_stream_abrupt_client_disconnect() {
    let gateway = Arc::new(AegisGateway::default());
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let stdio_transport = StdioTransport::new(handler);

    let (mut client_writer, server_reader) = tokio::io::duplex(1024);
    let (server_writer, _client_reader) = tokio::io::duplex(1024);

    let server_task = tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(server_reader);
        stdio_transport.process_stream(reader, server_writer).await
    });

    // Write incomplete JSON line and abruptly close pipe
    client_writer
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"meth")
        .await
        .expect("write partial");
    drop(client_writer); // Abrupt client crash/disconnect (EOF)

    // Server stream task must gracefully conclude without hanging or crashing
    let result = server_task.await.expect("task join");
    assert!(result.is_ok(), "Stream processor must handle abrupt EOF gracefully");
}

#[tokio::test]
async fn test_negative_invalid_topology_configuration() {
    // Bad config: neither command nor url provided
    let invalid_yaml = r#"
mcpServers:
  broken_server:
    timeout_secs: 10
"#;

    let cfg = TopologyConfigLoader::load_from_str(invalid_yaml).expect("parse yaml ok");
    let validation = TopologyConfigLoader::validate(&cfg);
    assert!(
        validation.is_err(),
        "Config validation must fail when neither command nor url is given"
    );
}
