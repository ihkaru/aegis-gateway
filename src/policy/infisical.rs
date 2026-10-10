// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::core::error::{AegisError, AegisResult};
use crate::core::secrets::SecretStore;

/// Cache entry with TTL expiration timestamp
#[derive(Debug, Clone)]
struct CacheEntry {
    value: String,
    expires_at: Instant,
}

/// Abstract transport trait for Infisical Universal Auth and Secret REST APIs
#[async_trait]
pub trait InfisicalTransport: Send + Sync {
    /// Perform Universal Auth login using machine identity client_id and client_secret
    async fn login_universal_auth(
        &self,
        base_url: &str,
        client_id: &str,
        client_secret: &str,
    ) -> AegisResult<String>;

    /// Fetch raw secret value for given key from Infisical workspace
    async fn fetch_raw_secret(
        &self,
        base_url: &str,
        token: &str,
        project_id: &str,
        environment: &str,
        key: &str,
    ) -> AegisResult<Option<String>>;
}

/// In-memory transport for testing and offline environments
pub struct LocalInfisicalTransport {
    vault: Arc<RwLock<HashMap<String, String>>>,
    token: String,
}

impl LocalInfisicalTransport {
    pub fn new() -> Self {
        Self {
            vault: Arc::new(RwLock::new(HashMap::new())),
            token: "mock_infisical_session_token".to_string(),
        }
    }

    pub fn with_secret(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if let Ok(mut lock) = self.vault.try_write() {
            lock.insert(key.into(), value.into());
        }
        self
    }
}

impl Default for LocalInfisicalTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InfisicalTransport for LocalInfisicalTransport {
    async fn login_universal_auth(
        &self,
        _base_url: &str,
        client_id: &str,
        client_secret: &str,
    ) -> AegisResult<String> {
        if client_id.is_empty() || client_secret.is_empty() {
            return Err(AegisError::SecretError("Invalid machine identity credentials".into()));
        }
        Ok(self.token.clone())
    }

    async fn fetch_raw_secret(
        &self,
        _base_url: &str,
        _token: &str,
        _project_id: &str,
        _environment: &str,
        key: &str,
    ) -> AegisResult<Option<String>> {
        let read = self.vault.read().await;
        Ok(read.get(key).cloned())
    }
}

/// Infisical Secret Store driver implementing dynamic pull & memory cache TTL
pub struct InfisicalSecretStore {
    base_url: String,
    client_id: String,
    client_secret: String,
    project_id: String,
    environment: String,
    ttl: Duration,
    transport: Arc<dyn InfisicalTransport>,
    cached_session_token: Arc<RwLock<Option<(String, Instant)>>>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl InfisicalSecretStore {
    pub fn new(
        base_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        project_id: impl Into<String>,
        environment: impl Into<String>,
        transport: Arc<dyn InfisicalTransport>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            project_id: project_id.into(),
            environment: environment.into(),
            ttl: Duration::from_secs(300),
            transport,
            cached_session_token: Arc::new(RwLock::new(None)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    async fn get_or_refresh_session_token(&self) -> AegisResult<String> {
        let read = self.cached_session_token.read().await;
        if let Some((tok, expiry)) = read.as_ref() {
            if Instant::now() < *expiry {
                return Ok(tok.clone());
            }
        }
        drop(read);

        let mut write = self.cached_session_token.write().await;
        let tok = self
            .transport
            .login_universal_auth(&self.base_url, &self.client_id, &self.client_secret)
            .await?;
        *write = Some((tok.clone(), Instant::now() + Duration::from_secs(3600)));
        Ok(tok)
    }
}

#[async_trait]
impl SecretStore for InfisicalSecretStore {
    async fn get_secret(&self, key: &str) -> AegisResult<Option<String>> {
        let read = self.cache.read().await;
        if let Some(entry) = read.get(key) {
            if Instant::now() < entry.expires_at {
                return Ok(Some(entry.value.clone()));
            }
        }
        drop(read);

        let token = self.get_or_refresh_session_token().await?;
        let secret = self
            .transport
            .fetch_raw_secret(&self.base_url, &token, &self.project_id, &self.environment, key)
            .await?;

        if let Some(val) = &secret {
            let mut write = self.cache.write().await;
            write.insert(
                key.to_string(),
                CacheEntry {
                    value: val.clone(),
                    expires_at: Instant::now() + self.ttl,
                },
            );
        }

        Ok(secret)
    }

    async fn set_secret(&self, key: &str, value: &str) -> AegisResult<()> {
        let mut write = self.cache.write().await;
        write.insert(
            key.to_string(),
            CacheEntry {
                value: value.to_string(),
                expires_at: Instant::now() + self.ttl,
            },
        );
        Ok(())
    }

    async fn invalidate(&self, key: &str) -> AegisResult<()> {
        let mut write = self.cache.write().await;
        write.remove(key);
        Ok(())
    }

    async fn health_check(&self) -> AegisResult<bool> {
        Ok(!self.base_url.is_empty() && !self.client_id.is_empty())
    }
}

/// Chained fallback secret store (Memory Cache -> Vault / Infisical -> Local Env)
pub struct ChainedSecretStore {
    stores: Vec<Arc<dyn SecretStore>>,
}

impl ChainedSecretStore {
    pub fn new(stores: Vec<Arc<dyn SecretStore>>) -> Self {
        Self { stores }
    }
}

#[async_trait]
impl SecretStore for ChainedSecretStore {
    async fn get_secret(&self, key: &str) -> AegisResult<Option<String>> {
        for store in &self.stores {
            if let Ok(Some(secret)) = store.get_secret(key).await {
                return Ok(Some(secret));
            }
        }
        Ok(None)
    }

    async fn set_secret(&self, key: &str, value: &str) -> AegisResult<()> {
        for store in &self.stores {
            let _ = store.set_secret(key, value).await;
        }
        Ok(())
    }

    async fn invalidate(&self, key: &str) -> AegisResult<()> {
        for store in &self.stores {
            let _ = store.invalidate(key).await;
        }
        Ok(())
    }

    async fn health_check(&self) -> AegisResult<bool> {
        for store in &self.stores {
            if store.health_check().await? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
