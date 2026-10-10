// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::core::approval::ApprovalTicket;
use crate::core::error::AegisResult;

/// Pluggable notification channel destinations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalChannelTarget {
    GenericWebhook {
        url: String,
        secret_token: Option<String>,
    },
    SlackWebhook {
        webhook_url: String,
        channel: Option<String>,
    },
    TeamsWebhook {
        webhook_url: String,
    },
    InBandMcp,
    ConsoleLog,
}

/// Rich payload for human-in-the-loop review
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalNotificationPayload {
    pub ticket: ApprovalTicket,
    pub title: String,
    pub description: String,
    pub proposed_action: String,
    pub resource: String,
    pub resolve_base_url: String,
    pub created_at_epoch_secs: u64,
}

/// Abstract contract for dispatching approval notifications across channels
#[async_trait]
pub trait ApprovalNotificationDispatcher: Send + Sync {
    /// Dispatch notification across all registered targets
    async fn dispatch(&self, payload: &ApprovalNotificationPayload) -> AegisResult<Vec<String>>;

    /// List active configured notification targets
    fn registered_channels(&self) -> Vec<ApprovalChannelTarget>;
}

/// Abstract contract for resuming suspended tasks post-approval
#[async_trait]
pub trait DurableResumeRouter: Send + Sync {
    /// Verify approval decision, unblock suspended task, and emit audit
    async fn resolve_and_resume(
        &self,
        ticket_id: &str,
        signature: &str,
        approved: bool,
        approver_id: &str,
    ) -> AegisResult<bool>;
}
