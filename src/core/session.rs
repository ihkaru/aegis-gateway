// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crate::core::error::PropylonResult;

/// Session Revocation & SCIM Deactivation registry
#[async_trait]
pub trait SessionRevocationRegistry: Send + Sync {
    /// Check whether a session or user identity has been revoked
    async fn is_revoked(&self, session_id: &str, subject: &str) -> PropylonResult<bool>;

    /// Revoke an active session immediately
    async fn revoke_session(&self, session_id: &str, reason: &str) -> PropylonResult<()>;

    /// Deactivate user subject across all sessions (SCIM auto-deprovisioning)
    async fn deactivate_subject(&self, subject: &str, reason: &str) -> PropylonResult<()>;
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
    async fn register_session(&self, session_id: &str, tenant: &str) -> PropylonResult<()>;

    /// Acquire an execution lease before running an in-flight operation.
    /// Fails if session is terminating or tombstoned.
    async fn acquire_lease(&self, session_id: &str) -> PropylonResult<u64>;

    /// Release an active lease once execution completes.
    async fn release_lease(&self, session_id: &str, lease_id: u64) -> PropylonResult<()>;

    /// Mark a session as terminating and tombstone it, preventing new leases or state resurrection.
    async fn terminate_session(&self, session_id: &str, reason: &str) -> PropylonResult<()>;

    /// Get current status of a session. Returns Tombstoned if evicted/terminated.
    async fn get_status(&self, session_id: &str) -> PropylonResult<SessionLifecycleStatus>;

    /// Write session-scoped state (cost, token counts, tool history).
    /// Rejects with PropylonError::SessionTerminated if the session has been terminated/tombstoned.
    async fn write_session_state(&self, session_id: &str, key: &str, value: serde_json::Value) -> PropylonResult<()>;

    /// Read session-scoped state.
    async fn read_session_state(&self, session_id: &str, key: &str) -> PropylonResult<Option<serde_json::Value>>;

    /// Active in-flight lease count for a session.
    async fn active_leases(&self, session_id: &str) -> usize;
}
