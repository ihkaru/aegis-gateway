// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::delegation::{DelegatedToken, IdentityDelegationBroker, ResourceScoper};
use crate::core::error::AegisResult;
use crate::core::types::CallerContext;

/// In-memory user-delegated OAuth token broker implementing OBO resolution
#[derive(Default)]
pub struct UserIdentityDelegationBroker {
    tokens: Arc<RwLock<HashMap<String, HashMap<String, DelegatedToken>>>>,
}

impl UserIdentityDelegationBroker {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl IdentityDelegationBroker for UserIdentityDelegationBroker {
    async fn resolve_user_token(
        &self,
        caller: &CallerContext,
        target_service: &str,
    ) -> AegisResult<Option<DelegatedToken>> {
        let read = self.tokens.read().await;
        if let Some(user_map) = read.get(&caller.subject) {
            if let Some(token) = user_map.get(target_service) {
                let now_epoch = chrono::Utc::now().timestamp() as u64;
                if now_epoch < token.expires_at_epoch_secs {
                    return Ok(Some(token.clone()));
                }
            }
        }
        Ok(None)
    }

    async fn store_user_token(
        &self,
        caller_id: &str,
        token: DelegatedToken,
    ) -> AegisResult<()> {
        let mut write = self.tokens.write().await;
        let user_map = write.entry(caller_id.to_string()).or_default();
        user_map.insert(token.service.clone(), token);
        Ok(())
    }

    async fn revoke_user_token(&self, caller_id: &str, target_service: &str) -> AegisResult<()> {
        let mut write = self.tokens.write().await;
        if let Some(user_map) = write.get_mut(caller_id) {
            user_map.remove(target_service);
        }
        Ok(())
    }
}

/// Virtual blast-radius resource scoper restricting Drive folders & repos
#[derive(Default)]
pub struct VirtualResourceScoper {
    allowed_scopes: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl VirtualResourceScoper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_user_scope(self, user_id: impl Into<String>, permitted_prefix: impl Into<String>) -> Self {
        if let Ok(mut lock) = self.allowed_scopes.try_write() {
            let list = lock.entry(user_id.into()).or_default();
            list.push(permitted_prefix.into());
        }
        self
    }
}

#[async_trait]
impl ResourceScoper for VirtualResourceScoper {
    async fn validate_resource_scope(
        &self,
        caller: &CallerContext,
        target_resource: &str,
        _operation: &str,
    ) -> AegisResult<bool> {
        let read = self.allowed_scopes.read().await;
        if let Some(allowed) = read.get(&caller.subject) {
            if allowed.is_empty() || allowed.iter().any(|s| s == "*") {
                return Ok(true);
            }
            let is_permitted = allowed.iter().any(|prefix| target_resource.starts_with(prefix));
            return Ok(is_permitted);
        }
        // Default permit if no explicit restrictive scope profile exists
        Ok(true)
    }
}
