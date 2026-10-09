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

/// Cryptographically chained audit event satisfying SOC 2 Type II non-repudiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashChainedEvent {
    pub sequence: u64,
    pub previous_hash: String,
    pub event_hash: String,
    pub event: AuditEvent,
}

/// OpenTelemetry W3C distributed trace context header representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelTraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub traceparent: String,
}

/// Verifier contract for validating integrity of sequential cryptographic log chains
pub trait AuditChainVerifier: Send + Sync {
    fn verify_chain(&self, events: &[HashChainedEvent]) -> AegisResult<bool>;
}

/// Pluggable enterprise audit streaming sink (Splunk, Datadog, Elastic, OTel)
#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn emit(&self, event: &AuditEvent) -> AegisResult<()>;
}

