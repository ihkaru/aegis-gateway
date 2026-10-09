// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use serde_json::json;

use aegis_gateway::audit::StructuredAuditLogger;
use aegis_gateway::core::transport::{
    JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, WireProtocolHandler,
    INVALID_PARAMS, METHOD_NOT_FOUND, PARSE_ERROR,
};
use aegis_gateway::core::types::ToolDefinition;
use aegis_gateway::dlp::PiiDlpPipeline;
use aegis_gateway::policy::AbacPolicyEngine;
use aegis_gateway::skills::LocalSkillRegistry;
use aegis_gateway::state::InMemoryStateBackend;
use aegis_gateway::transport::{McpProtocolHandler, StdioTransport, StreamableHttpTransport};
use aegis_gateway::AegisGateway;

fn build_test_gateway() -> Arc<AegisGateway> {
    Arc::new(AegisGateway::new(
        Arc::new(InMemoryStateBackend::new()),
        Arc::new(AbacPolicyEngine::new()),
        Arc::new(PiiDlpPipeline::new()),
        Arc::new(StructuredAuditLogger::new()),
        Arc::new(LocalSkillRegistry::new()),
    ))
}

#[tokio::test]
async fn test_json_rpc_structures() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(42)),
        method: "ping".to_string(),
        params: None,
    };
    let json_str = serde_json::to_string(&req).expect("serialize req");
    let parsed: JsonRpcRequest = serde_json::from_str(&json_str).expect("deserialize req");
    assert_eq!(parsed.id, Some(JsonRpcId::Number(42)));
    assert_eq!(parsed.method, "ping");

    let err_resp = JsonRpcResponse::error(
        JsonRpcId::String("alpha".to_string()),
        JsonRpcError::with_data(INVALID_PARAMS, "Invalid params", json!({ "detail": "bad field" })),
    );
    let err_json = serde_json::to_string(&err_resp).expect("serialize err");
    let parsed_err: JsonRpcResponse = serde_json::from_str(&err_json).expect("deserialize err");
    assert!(parsed_err.result.is_none());
    assert_eq!(parsed_err.error.unwrap().code, INVALID_PARAMS);
}

#[tokio::test]
async fn test_mcp_wire_lifecycle_initialize_and_ping() {
    let gateway = build_test_gateway();
    let handler = McpProtocolHandler::new(gateway);

    // 1. Initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    let resp = handler
        .handle_message(&init_req.to_string())
        .await
        .expect("handle init")
        .expect("response expected");
    let val: serde_json::Value = serde_json::from_str(&resp).expect("valid json");
    assert_eq!(val["id"], 1);
    assert_eq!(val["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(val["result"]["serverInfo"]["name"], "aegis-gateway");

    // 2. Initialized Notification (should produce NO response)
    let notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let notif_resp = handler.handle_message(&notif.to_string()).await.expect("handle notif");
    assert!(notif_resp.is_none(), "Notifications must yield None response");

    // 3. Ping
    let ping_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ping"
    });
    let ping_resp = handler
        .handle_message(&ping_req.to_string())
        .await
        .expect("handle ping")
        .expect("ping response");
    let ping_val: serde_json::Value = serde_json::from_str(&ping_resp).expect("valid ping json");
    assert_eq!(ping_val["id"], 2);
    assert!(ping_val["result"].is_object());
}

#[tokio::test]
async fn test_mcp_wire_tools_list_and_call() {
    let gateway = build_test_gateway();
    let handler = McpProtocolHandler::new(gateway);

    handler
        .register_tools(vec![ToolDefinition {
            name: "query_database".to_string(),
            server: "postgres".to_string(),
            description: "Execute a read-only SQL query".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "sql": { "type": "string" }
                },
                "required": ["sql"]
            }),
            required_params: vec!["sql".to_string()],
            when_to_use: "Read database records".to_string(),
            tags: vec!["db".to_string()],
        }])
        .await;

    // 1. tools/list
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/list"
    });
    let list_resp = handler
        .handle_message(&list_req.to_string())
        .await
        .expect("tools list")
        .expect("list response");
    let list_val: serde_json::Value = serde_json::from_str(&list_resp).expect("parse list");
    let tools = list_val["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "query_database");

    // 2. tools/call
    let call_req = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "query_database",
            "arguments": {
                "sql": "SELECT 1;"
            }
        }
    });
    let call_resp = handler
        .handle_message(&call_req.to_string())
        .await
        .expect("tools call")
        .expect("call response");
    let call_val: serde_json::Value = serde_json::from_str(&call_resp).expect("parse call");
    assert_eq!(call_val["id"], 11);
    assert_eq!(call_val["result"]["isError"], false);
    assert!(call_val["result"]["content"].is_array());

    // 3. gateway_search_tools (meta-tool progressive disclosure)
    let search_req = json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "gateway_search_tools",
        "params": {
            "query": "query"
        }
    });
    let search_resp = handler
        .handle_message(&search_req.to_string())
        .await
        .expect("search tools")
        .expect("search response");
    let search_val: serde_json::Value = serde_json::from_str(&search_resp).expect("parse search");
    assert_eq!(search_val["id"], 12);
    let search_tools = search_val["result"]["tools"].as_array().expect("search result tools");
    assert_eq!(search_tools.len(), 1);
}

#[tokio::test]
async fn test_mcp_wire_error_handling() {
    let gateway = build_test_gateway();
    let handler = McpProtocolHandler::new(gateway);

    // 1. Invalid JSON Parse Error (-32700)
    let parse_resp = handler
        .handle_message("NOT VALID JSON {{{")
        .await
        .expect("handled parse error")
        .expect("parse err response");
    let parse_val: serde_json::Value = serde_json::from_str(&parse_resp).expect("valid parse err json");
    assert_eq!(parse_val["error"]["code"], PARSE_ERROR);

    // 2. Method Not Found (-32601)
    let unknown_req = json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "non_existent_method"
    });
    let unknown_resp = handler
        .handle_message(&unknown_req.to_string())
        .await
        .expect("handle unknown")
        .expect("unknown response");
    let unknown_val: serde_json::Value = serde_json::from_str(&unknown_resp).expect("unknown json");
    assert_eq!(unknown_val["error"]["code"], METHOD_NOT_FOUND);

    // 3. Invalid Params (-32602)
    let invalid_param_req = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "tools/call",
        "params": {
            "invalid_field": 123
        }
    });
    let param_resp = handler
        .handle_message(&invalid_param_req.to_string())
        .await
        .expect("handle invalid param")
        .expect("param response");
    let param_val: serde_json::Value = serde_json::from_str(&param_resp).expect("param json");
    assert_eq!(param_val["error"]["code"], INVALID_PARAMS);
}

#[tokio::test]
async fn test_stdio_transport_stream_processing() {
    let gateway = build_test_gateway();
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let stdio = StdioTransport::new(handler);

    let (mut client_write, server_read) = tokio::io::duplex(4096);
    let (server_write, mut client_read) = tokio::io::duplex(4096);

    let stdio_task = tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(server_read);
        stdio.process_stream(reader, server_write).await
    });

    // Write a ping request
    let ping_req = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";
    client_write.write_all(ping_req.as_bytes()).await.expect("write client");
    client_write.flush().await.expect("flush client");

    use tokio::io::AsyncBufReadExt;
    let mut resp_reader = tokio::io::BufReader::new(&mut client_read);
    let mut resp_line = String::new();
    resp_reader.read_line(&mut resp_line).await.expect("read response");

    let resp_val: serde_json::Value = serde_json::from_str(resp_line.trim()).expect("parse line");
    assert_eq!(resp_val["id"], 1);
    assert!(resp_val["result"].is_object());

    drop(client_write);
    let _ = stdio_task.await;
}

#[tokio::test]
async fn test_streamable_http_transport() {
    let gateway = build_test_gateway();
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let http = StreamableHttpTransport::new(handler, "127.0.0.1:8080");

    assert_eq!(http.listen_address(), "127.0.0.1:8080");

    // 1. Unified POST /mcp (RFC 2025-03-26)
    let ping_req = json!({
        "jsonrpc": "2.0",
        "id": "http-1",
        "method": "ping"
    });
    let (code, content_type, body) = http
        .process_http_request("/mcp", "POST", &ping_req.to_string())
        .await
        .expect("process http ping");
    assert_eq!(code, 200);
    assert_eq!(content_type, "application/json");
    let val: serde_json::Value = serde_json::from_str(&body).expect("parse body");
    assert_eq!(val["id"], "http-1");

    // 2. Notification POST /mcp yields 204 No Content
    let notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let (code, _, body) = http
        .process_http_request("/mcp", "POST", &notif.to_string())
        .await
        .expect("process http notif");
    assert_eq!(code, 204);
    assert!(body.is_empty());

    // 3. GET /sse Streamable connection
    let (code, content_type, body) = http
        .process_http_request("/sse", "GET", "")
        .await
        .expect("process sse get");
    assert_eq!(code, 200);
    assert_eq!(content_type, "text/event-stream");
    assert!(body.contains("endpoint"));

    // 4. GET /healthz Probe
    let (code, content_type, body) = http
        .process_http_request("/healthz", "GET", "")
        .await
        .expect("process healthz");
    assert_eq!(code, 200);
    assert_eq!(content_type, "application/json");
    assert!(body.contains("healthy"));

    // 5. Route Not Found
    let err = http
        .process_http_request("/unknown", "POST", "{}")
        .await;
    assert!(err.is_err());
}
