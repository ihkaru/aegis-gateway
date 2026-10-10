// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::core::error::AegisResult;
use crate::core::types::CallerContext;

/// Delegated short-lived OAuth token bound to a human caller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedToken {
    pub user_id: String,
    pub service: String,
    pub access_token: String,
    pub token_type: String,
    pub expires_at_epoch_secs: u64,
    pub scopes: Vec<String>,
}

/// Abstract contract for 3-Legged user identity delegation
#[async_trait]
pub trait IdentityDelegationBroker: Send + Sync {
    /// Resolve user-specific delegated token based on caller identity
    async fn resolve_user_token(
        &self,
        caller: &CallerContext,
        target_service: &str,
    ) -> AegisResult<Option<DelegatedToken>>;

    /// Store or update a delegated user token
    async fn store_user_token(
        &self,
        caller_id: &str,
        token: DelegatedToken,
    ) -> AegisResult<()>;

    /// Invalidate delegated user token on logout or session revocation
    async fn revoke_user_token(&self, caller_id: &str, target_service: &str) -> AegisResult<()>;
}

/// Abstract contract for virtual blast-radius resource scoping
#[async_trait]
pub trait ResourceScoper: Send + Sync {
    /// Verify target resource URI conforms to caller's permitted virtual scope
    async fn validate_resource_scope(
        &self,
        caller: &CallerContext,
        target_resource: &str,
        operation: &str,
    ) -> AegisResult<bool>;
}
