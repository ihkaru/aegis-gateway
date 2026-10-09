// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use crate::core::backend::{AegisTopologyConfig, BackendConfig};
use crate::core::error::{AegisError, AegisResult};

/// Topology configuration loader supporting YAML and JSON
pub struct TopologyConfigLoader;

impl TopologyConfigLoader {
    /// Parse configuration from YAML or JSON string
    pub fn parse_str(raw: &str) -> AegisResult<AegisTopologyConfig> {
        let trimmed = raw.trim();

        let mut cfg: AegisTopologyConfig = if trimmed.starts_with('{') {
            serde_json::from_str::<AegisTopologyConfig>(trimmed).map_err(|e| {
                AegisError::Internal(format!("Failed to parse topology JSON configuration: {e}"))
            })?
        } else {
            serde_yaml::from_str::<AegisTopologyConfig>(trimmed).map_err(|e| {
                AegisError::Internal(format!("Failed to parse topology YAML configuration: {e}"))
            })?
        };

        for (name, srv) in &mut cfg.mcp_servers {
            if srv.name.is_empty() {
                srv.name = name.clone();
            }
        }

        Ok(cfg)
    }

    /// Alias for parse_str
    pub fn load_from_str(raw: &str) -> AegisResult<AegisTopologyConfig> {
        Self::parse_str(raw)
    }

    /// Validate server configurations
    pub fn validate(config: &AegisTopologyConfig) -> AegisResult<()> {
        for (name, srv) in &config.mcp_servers {
            if srv.command.is_none() && srv.url.is_none() {
                return Err(AegisError::Internal(format!(
                    "Backend '{}' is invalid: must specify either 'command' or 'url'",
                    name
                )));
            }
        }
        Ok(())
    }

    /// Generate default template config
    pub fn template() -> AegisTopologyConfig {
        let mut servers = HashMap::new();
        servers.insert(
            "postgres".to_string(),
            BackendConfig {
                name: "postgres".to_string(),
                command: Some("npx".to_string()),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-postgres".to_string()],
                env: HashMap::new(),
                url: None,
                timeout_secs: 30,
                enabled: true,
            },
        );
        AegisTopologyConfig {
            mcp_servers: servers,
            default_tenant: Some("enterprise-corp".to_string()),
        }
    }
}
