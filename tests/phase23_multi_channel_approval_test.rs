// SPDX-License-Identifier: MIT

use std::sync::Arc;
use aegis_gateway::audit::StructuredAuditLogger;
use aegis_gateway::core::approval::{ApprovalDecision, ApprovalGate, RiskTier};
use aegis_gateway::core::notification::{
    ApprovalChannelTarget, ApprovalNotificationDispatcher, ApprovalNotificationPayload,
    DurableResumeRouter,
};
use aegis_gateway::policy::{
    ActionApprovalGate, DurableTaskResumeRouter, MultiChannelApprovalDispatcher,
};

#[tokio::test]
async fn test_multi_channel_dispatcher_formatting_and_receipts() {
    let gate = ActionApprovalGate::new("phase23_test_hmac_secret");
    let ticket = match gate
        .evaluate_action(
            "alice@company.com",
            "session-xyz",
            "gdrive_delete_folder",
            &serde_json::json!({ "action": "delete_folder", "path": "/Root/Confidential" }),
        )
        .await
        .expect("Evaluation should succeed")
    {
        ApprovalDecision::RequireApproval(t) => t,
        other => panic!("Expected RequireApproval, got {:?}", other),
    };

    let payload = ApprovalNotificationPayload {
        ticket: ticket.clone(),
        title: "Delete Confidential Folder".to_string(),
        description: "Agent requests permission to recursively delete /Root/Confidential".to_string(),
        proposed_action: "DELETE /Root/Confidential".to_string(),
        resource: "gdrive://Confidential".to_string(),
        resolve_base_url: "https://mcp.dvlpid.my.id".to_string(),
        created_at_epoch_secs: 1728560000,
    };

    // 1. Verify Slack Blocks Format
    let slack_blocks = MultiChannelApprovalDispatcher::format_slack_blocks(&payload);
    let slack_json = serde_json::to_string(&slack_blocks).expect("JSON serialization");
    assert!(slack_json.contains("Aegis Security Approval Gate"));
    assert!(slack_json.contains("decision=approve"));
    assert!(slack_json.contains("decision=deny"));
    assert!(slack_json.contains(&ticket.hmac_signature));

    // 2. Verify Teams Adaptive Card Format
    let teams_card = MultiChannelApprovalDispatcher::format_teams_card(&payload);
    let teams_json = serde_json::to_string(&teams_card).expect("JSON serialization");
    assert!(teams_json.contains("AdaptiveCard"));
    assert!(teams_json.contains("alice@company.com"));

    // 3. Dispatch to multiple registered channels
    let dispatcher = MultiChannelApprovalDispatcher::new(vec![
        ApprovalChannelTarget::SlackWebhook {
            webhook_url: "https://hooks.slack.com/services/test".to_string(),
            channel: Some("#security-approvals".to_string()),
        },
        ApprovalChannelTarget::TeamsWebhook {
            webhook_url: "https://outlook.office.com/webhook/test".to_string(),
        },
        ApprovalChannelTarget::GenericWebhook {
            url: "https://internal-ops.corp/webhooks/approvals".to_string(),
            secret_token: Some("secret-token-123".to_string()),
        },
        ApprovalChannelTarget::InBandMcp,
        ApprovalChannelTarget::ConsoleLog,
    ]);

    let receipts = dispatcher
        .dispatch(&payload)
        .await
        .expect("Dispatch across channels should succeed");

    assert_eq!(receipts.len(), 5);
    assert!(receipts[0].starts_with("dispatched:slack:"));
    assert!(receipts[1].starts_with("dispatched:teams:"));
    assert!(receipts[2].starts_with("dispatched:webhook:"));
    assert!(receipts[3].starts_with("dispatched:inband_mcp:"));
    assert!(receipts[4].starts_with("dispatched:console:"));
}

#[tokio::test]
async fn test_durable_task_resumption_with_valid_hmac() {
    let gate = Arc::new(ActionApprovalGate::new("resume_router_test_secret"));
    let audit = Arc::new(StructuredAuditLogger::new());
    let router = DurableTaskResumeRouter::new(gate.clone(), audit.clone());

    let ticket = match gate
        .evaluate_action(
            "bob@company.com",
            "session-bob",
            "share_public",
            &serde_json::json!({ "type": "anyone", "role": "writer" }),
        )
        .await
        .expect("Evaluation should succeed")
    {
        ApprovalDecision::RequireApproval(t) => t,
        other => panic!("Expected RequireApproval, got {:?}", other),
    };

    assert_eq!(ticket.risk_tier, RiskTier::Critical);

    // Resolve with valid HMAC signature
    let success = router
        .resolve_and_resume(
            &ticket.ticket_id,
            &ticket.hmac_signature,
            true,
            "security-manager@company.com",
        )
        .await
        .expect("Resumption with valid signature should succeed");

    assert!(success);
}

#[tokio::test]
async fn test_durable_task_resumption_fails_closed_on_tampered_signature() {
    let gate = Arc::new(ActionApprovalGate::new("resume_router_test_secret"));
    let audit = Arc::new(StructuredAuditLogger::new());
    let router = DurableTaskResumeRouter::new(gate.clone(), audit.clone());

    let ticket = match gate
        .evaluate_action(
            "eve@company.com",
            "session-eve",
            "destructive_drop",
            &serde_json::json!({ "action": "delete_all" }),
        )
        .await
        .expect("Evaluation should succeed")
    {
        ApprovalDecision::RequireApproval(t) => t,
        other => panic!("Expected RequireApproval, got {:?}", other),
    };

    // Forged signature
    let forged_signature = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    let err = router
        .resolve_and_resume(
            &ticket.ticket_id,
            forged_signature,
            true,
            "attacker@unauthorized.com",
        )
        .await
        .expect_err("Resumption with forged signature MUST fail closed");

    assert!(err.to_string().contains("signature"));
}
