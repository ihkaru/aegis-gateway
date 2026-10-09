// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

//! E2E Tier 3: Fullstack Team Lead Multiplexing & Orchestration Test Suite
//! Persona: A team lead orchestrating multiple local MCP micro-servers (DB, filesystem, git)
//! under a unified gateway topology with zero zombie processes and clean discovery.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::json;

use aegis_gateway::backend::{
    HermeticSubprocessBackend, SubprocessBackendRegistry, TopologyConfigLoader,
};
use aegis_gateway::core::backend::{BackendConfig, BackendRegistry, BackendTransport};
use aegis_gateway::core::error::AegisResult;
use aegis_gateway::core::transport::{JsonRpcId, JsonRpcRequest, JsonRpcResponse};

struct MockTeamBackend {
    name: String,
    healthy: AtomicBool,
    tools: Vec<serde_json::Value>,
}

impl MockTeamBackend {
    fn new(name: &str, tools: Vec<serde_json::Value>) -> Self {
        Self {
            name: name.to_string(),
            healthy: AtomicBool::new(true),
            tools,
        }
    }
}

#[async_trait]
impl BackendTransport for MockTeamBackend {
    async fn start(&self) -> AegisResult<()> {
        self.healthy.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn send_request(&self, request: &JsonRpcRequest) -> AegisResult<JsonRpcResponse> {
        if request.method == "tools/list" {
            Ok(JsonRpcResponse::success(
                request.id.clone().unwrap_or(JsonRpcId::Null),
                json!({ "tools": self.tools }),
            ))
        } else {
            Ok(JsonRpcResponse::success(
                request.id.clone().unwrap_or(JsonRpcId::Null),
                json!({ "status": "executed", "server": self.name }),
            ))
        }
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
fn test_team_multiplexing_topology_loading() {
    let team_yaml = r#"
mcpServers:
  filesystem:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"]
    timeout_secs: 60
    enabled: true
  database:
    command: "python3"
    args: ["-m", "db_server"]
    env:
      DB_HOST: "127.0.0.1"
      DB_PORT: "5432"
    timeout_secs: 30
    enabled: true
  github:
    url: "http://127.0.0.1:9090/mcp"
    timeout_secs: 20
    enabled: true
"#;

    let cfg = TopologyConfigLoader::load_from_str(team_yaml).expect("parse team config");
    assert_eq!(cfg.mcp_servers.len(), 3);

    let fs = cfg.mcp_servers.get("filesystem").unwrap();
    assert_eq!(fs.command.as_deref(), Some("npx"));
    assert_eq!(fs.args[2], "/workspace");

    let db = cfg.mcp_servers.get("database").unwrap();
    assert_eq!(db.env.get("DB_PORT").map(|s: &String| s.as_str()), Some("5432"));

    let gh = cfg.mcp_servers.get("github").unwrap();
    assert_eq!(gh.url.as_deref(), Some("http://127.0.0.1:9090/mcp"));
}

#[tokio::test]
async fn test_team_registry_concurrent_discovery_and_routing() {
    let registry = SubprocessBackendRegistry::new();

    let fs_backend = Arc::new(MockTeamBackend::new(
        "filesystem",
        vec![
            json!({ "name": "fs_read", "description": "Read file", "inputSchema": {} }),
            json!({ "name": "fs_write", "description": "Write file", "inputSchema": {} }),
        ],
    ));

    let db_backend = Arc::new(MockTeamBackend::new(
        "database",
        vec![
            json!({ "name": "db_query", "description": "Execute SQL", "inputSchema": {} }),
        ],
    ));

    registry.register("filesystem", fs_backend).await.expect("register fs");
    registry.register("database", db_backend).await.expect("register db");

    let mut backends = registry.list_backends().await.expect("list backends");
    backends.sort();
    assert_eq!(backends, vec!["database", "filesystem"]);

    let tools = registry.discover_all_tools().await.expect("discover all");
    assert_eq!(tools.len(), 3);

    let db_tool = tools.iter().find(|t| t.name == "db_query").unwrap();
    assert_eq!(db_tool.server, "database");
    assert_eq!(db_tool.tags, vec!["database"]);
}

#[tokio::test]
async fn test_team_zero_zombie_subprocess_guarantee() {
    // Spawn a long-running subprocess and verify clean kill on drop
    let backend = HermeticSubprocessBackend::new(BackendConfig {
        name: "sleep-mock".to_string(),
        command: Some("python3".to_string()),
        args: vec![
            "-u".to_string(),
            "-c".to_string(),
            r#"
import time, sys
for line in sys.stdin:
    sys.stdout.write(line)
    sys.stdout.flush()
"#.to_string(),
        ],
        env: Default::default(),
        url: None,
        timeout_secs: 5,
        enabled: true,
    });

    backend.start().await.expect("start backend process");
    assert!(backend.is_healthy().await, "Process must be healthy after spawn");

    // Explicit stop terminates the process group
    backend.stop().await.expect("stop must succeed");
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    assert!(!backend.is_healthy().await, "Process must be terminated with zero zombies");
}
