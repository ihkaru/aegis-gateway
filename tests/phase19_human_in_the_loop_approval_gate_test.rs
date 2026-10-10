// SPDX-License-Identifier: MIT

use aegis_gateway::core::approval::{ApprovalDecision, ApprovalGate, RiskTier};
use aegis_gateway::policy::ActionApprovalGate;

#[tokio::test]
async fn test_approval_gate_low_risk_auto_approve() {
    let gate = ActionApprovalGate::new("hmac_test_secret_key_123");

    let low_risk_payload = serde_json::json!({
        "spreadsheet_id": "12345",
        "range": "A1:B2",
        "values": [["Item", "Price"]]
    });

    let decision = gate
        .evaluate_action("alice@company.com", "sess-1", "sheets_append", &low_risk_payload)
        .await
        .unwrap();

    assert_eq!(decision, ApprovalDecision::AutoApprove);
}

#[tokio::test]
async fn test_approval_gate_critical_public_share_requires_approval() {
    let gate = ActionApprovalGate::new("hmac_test_secret_key_123");

    let public_share_payload = serde_json::json!({
        "file_id": "sensitive_doc_id",
        "type": "anyone",
        "role": "writer"
    });

    let decision = gate
        .evaluate_action("bob@company.com", "sess-2", "drive_set_permissions", &public_share_payload)
        .await
        .unwrap();

    match decision {
        ApprovalDecision::RequireApproval(ticket) => {
            assert_eq!(ticket.risk_tier, RiskTier::Critical);
            assert_eq!(ticket.caller_id, "bob@company.com");
            assert!(ticket.action_summary.contains("Public link sharing"));

            // 1. Resolve with valid cryptographic signature -> approve
            let approved = gate
                .resolve_ticket(&ticket.ticket_id, &ticket.hmac_signature, true)
                .await
                .unwrap();
            assert!(approved);
        }
        other => panic!("Expected RequireApproval for public sharing, got {:?}", other),
    }
}

#[tokio::test]
async fn test_approval_gate_destructive_action_rejection() {
    let gate = ActionApprovalGate::new("hmac_test_secret_key_123");

    let delete_payload = serde_json::json!({
        "database": "production_users",
        "cascade": true
    });

    let decision = gate
        .evaluate_action("developer@company.com", "sess-3", "database_delete_table", &delete_payload)
        .await
        .unwrap();

    match decision {
        ApprovalDecision::RequireApproval(ticket) => {
            assert_eq!(ticket.risk_tier, RiskTier::High);

            // 2. Resolve with reject -> rejected
            let rejected = gate
                .resolve_ticket(&ticket.ticket_id, &ticket.hmac_signature, false)
                .await
                .unwrap();
            assert!(!rejected);
        }
        other => panic!("Expected RequireApproval for destructive tool, got {:?}", other),
    }
}

#[tokio::test]
async fn test_approval_gate_signature_mismatch_fails_closed() {
    let gate = ActionApprovalGate::new("hmac_test_secret_key_123");

    let delete_payload = serde_json::json!({ "file": "important.pdf" });
    let decision = gate
        .evaluate_action("user@company.com", "sess-4", "delete_file", &delete_payload)
        .await
        .unwrap();

    if let ApprovalDecision::RequireApproval(ticket) = decision {
        let fake_sig = "0123456789abcdef0123456789abcdef";
        let res = gate.resolve_ticket(&ticket.ticket_id, fake_sig, true).await;
        assert!(res.is_err());
        assert!(res.err().unwrap().to_string().contains("signature mismatch"));
    }
}
