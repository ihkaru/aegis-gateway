// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::AegisResult;
use crate::core::session::SessionRevocationRegistry;

/// Thread-safe in-memory session revocation and SCIM deactivation registry
#[derive(Clone, Default)]
pub struct MemoryRevocationRegistry {
    revoked_sessions: Arc<RwLock<HashSet<String>>>,
    deactivated_subjects: Arc<RwLock<HashSet<String>>>,
}

impl MemoryRevocationRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SessionRevocationRegistry for MemoryRevocationRegistry {
    async fn is_revoked(&self, session_id: &str, subject: &str) -> AegisResult<bool> {
        let sessions = self.revoked_sessions.read().await;
        if sessions.contains(session_id) {
            return Ok(true);
        }

        let subjects = self.deactivated_subjects.read().await;
        if subjects.contains(subject) {
            return Ok(true);
        }

        Ok(false)
    }

    async fn revoke_session(&self, session_id: &str, _reason: &str) -> AegisResult<()> {
        let mut sessions = self.revoked_sessions.write().await;
        sessions.insert(session_id.to_string());
        Ok(())
    }

    async fn deactivate_subject(&self, subject: &str, _reason: &str) -> AegisResult<()> {
        let mut subjects = self.deactivated_subjects.write().await;
        subjects.insert(subject.to_string());
        Ok(())
    }
}
