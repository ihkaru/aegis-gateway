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

/// Session lifecycle states preventing resurrection of dead sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SessionLifecycleStatus {
    Active,
    Terminating,
    Tombstoned,
}

/// Session fence trait to protect in-flight execution and prevent resurrection of dead sessions (MikkoParkkola/mcp-gateway#2568)
#[async_trait]
pub trait SessionFence: Send + Sync {
    /// Register a new session as active
    async fn register_session(&self, session_id: &str, tenant: &str) -> AegisResult<()>;

    /// Acquire an execution lease before running an in-flight operation.
    /// Fails if session is terminating or tombstoned.
    async fn acquire_lease(&self, session_id: &str) -> AegisResult<u64>;

    /// Release an active lease once execution completes.
    async fn release_lease(&self, session_id: &str, lease_id: u64) -> AegisResult<()>;

    /// Mark a session as terminating and tombstone it, preventing new leases or state resurrection.
    async fn terminate_session(&self, session_id: &str, reason: &str) -> AegisResult<()>;

    /// Get current status of a session. Returns Tombstoned if evicted/terminated.
    async fn get_status(&self, session_id: &str) -> AegisResult<SessionLifecycleStatus>;

    /// Write session-scoped state (cost, token counts, tool history).
    /// Rejects with AegisError::SessionTerminated if the session has been terminated/tombstoned.
    async fn write_session_state(&self, session_id: &str, key: &str, value: serde_json::Value) -> AegisResult<()>;

    /// Read session-scoped state.
    async fn read_session_state(&self, session_id: &str, key: &str) -> AegisResult<Option<serde_json::Value>>;

    /// Active in-flight lease count for a session.
    async fn active_leases(&self, session_id: &str) -> usize;
}
