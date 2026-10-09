// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crate::core::error::AegisResult;

/// Session Revocation & SCIM Deactivation registry
#[async_trait]
pub trait SessionRevocationRegistry: Send + Sync {
    /// Check whether a session or user identity has been revoked
    async fn is_revoked(&self, session_id: &str, subject: &str) -> AegisResult<bool>;

    /// Revoke an active session immediately
    async fn revoke_session(&self, session_id: &str, reason: &str) -> AegisResult<()>;

    /// Deactivate user subject across all sessions (SCIM auto-deprovisioning)
    async fn deactivate_subject(&self, subject: &str, reason: &str) -> AegisResult<()>;
}
