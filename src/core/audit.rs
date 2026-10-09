// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::core::error::AegisResult;
use crate::core::types::CallerContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    ToolDiscovered,
    ToolInvoked,
    PolicyEvaluated { allowed: bool, reason: Option<String> },
    DlpRedacted { categories: Vec<String> },
    CircuitTripped { backend: String },
    SkillLoaded { skill_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub caller: CallerContext,
    pub action: AuditAction,
    pub target_resource: String,
    pub payload_hash_sha256: String,
    pub metadata: Value,
}

/// Pluggable enterprise audit streaming sink (Splunk, Datadog, Elastic, OTel)
#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn emit(&self, event: &AuditEvent) -> AegisResult<()>;
}
