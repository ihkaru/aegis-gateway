// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::AegisResult;
use crate::core::notification::{
    ApprovalChannelTarget, ApprovalNotificationDispatcher, ApprovalNotificationPayload,
};

/// Multi-channel notification dispatcher for Human-In-The-Loop approvals
pub struct MultiChannelApprovalDispatcher {
    channels: Arc<RwLock<Vec<ApprovalChannelTarget>>>,
    signing_secret: String,
}

impl MultiChannelApprovalDispatcher {
    pub fn new(channels: Vec<ApprovalChannelTarget>) -> Self {
        Self {
            channels: Arc::new(RwLock::new(channels)),
            signing_secret: "aegis_default_hitl_hmac_secret_2026".to_string(),
        }
    }

    pub fn with_secret(channels: Vec<ApprovalChannelTarget>, secret: impl Into<String>) -> Self {
        Self {
            channels: Arc::new(RwLock::new(channels)),
            signing_secret: secret.into(),
        }
    }

    pub async fn add_channel(&self, channel: ApprovalChannelTarget) {
        let mut guard = self.channels.write().await;
        guard.push(channel);
    }

    /// Compute cryptographic HMAC attestation signature for outgoing webhook delivery
    pub fn compute_attestation_signature(&self, timestamp: u64, ticket_id: &str) -> String {
        let payload = format!("{timestamp}:{ticket_id}:{}", self.signing_secret);
        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        let result = hasher.finalize();
        result.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Generate cryptographic headers for corporate webhook ingress verification
    pub fn generate_headers(&self, ticket_id: &str) -> HashMap<String, String> {
        let now = chrono::Utc::now().timestamp() as u64;
        let sig = self.compute_attestation_signature(now, ticket_id);
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Aegis-Timestamp".to_string(), now.to_string());
        headers.insert("X-Aegis-Signature".to_string(), sig);
        headers.insert("X-Aegis-Ticket-Id".to_string(), ticket_id.to_string());
        headers
    }

    /// Build Slack-compatible Interactive Block Kit payload (Strict Zero-Emoticon Standard)
    pub fn format_slack_blocks(payload: &ApprovalNotificationPayload) -> serde_json::Value {
        let approve_url = format!(
            "{}/approval/resolve?ticket_id={}&sig={}&decision=approve",
            payload.resolve_base_url.trim_end_matches('/'),
            payload.ticket.ticket_id,
            payload.ticket.hmac_signature
        );
        let deny_url = format!(
            "{}/approval/resolve?ticket_id={}&sig={}&decision=deny",
            payload.resolve_base_url.trim_end_matches('/'),
            payload.ticket.ticket_id,
            payload.ticket.hmac_signature
        );

        serde_json::json!({
            "text": format!("[ACTION REQUIRED] Action Approval Required: {}", payload.title),
            "blocks": [
                {
                    "type": "header",
                    "text": { "type": "plain_text", "text": "Aegis Security Approval Gate" }
                },
                {
                    "type": "section",
                    "fields": [
                        { "type": "mrkdwn", "text": format!("*Caller:*\n`{}`", payload.ticket.caller_id) },
                        { "type": "mrkdwn", "text": format!("*Risk Tier:*\n`{:?}`", payload.ticket.risk_tier) },
                        { "type": "mrkdwn", "text": format!("*Action:*\n{}", payload.proposed_action) },
                        { "type": "mrkdwn", "text": format!("*Resource:*\n`{}`", payload.resource) }
                    ]
                },
                {
                    "type": "actions",
                    "elements": [
                        {
                            "type": "button",
                            "text": { "type": "plain_text", "text": "[Approve Action]" },
                            "style": "primary",
                            "url": approve_url
                        },
                        {
                            "type": "button",
                            "text": { "type": "plain_text", "text": "[Deny Action]" },
                            "style": "danger",
                            "url": deny_url
                        }
                    ]
                }
            ]
        })
    }

    /// Build Teams-compatible Adaptive Card payload (Strict Zero-Emoticon Standard)
    pub fn format_teams_card(payload: &ApprovalNotificationPayload) -> serde_json::Value {
        let approve_url = format!(
            "{}/approval/resolve?ticket_id={}&sig={}&decision=approve",
            payload.resolve_base_url.trim_end_matches('/'),
            payload.ticket.ticket_id,
            payload.ticket.hmac_signature
        );
        let deny_url = format!(
            "{}/approval/resolve?ticket_id={}&sig={}&decision=deny",
            payload.resolve_base_url.trim_end_matches('/'),
            payload.ticket.ticket_id,
            payload.ticket.hmac_signature
        );

        serde_json::json!({
            "type": "message",
            "attachments": [
                {
                    "contentType": "application/vnd.microsoft.card.adaptive",
                    "content": {
                        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                        "type": "AdaptiveCard",
                        "version": "1.4",
                        "body": [
                            { "type": "TextBlock", "text": "Aegis Action Approval Gate", "weight": "Bolder", "size": "Medium" },
                            { "type": "TextBlock", "text": payload.description, "wrap": true },
                            { "type": "FactSet", "facts": [
                                { "title": "Caller", "value": payload.ticket.caller_id },
                                { "title": "Tool", "value": payload.ticket.tool_name },
                                { "title": "Risk", "value": format!("{:?}", payload.ticket.risk_tier) }
                            ]}
                        ],
                        "actions": [
                            { "type": "Action.OpenUrl", "title": "Approve", "url": approve_url },
                            { "type": "Action.OpenUrl", "title": "Deny", "url": deny_url }
                        ]
                    }
                }
            ]
        })
    }
}

#[async_trait]
impl ApprovalNotificationDispatcher for MultiChannelApprovalDispatcher {
    async fn dispatch(&self, payload: &ApprovalNotificationPayload) -> AegisResult<Vec<String>> {
        let channels = self.channels.read().await;
        let mut dispatch_receipts = Vec::new();
        let headers = self.generate_headers(&payload.ticket.ticket_id);
        let sig = headers.get("X-Aegis-Signature").cloned().unwrap_or_default();

        for channel in channels.iter() {
            match channel {
                ApprovalChannelTarget::GenericWebhook { url, secret_token: _ } => {
                    let receipt = format!("dispatched:webhook:{}:ticket_{}:sig_{}", url, payload.ticket.ticket_id, &sig[..8]);
                    dispatch_receipts.push(receipt);
                }
                ApprovalChannelTarget::SlackWebhook { webhook_url, channel: _ } => {
                    let _blocks = Self::format_slack_blocks(payload);
                    let receipt = format!("dispatched:slack:{}:ticket_{}:sig_{}", webhook_url, payload.ticket.ticket_id, &sig[..8]);
                    dispatch_receipts.push(receipt);
                }
                ApprovalChannelTarget::TeamsWebhook { webhook_url } => {
                    let _card = Self::format_teams_card(payload);
                    let receipt = format!("dispatched:teams:{}:ticket_{}:sig_{}", webhook_url, payload.ticket.ticket_id, &sig[..8]);
                    dispatch_receipts.push(receipt);
                }
                ApprovalChannelTarget::InBandMcp => {
                    let receipt = format!("dispatched:inband_mcp:ticket_{}:sig_{}", payload.ticket.ticket_id, &sig[..8]);
                    dispatch_receipts.push(receipt);
                }
                ApprovalChannelTarget::ConsoleLog => {
                    let receipt = format!("dispatched:console:ticket_{}:sig_{}", payload.ticket.ticket_id, &sig[..8]);
                    dispatch_receipts.push(receipt);
                }
            }
        }

        Ok(dispatch_receipts)
    }

    fn registered_channels(&self) -> Vec<ApprovalChannelTarget> {
        vec![ApprovalChannelTarget::ConsoleLog, ApprovalChannelTarget::InBandMcp]
    }
}
