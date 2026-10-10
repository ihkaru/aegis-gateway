// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::approval::{ApprovalDecision, ApprovalGate, ApprovalTicket, RiskTier};
use crate::core::error::{AegisError, AegisResult};

/// Concrete implementation of Human-in-the-Loop policy gate
pub struct ActionApprovalGate {
    signing_secret: String,
    tickets: Arc<RwLock<HashMap<String, ApprovalTicket>>>,
    ticket_ttl_secs: u64,
}

impl ActionApprovalGate {
    pub fn new(signing_secret: impl Into<String>) -> Self {
        Self {
            signing_secret: signing_secret.into(),
            tickets: Arc::new(RwLock::new(HashMap::new())),
            ticket_ttl_secs: 900, // 15 minutes default TTL
        }
    }

    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.ticket_ttl_secs = ttl_secs;
        self
    }

    fn generate_signature(&self, ticket_id: &str, caller_id: &str, tool_name: &str, expires: u64) -> String {
        let mut hasher = Sha256::new();
        let payload = format!("{ticket_id}:{caller_id}:{tool_name}:{expires}:{}", self.signing_secret);
        hasher.update(payload.as_bytes());
        let result = hasher.finalize();
        result.iter().fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{:02x}", b);
            acc
        })
    }

    fn assess_risk_tier(&self, tool: &str, arguments: &serde_json::Value) -> (RiskTier, String) {
        let arg_str = arguments.to_string().to_lowercase();
        let tool_lower = tool.to_lowercase();

        // 1. Critical Risk: Public Sharing to Anyone With Link
        if arg_str.contains("\"anyone\"") && (arg_str.contains("\"writer\"") || arg_str.contains("\"editor\"")) {
            return (RiskTier::Critical, "Public link sharing with write/edit access to unauthorized parties".to_string());
        }

        // 2. High Risk: Destructive Actions / Deletions / Privilege Escalation
        if tool_lower.contains("delete") || tool_lower.contains("drop") || tool_lower.contains("purge") || arg_str.contains("rm -rf") {
            return (RiskTier::High, format!("Destructive or unrecoverable operation via tool '{tool}'"));
        }

        if tool_lower.contains("data_egress") || arg_str.contains("egress_raw_dataset") {
            return (RiskTier::High, "Raw restricted data egress exceeding security threshold".to_string());
        }

        if arg_str.contains("iam") || arg_str.contains("grant_admin") || tool_lower.contains("grant_permission") {
            return (RiskTier::High, "Privilege escalation or IAM modification".to_string());
        }

        (RiskTier::Low, "Standard read/write operation within permitted boundaries".to_string())
    }
}

#[async_trait]
impl ApprovalGate for ActionApprovalGate {
    async fn evaluate_action(
        &self,
        caller_id: &str,
        session_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> AegisResult<ApprovalDecision> {
        let (tier, reason) = self.assess_risk_tier(tool_name, arguments);

        if tier == RiskTier::Low {
            return Ok(ApprovalDecision::AutoApprove);
        }

        let now_epoch = chrono::Utc::now().timestamp() as u64;
        let expires_at = now_epoch + self.ticket_ttl_secs;
        let ticket_id = uuid::Uuid::new_v4().to_string();
        let hmac_signature = self.generate_signature(&ticket_id, caller_id, tool_name, expires_at);

        let ticket = ApprovalTicket {
            ticket_id: ticket_id.clone(),
            session_id: session_id.to_string(),
            caller_id: caller_id.to_string(),
            tool_name: tool_name.to_string(),
            action_summary: reason,
            risk_tier: tier,
            hmac_signature,
            expires_at_epoch_secs: expires_at,
        };

        let mut write = self.tickets.write().await;
        write.insert(ticket_id, ticket.clone());

        Ok(ApprovalDecision::RequireApproval(ticket))
    }

    async fn resolve_ticket(
        &self,
        ticket_id: &str,
        signature: &str,
        approved: bool,
    ) -> AegisResult<bool> {
        let mut write = self.tickets.write().await;
        let ticket = write
            .get(ticket_id)
            .ok_or_else(|| AegisError::PolicyDenied(format!("Approval ticket '{ticket_id}' not found")))?;

        let now_epoch = chrono::Utc::now().timestamp() as u64;
        if now_epoch > ticket.expires_at_epoch_secs {
            write.remove(ticket_id);
            return Err(AegisError::PolicyDenied(format!("Approval ticket '{ticket_id}' has expired")));
        }

        let expected_sig = self.generate_signature(ticket_id, &ticket.caller_id, &ticket.tool_name, ticket.expires_at_epoch_secs);
        if signature != expected_sig {
            return Err(AegisError::PolicyDenied("Cryptographic approval signature mismatch".to_string()));
        }

        write.remove(ticket_id);
        Ok(approved)
    }
}
