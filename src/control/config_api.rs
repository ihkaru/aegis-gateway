// SPDX-License-Identifier: MIT

use std::path::Path;
use std::sync::Arc;
use serde_json::json;

use crate::backend::{HermeticSubprocessBackend, RemoteHttpBackend, TopologyConfigLoader};
use crate::core::backend::{BackendRegistry, BackendTransport};

/// Dispatcher for dynamic runtime configuration and topology management API
pub struct ConfigApiDispatcher;

impl ConfigApiDispatcher {
    pub async fn dispatch(
        method: &str,
        path: &str,
        body_bytes: &[u8],
        registry: Option<&Arc<dyn BackendRegistry>>,
    ) -> (u16, Vec<u8>, &'static str) {
        match (method, path) {
            ("GET", "/api/v1/config/raw") => Self::handle_get_raw(),
            ("POST", "/api/v1/config/validate") => Self::handle_validate(body_bytes),
            ("POST", "/api/v1/config/apply") => Self::handle_apply(body_bytes, registry).await,
            ("GET", "/api/v1/config/template") => Self::handle_get_template(),
            _ => (404, json!({ "error": "Config endpoint not found" }).to_string().into_bytes(), "application/json"),
        }
    }

    fn handle_get_raw() -> (u16, Vec<u8>, &'static str) {
        let (config_path, content) = if Path::new("propylon.yaml").exists() {
            let text = std::fs::read_to_string("propylon.yaml").unwrap_or_default();
            ("propylon.yaml", text)
        } else if Path::new("propylon.json").exists() {
            let text = std::fs::read_to_string("propylon.json").unwrap_or_default();
            ("propylon.json", text)
        } else {
            let default_template = TopologyConfigLoader::template();
            let yaml_str = serde_yaml::to_string(&default_template).unwrap_or_default();
            ("propylon.yaml", yaml_str)
        };

        let payload = json!({
            "path": config_path,
            "format": if config_path.ends_with(".json") { "json" } else { "yaml" },
            "content": content,
            "status": "LOADED"
        });
        (200, payload.to_string().into_bytes(), "application/json")
    }

    fn handle_validate(body_bytes: &[u8]) -> (u16, Vec<u8>, &'static str) {
        let Ok(req) = serde_json::from_slice::<serde_json::Value>(body_bytes) else {
            return (400, json!({ "valid": false, "error": "Invalid JSON request body" }).to_string().into_bytes(), "application/json");
        };

        let Some(content) = req.get("content").and_then(|c| c.as_str()) else {
            return (400, json!({ "valid": false, "error": "Missing 'content' field in request body" }).to_string().into_bytes(), "application/json");
        };

        match TopologyConfigLoader::parse_str(content) {
            Ok(cfg) => match TopologyConfigLoader::validate(&cfg) {
                Ok(_) => {
                    let backends: Vec<String> = cfg.mcp_servers.keys().cloned().collect();
                    let payload = json!({
                        "valid": true,
                        "backends_count": cfg.mcp_servers.len(),
                        "default_tenant": cfg.default_tenant,
                        "backends": backends
                    });
                    (200, payload.to_string().into_bytes(), "application/json")
                }
                Err(err) => (
                    422,
                    json!({ "valid": false, "error": format!("{err}") }).to_string().into_bytes(),
                    "application/json",
                ),
            },
            Err(err) => (
                422,
                json!({ "valid": false, "error": format!("{err}") }).to_string().into_bytes(),
                "application/json",
            ),
        }
    }

    async fn handle_apply(
        body_bytes: &[u8],
        registry: Option<&Arc<dyn BackendRegistry>>,
    ) -> (u16, Vec<u8>, &'static str) {
        let Ok(req) = serde_json::from_slice::<serde_json::Value>(body_bytes) else {
            return (400, json!({ "status": "ERROR", "error": "Invalid JSON request body" }).to_string().into_bytes(), "application/json");
        };

        let Some(content) = req.get("content").and_then(|c| c.as_str()) else {
            return (400, json!({ "status": "ERROR", "error": "Missing 'content' field in request body" }).to_string().into_bytes(), "application/json");
        };

        // 1. Validate structure
        let cfg = match TopologyConfigLoader::parse_str(content) {
            Ok(c) => match TopologyConfigLoader::validate(&c) {
                Ok(_) => c,
                Err(e) => {
                    return (
                        422,
                        json!({ "status": "VALIDATION_FAILED", "error": format!("{e}") }).to_string().into_bytes(),
                        "application/json",
                    );
                }
            },
            Err(e) => {
                return (
                    422,
                    json!({ "status": "PARSE_ERROR", "error": format!("{e}") }).to_string().into_bytes(),
                    "application/json",
                );
            }
        };

        // 2. Persist safely with backup
        if Path::new("propylon.yaml").exists() {
            let _ = std::fs::copy("propylon.yaml", "propylon.yaml.bak");
        }
        if let Err(e) = std::fs::write("propylon.yaml", content) {
            return (
                500,
                json!({ "status": "IO_ERROR", "error": format!("Failed to write propylon.yaml: {e}") }).to_string().into_bytes(),
                "application/json",
            );
        }

        // 3. Hot-reload topology into active registry with zero downtime
        let mut reloaded_backends = Vec::new();
        if let Some(reg) = registry {
            for (name, srv) in &cfg.mcp_servers {
                if !srv.enabled {
                    continue;
                }
                if srv.command.is_some() {
                    let backend = Arc::new(HermeticSubprocessBackend::new(srv.clone()));
                    if let Err(e) = backend.start().await {
                        eprintln!("[PROPYLON HOT-RELOAD] Warning spawning backend '{name}': {e}");
                    } else if reg.register(name, backend).await.is_ok() {
                        reloaded_backends.push(name.clone());
                    }
                } else if let Some(url) = &srv.url {
                    let remote: Arc<dyn BackendTransport> = Arc::new(RemoteHttpBackend::new(name, url.clone()));
                    if reg.register(name, remote).await.is_ok() {
                        reloaded_backends.push(name.clone());
                    }
                }
            }
        }

        let payload = json!({
            "status": "APPLIED",
            "message": "Configuration saved and topology hot-reloaded successfully",
            "backends_count": cfg.mcp_servers.len(),
            "reloaded_backends": reloaded_backends,
            "default_tenant": cfg.default_tenant
        });
        (200, payload.to_string().into_bytes(), "application/json")
    }

    fn handle_get_template() -> (u16, Vec<u8>, &'static str) {
        let template_cfg = TopologyConfigLoader::template();
        let yaml_str = serde_yaml::to_string(&template_cfg).unwrap_or_else(|_| {
            "mcp_servers:\n  sqlite_ledger:\n    name: sqlite_ledger\n    command: python3\n    args:\n    - scripts/sqlite_mcp_server.py\n    timeout_secs: 30\n    enabled: true\ndefault_tenant: enterprise-corp\n".to_string()
        });

        let payload = json!({
            "format": "yaml",
            "content": yaml_str
        });
        (200, payload.to_string().into_bytes(), "application/json")
    }
}
