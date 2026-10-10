// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::error::AegisResult;
use crate::core::types::TenantId;

/// Supported sandbox programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionLanguage {
    Python,
    Bash,
    JavaScript,
}

impl Default for ExecutionLanguage {
    fn default() -> Self {
        Self::Python
    }
}

impl std::str::FromStr for ExecutionLanguage {
    type Err = crate::core::error::AegisError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "python" | "py" | "python3" => Ok(Self::Python),
            "bash" | "sh" | "shell" => Ok(Self::Bash),
            "javascript" | "js" | "node" => Ok(Self::JavaScript),
            other => Err(crate::core::error::AegisError::Internal(format!(
                "Unsupported sandbox execution language: '{other}'"
            ))),
        }
    }
}

/// Execution request payload for hermetic sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxExecutionRequest {
    pub language: ExecutionLanguage,
    pub code: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub services: Vec<String>,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    #[serde(default)]
    pub tenant_id: Option<TenantId>,
    #[serde(default)]
    pub caller_id: Option<String>,
}

/// Execution result returned from hermetic sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxExecutionResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub code_hash_sha256: String,
    pub credentials_injected: Vec<String>,
    pub attestation: Option<String>,
}

/// Abstract contract for hermetic code sandbox execution
#[async_trait]
pub trait CodeSandboxEngine: Send + Sync {
    /// Execute code payload within hermetic process isolation
    async fn execute(&self, req: &SandboxExecutionRequest) -> AegisResult<SandboxExecutionResult>;

    /// Health check for underlying runtime availability (python3, bash, etc.)
    async fn is_available(&self) -> bool;
}

/// Abstract contract for zero-knowledge credential brokerage
#[async_trait]
pub trait CredentialBroker: Send + Sync {
    /// Broker short-lived scoped credentials for requested services
    async fn broker_credentials(&self, services: &[String]) -> AegisResult<HashMap<String, String>>;

    /// Redact any known brokered credentials from raw text
    async fn redact_secrets(&self, text: &str) -> String;
}

/// Abstract contract for preflight code egress inspection
#[async_trait]
pub trait EgressFirewall: Send + Sync {
    /// Inspect code source for forbidden egress patterns or destination targets
    async fn check_code_egress(&self, code: &str) -> AegisResult<()>;

    /// Inspect target host for SSRF or unauthorized egress destination
    async fn check_destination(&self, host: &str, port: u16) -> AegisResult<()>;
}
