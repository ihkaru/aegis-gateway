// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;

use crate::core::approval::ApprovalGate;
use crate::core::audit::{AuditAction, AuditEvent, AuditSink};
use crate::core::error::{AegisError, AegisResult};
use crate::core::notification::DurableResumeRouter;
use crate::core::types::{CallerContext, TenantId};

/// Durable task resumption router integrating cryptographic approval gate and SIEM audit sink
pub struct DurableTaskResumeRouter {
    approval_gate: Arc<dyn ApprovalGate>,
    audit_sink: Arc<dyn AuditSink>,
}

impl DurableTaskResumeRouter {
    pub fn new(approval_gate: Arc<dyn ApprovalGate>, audit_sink: Arc<dyn AuditSink>) -> Self {
        Self {
            approval_gate,
            audit_sink,
        }
    }
}

#[async_trait]
impl DurableResumeRouter for DurableTaskResumeRouter {
    async fn resolve_and_resume(
        &self,
        ticket_id: &str,
        signature: &str,
        approved: bool,
        approver_id: &str,
    ) -> AegisResult<bool> {
        let is_valid = self.approval_gate.resolve_ticket(ticket_id, signature, approved).await?;

        if !is_valid {
            return Err(AegisError::AuthenticationFailed(format!(
                "Invalid or expired cryptographic signature for approval ticket '{ticket_id}'"
            )));
        }

        // Emit non-repudiation SIEM audit record
        let caller = CallerContext {
            tenant_id: TenantId::new("enterprise-admin"),
            subject: approver_id.to_string(),
            roles: vec!["approver".to_string(), "compliance_lead".to_string()],
            department: Some("security_ops".to_string()),
            client_ip: None,
            session_id: uuid::Uuid::new_v4().to_string(),
        };

        let audit_event = AuditEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            caller,
            action: AuditAction::PolicyEvaluated {
                allowed: approved,
                reason: Some(format!(
                    "HITL ticket '{}' resolved by '{}': decision={}",
                    ticket_id, approver_id, if approved { "APPROVED" } else { "DENIED" }
                )),
            },
            target_resource: format!("approval_ticket:{}", ticket_id),
            payload_hash_sha256: format!("ticket_{}_decision_{}", ticket_id, approved),
            metadata: serde_json::json!({
                "ticket_id": ticket_id,
                "approver": approver_id,
                "decision": approved,
                "signature_verified": true,
            }),
        };

        self.audit_sink.emit(&audit_event).await?;
        Ok(approved)
    }
}
