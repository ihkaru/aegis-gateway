// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crate::core::error::AegisResult;

/// Enterprise Secret Store abstraction (HashiCorp Vault, AWS Secrets Manager, Env)
#[async_trait]
pub trait SecretStore: Send + Sync {
    /// Retrieve secret value by path/key
    async fn get_secret(&self, key: &str) -> AegisResult<Option<String>>;

    /// Store or update secret (optional depending on permissions)
    async fn set_secret(&self, key: &str, value: &str) -> AegisResult<()>;

    /// Check if secret store is reachable
    async fn health_check(&self) -> AegisResult<bool>;
}
