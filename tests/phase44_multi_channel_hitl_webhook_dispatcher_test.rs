// SPDX-License-Identifier: MIT
//! Phase 44: Multi-Channel HITL Webhook Dispatcher Test Suite
//! Validates Slack BlockKit, Teams Adaptive Cards, HMAC attestation, and multi-channel dispatching.

use propylon::core::approval::{ApprovalTicket, RiskTier};
use propylon::core::notification::{ApprovalChannelTarget, ApprovalNotificationDispatcher, ApprovalNotificationPayload};
use propylon::policy::approval_dispatcher::MultiChannelApprovalDispatcher;

fn make_test_payload() -> ApprovalNotificationPayload {
    let ticket = ApprovalTicket {
        ticket_id: "tkt_hitl_987654".to_string(),
        session_id: "sess_prod_123".to_string(),
        caller_id: "autonomous_agent_01".to_string(),
        tool_name: "destructive_drop_table".to_string(),
        action_summary: "Drop production transactional table".to_string(),
        risk_tier: RiskTier::Critical,
        hmac_signature: "hmac_sig_abcdef123456".to_string(),
        expires_at_epoch_secs: 1770000000,
    };

    ApprovalNotificationPayload {
        ticket,
        title: "Table Drop Authorization Request".to_string(),
        description: "Agent requested destructive drop of production database table".to_string(),
        proposed_action: "DROP TABLE production_ledger".to_string(),
        resource: "db:production_ledger".to_string(),
        resolve_base_url: "https://propylon.enterprise.internal:8485".to_string(),
        created_at_epoch_secs: 1769999000,
    }
}

#[test]
fn test_phase44_slack_block_kit_formatting_zero_emoticon() {
    let payload = make_test_payload();
    let blocks = MultiChannelApprovalDispatcher::format_slack_blocks(&payload);

    let json_str = serde_json::to_string(&blocks).expect("serialize slack blocks");
    // Verify zero emoticons
    assert!(!json_str.contains("🚨"));
    assert!(!json_str.contains("🛡️"));
    assert!(!json_str.contains("✅"));
    assert!(!json_str.contains("🛑"));

    // Verify key fields
    assert!(json_str.contains("Action Approval Required"));
    assert!(json_str.contains("destructive_drop_table") || json_str.contains("DROP TABLE"));
    assert!(json_str.contains("Critical"));
    assert!(json_str.contains("approve"));
    assert!(json_str.contains("deny"));
}

#[test]
fn test_phase44_teams_adaptive_card_formatting() {
    let payload = make_test_payload();
    let card = MultiChannelApprovalDispatcher::format_teams_card(&payload);

    let json_str = serde_json::to_string(&card).expect("serialize teams card");
    assert!(json_str.contains("application/vnd.microsoft.card.adaptive"));
    assert!(json_str.contains("AdaptiveCard"));
    assert!(json_str.contains("autonomous_agent_01"));
    assert!(json_str.contains("destructive_drop_table"));
    assert!(json_str.contains("Approve"));
    assert!(json_str.contains("Deny"));
}

#[test]
fn test_phase44_hmac_attestation_signature_verification() {
    let dispatcher = MultiChannelApprovalDispatcher::with_secret(
        vec![ApprovalChannelTarget::ConsoleLog],
        "my_enterprise_secret_key_888",
    );

    let ts = 1769999999;
    let ticket_id = "tkt_test_777";
    let sig1 = dispatcher.compute_attestation_signature(ts, ticket_id);
    let sig2 = dispatcher.compute_attestation_signature(ts, ticket_id);
    assert_eq!(sig1, sig2, "HMAC must be deterministic");
    assert_eq!(sig1.len(), 64, "SHA-256 hex must be 64 characters");

    // Different timestamp or ticket yields different signature
    let sig3 = dispatcher.compute_attestation_signature(ts + 1, ticket_id);
    assert_ne!(sig1, sig3);

    // Verify header generation
    let headers = dispatcher.generate_headers(ticket_id);
    assert!(headers.contains_key("X-Propylon-Signature"));
    assert!(headers.contains_key("X-Propylon-Timestamp"));
    assert_eq!(headers.get("X-Propylon-Ticket-Id").unwrap(), ticket_id);
}

#[tokio::test]
async fn test_phase44_multi_channel_dispatch_receipts() {
    let channels = vec![
        ApprovalChannelTarget::SlackWebhook {
            webhook_url: "https://hooks.slack.com/services/T00/B00/X00".to_string(),
            channel: Some("#security-alerts".to_string()),
        },
        ApprovalChannelTarget::TeamsWebhook {
            webhook_url: "https://outlook.office.com/webhook/test-teams".to_string(),
        },
        ApprovalChannelTarget::GenericWebhook {
            url: "https://api.pagerduty.com/v2/enqueue".to_string(),
            secret_token: Some("secret_pd".to_string()),
        },
        ApprovalChannelTarget::InBandMcp,
        ApprovalChannelTarget::ConsoleLog,
    ];

    let dispatcher = MultiChannelApprovalDispatcher::with_secret(channels, "test_secret_999");
    let payload = make_test_payload();

    let receipts = dispatcher
        .dispatch(&payload)
        .await
        .expect("dispatch across channels must succeed");

    assert_eq!(receipts.len(), 5);
    assert!(receipts[0].starts_with("dispatched:slack:"));
    assert!(receipts[1].starts_with("dispatched:teams:"));
    assert!(receipts[2].starts_with("dispatched:webhook:"));
    assert!(receipts[3].starts_with("dispatched:inband_mcp:"));
    assert!(receipts[4].starts_with("dispatched:console:"));

    // Every receipt contains the ticket_id and signature prefix
    for r in &receipts {
        assert!(r.contains("tkt_hitl_987654"));
        assert!(r.contains(":sig_"));
    }
}
