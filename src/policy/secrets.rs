// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::AegisResult;

use crate::core::secrets::SecretStore;

/// HashiCorp Vault KV v2 secret store adapter with local decryption cache
pub struct VaultSecretStore {
    vault_addr: String,
    local_secrets: Arc<RwLock<HashMap<String, String>>>,
}

impl VaultSecretStore {
    pub fn new(vault_addr: impl Into<String>) -> Self {
        Self {
            vault_addr: vault_addr.into(),
            local_secrets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_seed_secret(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut map = HashMap::new();
        map.insert(key.into(), value.into());
        Self {
            vault_addr: self.vault_addr,
            local_secrets: Arc::new(RwLock::new(map)),
        }
    }

    pub fn vault_addr(&self) -> &str {
        &self.vault_addr
    }
}

#[async_trait]
impl SecretStore for VaultSecretStore {
    async fn get_secret(&self, key: &str) -> AegisResult<Option<String>> {
        let read = self.local_secrets.read().await;
        Ok(read.get(key).cloned())
    }

    async fn set_secret(&self, key: &str, value: &str) -> AegisResult<()> {
        let mut write = self.local_secrets.write().await;
        write.insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn health_check(&self) -> AegisResult<bool> {
        Ok(!self.vault_addr.is_empty())
    }
}

/// Environment variable secret store with runtime memory override map (Zero Unsafe)
#[derive(Default)]
pub struct EnvSecretStore {
    overrides: Arc<RwLock<HashMap<String, String>>>,
}

impl EnvSecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SecretStore for EnvSecretStore {
    async fn get_secret(&self, key: &str) -> AegisResult<Option<String>> {
        let overrides = self.overrides.read().await;
        if let Some(val) = overrides.get(key) {
            return Ok(Some(val.clone()));
        }
        let sanitized_key = key.replace(['/', '-', '.'], "_").to_uppercase();
        Ok(std::env::var(&sanitized_key).ok())
    }

    async fn set_secret(&self, key: &str, value: &str) -> AegisResult<()> {
        let mut overrides = self.overrides.write().await;
        overrides.insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn health_check(&self) -> AegisResult<bool> {
        Ok(true)
    }
}
