// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crate::core::error::PropylonResult;

/// Enterprise Secret Store abstraction (HashiCorp Vault, AWS Secrets Manager, Env)
#[async_trait]
pub trait SecretStore: Send + Sync {
    /// Retrieve secret value by path/key
    async fn get_secret(&self, key: &str) -> PropylonResult<Option<String>>;

    /// Store or update secret (optional depending on permissions)
    async fn set_secret(&self, key: &str, value: &str) -> PropylonResult<()>;

    /// Check if secret store is reachable
    async fn health_check(&self) -> PropylonResult<bool>;

    /// List available secret keys (zero-knowledge: names only, never secrets)
    async fn list_secret_keys(&self) -> PropylonResult<Vec<String>> {
        Ok(vec![])
    }

    /// Invalidate in-memory cache entry for a given secret (optional/hot-rotation)
    async fn invalidate(&self, _key: &str) -> PropylonResult<()> {
        Ok(())
    }
}
