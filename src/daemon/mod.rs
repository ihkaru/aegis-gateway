// SPDX-License-Identifier: MIT

use std::path::Path;
use std::sync::Arc;

use crate::backend::{
    HermeticSubprocessBackend, RemoteHttpBackend, SubprocessBackendRegistry, TopologyConfigLoader,
};
use crate::core::backend::{PropylonTopologyConfig, BackendRegistry, BackendTransport};
use crate::core::error::{PropylonError, PropylonResult};
use crate::core::transport::IngressTransport;
use crate::transport::{LiveHttpServer, McpProtocolHandler, StdioTransport};
use crate::PropylonGateway;

/// Daemon supervisor coordinating bootstrap, multi-backend spawning, discovery, and runtime serving
pub struct DaemonSupervisor;

impl DaemonSupervisor {
    /// Load configuration file or discover default `propylon.yaml`
    pub fn resolve_config(config_path: Option<&str>) -> PropylonResult<Option<PropylonTopologyConfig>> {
        let path = match config_path {
            Some(p) => Some(p.to_string()),
            None => {
                if Path::new("propylon.yaml").exists() {
                    Some("propylon.yaml".to_string())
                } else if Path::new("propylon.json").exists() {
                    Some("propylon.json".to_string())
                } else if let Ok(home) = std::env::var("HOME") {
                    let user_cfg = Path::new(&home).join(".config/propylon/propylon.yaml");
                    if user_cfg.exists() {
                        Some(user_cfg.to_string_lossy().to_string())
                    } else if Path::new("/etc/propylon/propylon.yaml").exists() {
                        Some("/etc/propylon/propylon.yaml".to_string())
                    } else {
                        None
                    }
                } else if Path::new("/etc/propylon/propylon.yaml").exists() {
                    Some("/etc/propylon/propylon.yaml".to_string())
                } else {
                    None
                }
            }
        };

        if let Some(p) = path {
            let content = std::fs::read_to_string(&p).map_err(|e| {
                PropylonError::Internal(format!("Failed to read configuration file at '{p}': {e}"))
            })?;
            let cfg = TopologyConfigLoader::load_from_str(&content)?;
            TopologyConfigLoader::validate(&cfg)?;
            eprintln!("[PROPYLON] Loaded topology configuration from '{p}' ({} backends)", cfg.mcp_servers.len());
            Ok(Some(cfg))
        } else {
            eprintln!("[PROPYLON] No configuration file specified; running with default in-memory drivers");
            Ok(None)
        }
    }

    /// Bootstrap the entire operational gateway stack and spawn configured backends
    pub async fn bootstrap(
        config_path: Option<&str>,
    ) -> PropylonResult<(
        Arc<PropylonGateway>,
        Arc<McpProtocolHandler>,
        Arc<SubprocessBackendRegistry>,
    )> {
        let gateway = Arc::new(PropylonGateway::default());
        let registry = Arc::new(SubprocessBackendRegistry::with_default_local_engine());
        let handler = Arc::new(McpProtocolHandler::with_registry(
            Arc::clone(&gateway),
            Some(Arc::clone(&registry) as Arc<dyn BackendRegistry>),
        ));

        if let Some(cfg) = Self::resolve_config(config_path)? {
            for (name, srv) in cfg.mcp_servers {
                if !srv.enabled {
                    continue;
                }

                if srv.command.is_some() {
                    let backend = Arc::new(HermeticSubprocessBackend::new(srv));
                    if let Err(e) = backend.start().await {
                        eprintln!("[PROPYLON] Warning: failed to spawn backend '{name}': {e}");
                    } else {
                        eprintln!("[PROPYLON] Spawned hermetic subprocess backend '{name}'");
                        registry.register(&name, backend).await?;
                    }
                } else if let Some(url) = srv.url {
                    let remote: Arc<dyn BackendTransport> = Arc::new(RemoteHttpBackend::new(&name, url));
                    registry.register(&name, remote).await?;
                    eprintln!("[PROPYLON] Registered remote backend '{name}'");
                }
            }

            // Discover all tools across active backends and populate protocol handler
            let tools = registry.discover_all_tools().await?;
            eprintln!("[PROPYLON] Discovered {} tools across registered backends", tools.len());
            handler.register_tools(tools).await;
        } else {
            let enable_demo = std::env::var("PROPYLON_ENABLE_DEMO_FIXTURES")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false);
            if enable_demo {
                let defaults = crate::discovery::default_enterprise_catalog();
                eprintln!("[PROPYLON] Seeded {} demo enterprise tools into catalog", defaults.len());
                handler.register_tools(defaults).await;
            } else {
                eprintln!("[PROPYLON] Clean gateway mode: 0 synthetic catalog fixtures seeded. Operating with native meta-tools and registered backends.");
            }
        }

        Ok((gateway, handler, registry))
    }

    /// Run the daemon over Stdio (Claude Desktop, Cursor)
    pub async fn run_stdio(handler: Arc<McpProtocolHandler>) -> PropylonResult<()> {
        let stdio = StdioTransport::new(handler);
        stdio.run().await
    }

    /// Run the daemon as a live Streamable HTTP server
    pub async fn run_http(
        gateway: Arc<PropylonGateway>,
        handler: Arc<McpProtocolHandler>,
        host: &str,
        port: u16,
    ) -> PropylonResult<()> {
        let server = LiveHttpServer::new(
            handler,
            Arc::clone(gateway.drain_coordinator_arc()),
            host,
            port,
        )?;
        server.run().await
    }
}
