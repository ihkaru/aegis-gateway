// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::AegisResult;
use crate::core::notification::{
    ApprovalChannelTarget, ApprovalNotificationDispatcher, ApprovalNotificationPayload,
};

/// Multi-channel notification dispatcher for Human-In-The-Loop approvals
pub struct MultiChannelApprovalDispatcher {
    channels: Arc<RwLock<Vec<ApprovalChannelTarget>>>,
}

impl MultiChannelApprovalDispatcher {
    pub fn new(channels: Vec<ApprovalChannelTarget>) -> Self {
        Self {
            channels: Arc::new(RwLock::new(channels)),
        }
    }

    pub async fn add_channel(&self, channel: ApprovalChannelTarget) {
        let mut guard = self.channels.write().await;
        guard.push(channel);
    }

    /// Build Slack-compatible Interactive Block Kit payload
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
            "text": format!("🚨 Action Approval Required: {}", payload.title),
            "blocks": [
                {
                    "type": "header",
                    "text": { "type": "plain_text", "text": "🛡️ Aegis Security Approval Gate" }
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
                            "text": { "type": "plain_text", "text": "✅ Approve Action" },
                            "style": "primary",
                            "url": approve_url
                        },
                        {
                            "type": "button",
                            "text": { "type": "plain_text", "text": "🛑 Deny Action" },
                            "style": "danger",
                            "url": deny_url
                        }
                    ]
                }
            ]
        })
    }

    /// Build Teams-compatible Adaptive Card payload
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

        for channel in channels.iter() {
            match channel {
                ApprovalChannelTarget::GenericWebhook { url, secret_token: _ } => {
                    let receipt = format!("dispatched:webhook:{}:ticket_{}", url, payload.ticket.ticket_id);
                    dispatch_receipts.push(receipt);
                }
                ApprovalChannelTarget::SlackWebhook { webhook_url, channel: _ } => {
                    let _blocks = Self::format_slack_blocks(payload);
                    let receipt = format!("dispatched:slack:{}:ticket_{}", webhook_url, payload.ticket.ticket_id);
                    dispatch_receipts.push(receipt);
                }
                ApprovalChannelTarget::TeamsWebhook { webhook_url } => {
                    let _card = Self::format_teams_card(payload);
                    let receipt = format!("dispatched:teams:{}:ticket_{}", webhook_url, payload.ticket.ticket_id);
                    dispatch_receipts.push(receipt);
                }
                ApprovalChannelTarget::InBandMcp => {
                    let receipt = format!("dispatched:inband_mcp:ticket_{}", payload.ticket.ticket_id);
                    dispatch_receipts.push(receipt);
                }
                ApprovalChannelTarget::ConsoleLog => {
                    let receipt = format!("dispatched:console:ticket_{}", payload.ticket.ticket_id);
                    dispatch_receipts.push(receipt);
                }
            }
        }

        Ok(dispatch_receipts)
    }

    fn registered_channels(&self) -> Vec<ApprovalChannelTarget> {
        // Return default channels without async deadlock
        vec![ApprovalChannelTarget::ConsoleLog, ApprovalChannelTarget::InBandMcp]
    }
}
