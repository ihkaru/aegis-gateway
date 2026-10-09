// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::{AegisError, AegisResult};
use crate::core::session::{SessionFence, SessionLifecycleStatus};

#[derive(Debug, Clone)]
struct SessionRecord {
    tenant: String,
    status: SessionLifecycleStatus,
    active_leases: HashSet<u64>,
    next_lease_id: u64,
    state_store: HashMap<String, Value>,
    termination_reason: Option<String>,
}

/// In-memory implementation of SessionFence preventing in-flight resurrection (MikkoParkkola/mcp-gateway#2568)
#[derive(Clone, Default)]
pub struct InMemorySessionFence {
    sessions: Arc<RwLock<HashMap<String, SessionRecord>>>,
    tombstones: Arc<RwLock<HashSet<String>>>,
}

impl InMemorySessionFence {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            tombstones: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Check if a session ID is marked in tombstone history
    pub async fn is_tombstoned(&self, session_id: &str) -> bool {
        let tombs = self.tombstones.read().await;
        tombs.contains(session_id)
    }

    /// Total count of sessions in memory (including active & terminating)
    pub async fn total_sessions(&self) -> usize {
        let s = self.sessions.read().await;
        s.len()
    }

    /// Retrieve the tenant associated with a session ID
    pub async fn get_tenant(&self, session_id: &str) -> Option<String> {
        let s = self.sessions.read().await;
        s.get(session_id).map(|r| r.tenant.clone())
    }
}

#[async_trait]
impl SessionFence for InMemorySessionFence {
    async fn register_session(&self, session_id: &str, tenant: &str) -> AegisResult<()> {
        let tombs = self.tombstones.read().await;
        if tombs.contains(session_id) {
            return Err(AegisError::SessionTerminated(format!(
                "Cannot register session '{}': permanently tombstoned",
                session_id
            )));
        }
        drop(tombs);

        let mut s = self.sessions.write().await;
        s.insert(
            session_id.to_string(),
            SessionRecord {
                tenant: tenant.to_string(),
                status: SessionLifecycleStatus::Active,
                active_leases: HashSet::new(),
                next_lease_id: 1,
                state_store: HashMap::new(),
                termination_reason: None,
            },
        );
        Ok(())
    }

    async fn acquire_lease(&self, session_id: &str) -> AegisResult<u64> {
        let tombs = self.tombstones.read().await;
        if tombs.contains(session_id) {
            return Err(AegisError::SessionTerminated(format!(
                "Session '{}' is tombstoned; lease acquisition denied",
                session_id
            )));
        }
        drop(tombs);

        let mut s = self.sessions.write().await;
        let record = s.get_mut(session_id).ok_or_else(|| {
            AegisError::SessionRevoked(format!("Session '{}' does not exist", session_id))
        })?;

        match record.status {
            SessionLifecycleStatus::Active => {
                let lease_id = record.next_lease_id;
                record.next_lease_id += 1;
                record.active_leases.insert(lease_id);
                Ok(lease_id)
            }
            SessionLifecycleStatus::Terminating | SessionLifecycleStatus::Tombstoned => {
                Err(AegisError::SessionTerminated(format!(
                    "Session '{}' is in {:?} state; rejecting new lease",
                    session_id, record.status
                )))
            }
        }
    }

    async fn release_lease(&self, session_id: &str, lease_id: u64) -> AegisResult<()> {
        let mut s = self.sessions.write().await;
        if let Some(record) = s.get_mut(session_id) {
            record.active_leases.remove(&lease_id);
            if record.status == SessionLifecycleStatus::Terminating && record.active_leases.is_empty() {
                record.status = SessionLifecycleStatus::Tombstoned;
                record.state_store.clear();
                let mut tombs = self.tombstones.write().await;
                tombs.insert(session_id.to_string());
            }
        }
        Ok(())
    }

    async fn terminate_session(&self, session_id: &str, reason: &str) -> AegisResult<()> {
        let mut s = self.sessions.write().await;
        if let Some(record) = s.get_mut(session_id) {
            record.termination_reason = Some(reason.to_string());
            if record.active_leases.is_empty() {
                record.status = SessionLifecycleStatus::Tombstoned;
                record.state_store.clear();
                let mut tombs = self.tombstones.write().await;
                tombs.insert(session_id.to_string());
            } else {
                record.status = SessionLifecycleStatus::Terminating;
            }
        } else {
            let mut tombs = self.tombstones.write().await;
            tombs.insert(session_id.to_string());
        }
        Ok(())
    }

    async fn get_status(&self, session_id: &str) -> AegisResult<SessionLifecycleStatus> {
        let s = self.sessions.read().await;
        if let Some(rec) = s.get(session_id) {
            return Ok(rec.status);
        }
        drop(s);

        let tombs = self.tombstones.read().await;
        if tombs.contains(session_id) {
            Ok(SessionLifecycleStatus::Tombstoned)
        } else {
            Ok(SessionLifecycleStatus::Tombstoned)
        }
    }

    async fn write_session_state(&self, session_id: &str, key: &str, value: Value) -> AegisResult<()> {
        let tombs = self.tombstones.read().await;
        if tombs.contains(session_id) {
            return Err(AegisError::SessionTerminated(format!(
                "Session '{}' was terminated; refusing to recreate or mutate state",
                session_id
            )));
        }
        drop(tombs);

        let mut s = self.sessions.write().await;
        let record = s.get_mut(session_id).ok_or_else(|| {
            AegisError::SessionTerminated(format!(
                "Session '{}' does not exist; cannot write orphaned state",
                session_id
            ))
        })?;

        if record.status != SessionLifecycleStatus::Active {
            return Err(AegisError::SessionTerminated(format!(
                "Session '{}' is {:?}; state write rejected",
                session_id, record.status
            )));
        }

        record.state_store.insert(key.to_string(), value);
        Ok(())
    }

    async fn read_session_state(&self, session_id: &str, key: &str) -> AegisResult<Option<Value>> {
        let tombs = self.tombstones.read().await;
        if tombs.contains(session_id) {
            return Ok(None);
        }
        drop(tombs);

        let s = self.sessions.read().await;
        if let Some(record) = s.get(session_id) {
            if record.status == SessionLifecycleStatus::Active {
                return Ok(record.state_store.get(key).cloned());
            }
        }
        Ok(None)
    }

    async fn active_leases(&self, session_id: &str) -> usize {
        let s = self.sessions.read().await;
        s.get(session_id).map(|r| r.active_leases.len()).unwrap_or(0)
    }
}
