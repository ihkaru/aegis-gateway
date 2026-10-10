// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::core::error::AegisResult;

/// Agnostic OAuth 2.1 Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProviderConfig {
    pub provider_name: String,
    pub auth_endpoint: String,
    pub token_endpoint: String,
    pub client_id: String,
    pub default_scopes: Vec<String>,
}

/// Pending interactive OAuth PKCE connect session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectSession {
    pub session_id: String,
    pub caller_id: String,
    pub provider: String,
    pub code_verifier: String,
    pub state_nonce: String,
    pub created_at_epoch_secs: u64,
}

/// Structured response emitted when a tool requires user authentication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthRequiredResponse {
    pub status: String,
    pub provider: String,
    pub connect_url: String,
    pub message: String,
}

/// Abstract contract for vendor-agnostic managed OAuth connect engine
#[async_trait]
pub trait OAuthConnectEngine: Send + Sync {
    /// Initiate an agnostic OAuth connect flow with PKCE
    async fn initiate_connect(
        &self,
        caller_id: &str,
        provider: &str,
        redirect_uri: &str,
    ) -> AegisResult<(String, ConnectSession)>;

    /// Complete authorization callback and persist tokens
    async fn handle_callback(
        &self,
        code: &str,
        state: &str,
    ) -> AegisResult<String>;

    /// Check if caller has active authorized token for provider
    async fn has_active_token(&self, caller_id: &str, provider: &str) -> AegisResult<bool>;
}
