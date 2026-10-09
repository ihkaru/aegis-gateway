// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::json;

use aegis_gateway::backend::{
    HermeticSubprocessBackend, RemoteHttpBackend, SubprocessBackendRegistry, TopologyConfigLoader,
};
use aegis_gateway::core::backend::{BackendConfig, BackendRegistry, BackendTransport};
use aegis_gateway::core::error::AegisResult;
use aegis_gateway::core::transport::{JsonRpcId, JsonRpcRequest, JsonRpcResponse};

/// Mock upstream backend transport for testing registry aggregation
struct MockBackendTransport {
    tools_response: JsonRpcResponse,
    healthy: AtomicBool,
}

impl MockBackendTransport {
    fn new(tools: Vec<serde_json::Value>) -> Self {
        Self {
            tools_response: JsonRpcResponse::success(
                JsonRpcId::Number(100),
                json!({ "tools": tools }),
            ),
            healthy: AtomicBool::new(true),
        }
    }
}

#[async_trait]
impl BackendTransport for MockBackendTransport {
    async fn start(&self) -> AegisResult<()> {
        self.healthy.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn send_request(&self, _request: &JsonRpcRequest) -> AegisResult<JsonRpcResponse> {
        Ok(self.tools_response.clone())
    }

    async fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::SeqCst)
    }

    async fn stop(&self) -> AegisResult<()> {
        self.healthy.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn test_topology_config_yaml_loading() {
    let yaml = r#"
mcpServers:
  postgres:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/db"]
    env:
      PG_MAX_CONN: "10"
    timeout_secs: 45
    enabled: true
  weather:
    url: "http://127.0.0.1:8081/sse"
    timeout_secs: 15
"#;

    let config = TopologyConfigLoader::load_from_str(yaml).expect("parse yaml");
    assert_eq!(config.mcp_servers.len(), 2);

    let pg = config.mcp_servers.get("postgres").expect("postgres config");
    assert_eq!(pg.command.as_deref(), Some("npx"));
    assert_eq!(pg.args.len(), 3);
    assert_eq!(pg.env.get("PG_MAX_CONN").map(|s: &String| s.as_str()), Some("10"));
    assert_eq!(pg.timeout_secs, 45);
    assert!(pg.enabled);

    let weather = config.mcp_servers.get("weather").expect("weather config");
    assert_eq!(weather.url.as_deref(), Some("http://127.0.0.1:8081/sse"));
    assert_eq!(weather.timeout_secs, 15);
}

#[test]
fn test_topology_config_json_loading() {
    let json_str = r#"{
  "mcpServers": {
    "git": {
      "command": "git-mcp-server",
      "args": ["--read-only"]
    }
  },
  "default_tenant": "acme-corp"
}"#;

    let config = TopologyConfigLoader::load_from_str(json_str).expect("parse json");
    assert_eq!(config.default_tenant.as_deref(), Some("acme-corp"));
    let git = config.mcp_servers.get("git").expect("git backend");
    assert_eq!(git.command.as_deref(), Some("git-mcp-server"));
    assert_eq!(git.timeout_secs, 30); // Default timeout
    assert!(git.enabled); // Default enabled
}

#[tokio::test]
async fn test_hermetic_subprocess_env_isolation() {
    // Verify that environment variables from host are not leaked by default (OWASP LLM08)
    let mut explicit_env = HashMap::new();
    explicit_env.insert("EXPLICIT_TEST_VAR".to_string(), "HERMETIC_ISOLATION_OK".to_string());

    let backend = HermeticSubprocessBackend::new(BackendConfig {
        name: "test-echo".to_string(),
        command: Some("python3".to_string()),
        args: vec![
            "-u".to_string(),
            "-c".to_string(),
            r#"
import sys, json, os
for line in sys.stdin:
    req = json.loads(line)
    val = os.environ.get("EXPLICIT_TEST_VAR", "MISSING")
    # CARGO and CARGO_MANIFEST_DIR exist in test runner environment; must be cleared in child
    leak = "LEAKED" if ("CARGO" in os.environ or "CARGO_MANIFEST_DIR" in os.environ) else "CLEAR"
    resp = {
        "jsonrpc": "2.0",
        "id": req.get("id"),
        "result": {
            "explicit_val": val,
            "secret_leaked": leak
        }
    }
    sys.stdout.write(json.dumps(resp) + "\n")
    sys.stdout.flush()
"#.to_string(),
        ],
        env: explicit_env,
        url: None,
        timeout_secs: 10,
        enabled: true,
    });

    backend.start().await.expect("start subprocess backend");
    assert!(backend.is_healthy().await);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(1)),
        method: "test_env".to_string(),
        params: None,
    };

    let resp = backend.send_request(&req).await.expect("send request to subprocess");
    assert_eq!(resp.id, JsonRpcId::Number(1));
    let result = resp.result.expect("result expected");
    assert_eq!(result["explicit_val"], "HERMETIC_ISOLATION_OK");
    assert_eq!(result["secret_leaked"], "CLEAR", "Host environment must be cleared!");

    backend.stop().await.expect("stop subprocess");
}

#[tokio::test]
async fn test_remote_http_backend_lifecycle() {
    let remote = RemoteHttpBackend::new("cloud-mcp", "https://api.aegis.cloud/mcp");
    assert!(remote.is_healthy().await);

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::String("rem-1".to_string())),
        method: "ping".to_string(),
        params: None,
    };

    let resp = remote.send_request(&req).await.expect("send remote request");
    assert_eq!(resp.id, JsonRpcId::String("rem-1".to_string()));
    let result = resp.result.expect("result object");
    assert_eq!(result["server"], "cloud-mcp");

    remote.stop().await.expect("stop remote");
    assert!(!remote.is_healthy().await);
}

#[tokio::test]
async fn test_subprocess_backend_registry_and_discovery() {
    let registry = SubprocessBackendRegistry::new();

    // Register backend 1 with 2 tools
    let mock1 = Arc::new(MockBackendTransport::new(vec![
        json!({
            "name": "read_file",
            "description": "Read file contents from filesystem",
            "inputSchema": { "type": "object", "properties": { "path": { "type": "string" } } }
        }),
        json!({
            "name": "write_file",
            "description": "Write contents to filesystem",
            "inputSchema": { "type": "object", "properties": { "path": { "type": "string" }, "content": { "type": "string" } } }
        }),
    ]));
    registry.register("fs-server", mock1).await.expect("register fs-server");

    // Register backend 2 with 1 tool
    let mock2 = Arc::new(MockBackendTransport::new(vec![
        json!({
            "name": "fetch_http",
            "description": "Fetch remote URL",
            "inputSchema": { "type": "object" }
        }),
    ]));
    registry.register("http-server", mock2).await.expect("register http-server");

    // 1. Verify list_backends
    let mut backends = registry.list_backends().await.expect("list backends");
    backends.sort();
    assert_eq!(backends, vec!["fs-server", "http-server"]);

    // 2. Verify get backend
    let retrieved = registry.get("fs-server").await.expect("get fs-server");
    assert!(retrieved.is_healthy().await);

    // 3. Verify discover_all_tools aggregates across active backends
    let tools = registry.discover_all_tools().await.expect("discover all tools");
    assert_eq!(tools.len(), 3);

    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    assert!(tool_names.contains(&"read_file".to_string()));
    assert!(tool_names.contains(&"write_file".to_string()));
    assert!(tool_names.contains(&"fetch_http".to_string()));

    // Check server attribution
    let read_tool = tools.iter().find(|t| t.name == "read_file").unwrap();
    assert_eq!(read_tool.server, "fs-server");
    assert_eq!(read_tool.tags, vec!["fs-server"]);
}
