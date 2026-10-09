// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

//! E2E Tier 1: Dev Hobbyist Zero-Config Quickstart Test Suite
//! Persona: A single developer who clones the repo and wants the core MCP Gateway
//! running out-of-the-box in under 60 seconds with Claude Desktop / Cursor via Stdio.
//! Invariant: Zero external services (no Redis, no OPA, no Presidio), 100% clean stdout.

use std::sync::Arc;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use aegis_gateway::backend::TopologyConfigLoader;
use aegis_gateway::core::transport::WireProtocolHandler;
use aegis_gateway::core::types::ToolDefinition;
use aegis_gateway::transport::{McpProtocolHandler, StdioTransport};
use aegis_gateway::AegisGateway;

#[tokio::test]
async fn test_hobbyist_zero_config_initialization() {
    // Zero-config: 1-line creation with default in-memory drivers
    let gateway = Arc::new(AegisGateway::default());
    let handler = McpProtocolHandler::new(gateway);

    // 1. Initialize Handshake (Claude Desktop startup sequence)
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    let init_resp = handler
        .handle_message(&init_req.to_string())
        .await
        .expect("handler must succeed")
        .expect("response expected for initialize");

    let val: serde_json::Value = serde_json::from_str(&init_resp).expect("valid json-rpc");
    assert_eq!(val["id"], 1);
    assert_eq!(val["result"]["serverInfo"]["name"], "aegis-gateway");
    assert_eq!(val["result"]["protocolVersion"], "2024-11-05");

    // 2. Ping Healthcheck
    let ping_req = json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" });
    let ping_resp = handler
        .handle_message(&ping_req.to_string())
        .await
        .expect("ping must succeed")
        .expect("ping response");
    let ping_val: serde_json::Value = serde_json::from_str(&ping_resp).expect("valid ping");
    assert_eq!(ping_val["id"], 2);
}

#[tokio::test]
async fn test_hobbyist_stdio_clean_channel_and_tool_call() {
    // Register a simple hobbyist tool
    let protocol = McpProtocolHandler::new(Arc::new(AegisGateway::default()));
    protocol
        .register_tools(vec![ToolDefinition {
            name: "greet_user".to_string(),
            server: "local_scripts".to_string(),
            description: "Friendly greeting for developers".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "username": { "type": "string" }
                }
            }),
            required_params: vec![],
            when_to_use: "Greet the user".to_string(),
            tags: vec!["fun".to_string()],
        }])
        .await;

    let stdio_transport = StdioTransport::new(Arc::new(protocol));

    // Simulate bi-directional OS pipe (stdin / stdout)
    let (mut client_stdin_writer, server_stdin_reader) = tokio::io::duplex(4096);
    let (server_stdout_writer, mut client_stdout_reader) = tokio::io::duplex(4096);

    let server_task = tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(server_stdin_reader);
        stdio_transport
            .process_stream(reader, server_stdout_writer)
            .await
    });

    // 1. Send initialize
    client_stdin_writer
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":10,\"method\":\"initialize\",\"params\":{}}\n")
        .await
        .expect("write init");
    client_stdin_writer.flush().await.expect("flush");

    let mut line_reader = tokio::io::BufReader::new(&mut client_stdout_reader);
    let mut line = String::new();
    line_reader.read_line(&mut line).await.expect("read line");
    assert!(line.contains("\"result\":{\"capabilities\""));

    // 2. Send initialized notification (must produce 0 bytes on stdout)
    client_stdin_writer
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .expect("write notif");
    client_stdin_writer.flush().await.expect("flush");

    // 3. Send tools/list
    client_stdin_writer
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":11,\"method\":\"tools/list\"}\n")
        .await
        .expect("write list");
    client_stdin_writer.flush().await.expect("flush");

    line.clear();
    line_reader.read_line(&mut line).await.expect("read tools list");
    let list_val: serde_json::Value = serde_json::from_str(&line).expect("clean json on stdout");
    assert_eq!(list_val["id"], 11);
    assert_eq!(list_val["result"]["tools"][0]["name"], "greet_user");

    // 4. Send tools/call
    client_stdin_writer
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":12,\"method\":\"tools/call\",\"params\":{\"name\":\"greet_user\",\"arguments\":{\"username\":\"Alice\"}}}\n")
        .await
        .expect("write call");
    client_stdin_writer.flush().await.expect("flush");

    line.clear();
    line_reader.read_line(&mut line).await.expect("read tools call");
    let call_val: serde_json::Value = serde_json::from_str(&line).expect("clean call json");
    assert_eq!(call_val["id"], 12);
    assert_eq!(call_val["result"]["isError"], false);

    drop(client_stdin_writer);
    let _ = server_task.await;
}

#[tokio::test]
async fn test_hobbyist_minimal_config_loading() {
    // Developer uses a quick 4-line yaml without complex schemas
    let quick_yaml = r#"
mcpServers:
  echo:
    command: "echo"
    args: ["hello"]
"#;

    let config = TopologyConfigLoader::load_from_str(quick_yaml).expect("parse minimal yaml");
    assert_eq!(config.mcp_servers.len(), 1);
    let echo_server = config.mcp_servers.get("echo").expect("server exists");
    assert_eq!(echo_server.name, "echo");
    assert_eq!(echo_server.command.as_deref(), Some("echo"));
    assert_eq!(echo_server.timeout_secs, 30); // sensible default
    assert!(echo_server.enabled); // sensible default
}
