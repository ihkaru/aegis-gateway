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

/// Request initiating OAuth 2.1 Device Authorization Grant (RFC 8628)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeviceAuthRequest {
    pub client_id: String,
    pub scopes: Vec<String>,
}

/// Response returned to CLI client with verification codes and URLs
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// Current status of a device authorization session
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeviceCodeStatus {
    Pending,
    Approved { claims: CallerContext },
    Denied { reason: String },
    Expired,
}

/// RFC 8693 Token Exchange / Assertion Ingestion Request
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AssertionExchangeRequest {
    pub assertion_type: String, // e.g. "urn:ietf:params:oauth:grant-type:saml2-bearer"
    pub assertion: String,
    pub resource: Option<String>, // RFC 8707 Resource Indicator
}

/// Federated Token Issuance Response
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FederatedTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: String,
}

/// Interface contract for Enterprise Identity Federation & RFC 8628 Device Flow
#[async_trait]
pub trait IdentityFederationBroker: Send + Sync {
    /// Ingests SAML/OIDC enterprise assertion and issues scoped token
    async fn exchange_assertion(&self, req: &AssertionExchangeRequest) -> AegisResult<FederatedTokenResponse>;

    /// Initiates RFC 8628 Device Authorization for CLI / IDE agents
    async fn start_device_flow(&self, req: &DeviceAuthRequest) -> AegisResult<DeviceAuthResponse>;

    /// Polls device authorization state during browser SSO authentication
    async fn poll_device_flow(&self, device_code: &str) -> AegisResult<DeviceCodeStatus>;

    /// Completes device flow approval upon successful IdP SSO login
    async fn approve_device_flow(&self, user_code: &str, context: CallerContext) -> AegisResult<()>;
}


