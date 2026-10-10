// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crate::core::error::AegisResult;
use crate::core::types::CallerContext;

/// Enterprise OIDC / SAML Identity Provider Token Validator
#[async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validate bearer token, verify signatures, check expiry, and resolve CallerContext
    async fn validate_token(&self, token: &str) -> AegisResult<CallerContext>;
}

/// Configurable Server Identity for MCP wire initialize responses (resolves docker/mcp-gateway#583)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatewayServerIdentity {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
}

impl Default for GatewayServerIdentity {
    fn default() -> Self {
        Self {
            name: "aegis-gateway".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: "2024-11-05".to_string(),
        }
    }
}

impl GatewayServerIdentity {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            protocol_version: "2024-11-05".to_string(),
        }
    }

    pub fn with_protocol_version(mut self, protocol_version: impl Into<String>) -> Self {
        self.protocol_version = protocol_version.into();
        self
    }
}

