// SPDX-License-Identifier: MIT

use std::path::Path;
use std::sync::Arc;

use crate::backend::{
    HermeticSubprocessBackend, RemoteHttpBackend, SubprocessBackendRegistry, TopologyConfigLoader,
};
use crate::core::backend::{AegisTopologyConfig, BackendRegistry, BackendTransport};
use crate::core::error::{AegisError, AegisResult};
use crate::core::transport::IngressTransport;
use crate::transport::{LiveHttpServer, McpProtocolHandler, StdioTransport};
use crate::AegisGateway;

/// Daemon supervisor coordinating bootstrap, multi-backend spawning, discovery, and runtime serving
pub struct DaemonSupervisor;

impl DaemonSupervisor {
    /// Load configuration file or discover default `aegis.yaml`
    pub fn resolve_config(config_path: Option<&str>) -> AegisResult<Option<AegisTopologyConfig>> {
        let path = match config_path {
            Some(p) => Some(p.to_string()),
            None => {
                if Path::new("aegis.yaml").exists() {
                    Some("aegis.yaml".to_string())
                } else if Path::new("aegis.json").exists() {
                    Some("aegis.json".to_string())
                } else if let Ok(home) = std::env::var("HOME") {
                    let user_cfg = Path::new(&home).join(".config/aegis/aegis.yaml");
                    if user_cfg.exists() {
                        Some(user_cfg.to_string_lossy().to_string())
                    } else if Path::new("/etc/aegis/aegis.yaml").exists() {
                        Some("/etc/aegis/aegis.yaml".to_string())
                    } else {
                        None
                    }
                } else if Path::new("/etc/aegis/aegis.yaml").exists() {
                    Some("/etc/aegis/aegis.yaml".to_string())
                } else {
                    None
                }
            }
        };

        if let Some(p) = path {
            let content = std::fs::read_to_string(&p).map_err(|e| {
                AegisError::Internal(format!("Failed to read configuration file at '{p}': {e}"))
            })?;
            let cfg = TopologyConfigLoader::load_from_str(&content)?;
            TopologyConfigLoader::validate(&cfg)?;
            eprintln!("[AEGIS] Loaded topology configuration from '{p}' ({} backends)", cfg.mcp_servers.len());
            Ok(Some(cfg))
        } else {
            eprintln!("[AEGIS] No configuration file specified; running with default in-memory drivers");
            Ok(None)
        }
    }

    /// Bootstrap the entire operational gateway stack and spawn configured backends
    pub async fn bootstrap(
        config_path: Option<&str>,
    ) -> AegisResult<(
        Arc<AegisGateway>,
        Arc<McpProtocolHandler>,
        Arc<SubprocessBackendRegistry>,
    )> {
        let gateway = Arc::new(AegisGateway::default());
        let handler = Arc::new(McpProtocolHandler::new(Arc::clone(&gateway)));
        let registry = Arc::new(SubprocessBackendRegistry::new());

        if let Some(cfg) = Self::resolve_config(config_path)? {
            for (name, srv) in cfg.mcp_servers {
                if !srv.enabled {
                    continue;
                }

                if srv.command.is_some() {
                    let backend = Arc::new(HermeticSubprocessBackend::new(srv));
                    if let Err(e) = backend.start().await {
                        eprintln!("[AEGIS] Warning: failed to spawn backend '{name}': {e}");
                    } else {
                        eprintln!("[AEGIS] Spawned hermetic subprocess backend '{name}'");
                        registry.register(&name, backend).await?;
                    }
                } else if let Some(url) = srv.url {
                    let remote: Arc<dyn BackendTransport> = Arc::new(RemoteHttpBackend::new(&name, url));
                    registry.register(&name, remote).await?;
                    eprintln!("[AEGIS] Registered remote backend '{name}'");
                }
            }

            // Discover all tools across active backends and populate protocol handler
            let tools = registry.discover_all_tools().await?;
            eprintln!("[AEGIS] Discovered {} tools across registered backends", tools.len());
            handler.register_tools(tools).await;
        }

        Ok((gateway, handler, registry))
    }

    /// Run the daemon over Stdio (Claude Desktop, Cursor)
    pub async fn run_stdio(handler: Arc<McpProtocolHandler>) -> AegisResult<()> {
        let stdio = StdioTransport::new(handler);
        stdio.run().await
    }

    /// Run the daemon as a live Streamable HTTP server
    pub async fn run_http(
        gateway: Arc<AegisGateway>,
        handler: Arc<McpProtocolHandler>,
        host: &str,
        port: u16,
    ) -> AegisResult<()> {
        let server = LiveHttpServer::new(
            handler,
            Arc::clone(gateway.drain_coordinator_arc()),
            host,
            port,
        )?;
        server.run().await
    }
}
