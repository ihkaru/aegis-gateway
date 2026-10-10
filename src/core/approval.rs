// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::core::error::AegisResult;

/// Action risk tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskTier {
    Low,
    Medium,
    High,
    Critical,
}

/// Dynamic approval decision
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    AutoApprove,
    RequireApproval(ApprovalTicket),
    Deny { reason: String },
}

/// Cryptographically signed approval ticket for suspended tasks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalTicket {
    pub ticket_id: String,
    pub session_id: String,
    pub caller_id: String,
    pub tool_name: String,
    pub action_summary: String,
    pub risk_tier: RiskTier,
    pub hmac_signature: String,
    pub expires_at_epoch_secs: u64,
}

/// Abstract Human-in-the-Loop (HITL) gate contract
#[async_trait]
pub trait ApprovalGate: Send + Sync {
    /// Classify action payload and evaluate approval requirement
    async fn evaluate_action(
        &self,
        caller_id: &str,
        session_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> AegisResult<ApprovalDecision>;

    /// Validate cryptographic approval and determine if execution may resume
    async fn resolve_ticket(
        &self,
        ticket_id: &str,
        signature: &str,
        approved: bool,
    ) -> AegisResult<bool>;

    /// List currently suspended pending tickets awaiting authorization
    async fn list_pending_tickets(&self) -> AegisResult<Vec<ApprovalTicket>> {
        Ok(vec![])
    }
}
