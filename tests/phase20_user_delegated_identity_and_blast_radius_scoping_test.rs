// SPDX-License-Identifier: MIT

use aegis_gateway::core::delegation::{
    DelegatedToken, IdentityDelegationBroker, ResourceScoper,
};
use aegis_gateway::core::types::{CallerContext, TenantId};
use aegis_gateway::policy::delegation::{
    UserIdentityDelegationBroker, VirtualResourceScoper,
};

#[tokio::test]
async fn test_user_delegated_identity_obo_resolution() {
    let broker = UserIdentityDelegationBroker::new();

    let now_epoch = chrono::Utc::now().timestamp() as u64;

    // 1. Store Alice's token
    let alice_token = DelegatedToken {
        user_id: "alice@company.com".to_string(),
        service: "google".to_string(),
        access_token: "ya29.alice_unique_token".to_string(),
        token_type: "Bearer".to_string(),
        expires_at_epoch_secs: now_epoch + 3600,
        scopes: vec!["https://www.googleapis.com/auth/drive.file".to_string()],
    };
    broker
        .store_user_token("alice@company.com", alice_token)
        .await
        .unwrap();

    // 2. Store Bob's token
    let bob_token = DelegatedToken {
        user_id: "bob@company.com".to_string(),
        service: "google".to_string(),
        access_token: "ya29.bob_unique_token".to_string(),
        token_type: "Bearer".to_string(),
        expires_at_epoch_secs: now_epoch + 3600,
        scopes: vec!["https://www.googleapis.com/auth/drive.readonly".to_string()],
    };
    broker
        .store_user_token("bob@company.com", bob_token)
        .await
        .unwrap();

    let alice_caller = CallerContext {
        tenant_id: TenantId::new("acme-corp"),
        subject: "alice@company.com".to_string(),
        roles: vec!["employee".to_string()],
        department: Some("marketing".to_string()),
        client_ip: None,
        session_id: "sess-alice".to_string(),
    };

    let bob_caller = CallerContext {
        tenant_id: TenantId::new("acme-corp"),
        subject: "bob@company.com".to_string(),
        roles: vec!["employee".to_string()],
        department: Some("engineering".to_string()),
        client_ip: None,
        session_id: "sess-bob".to_string(),
    };

    // Verify Alice gets her token
    let resolved_alice = broker
        .resolve_user_token(&alice_caller, "google")
        .await
        .unwrap();
    assert!(resolved_alice.is_some());
    assert_eq!(
        resolved_alice.unwrap().access_token,
        "ya29.alice_unique_token"
    );

    // Verify Bob gets his token
    let resolved_bob = broker
        .resolve_user_token(&bob_caller, "google")
        .await
        .unwrap();
    assert!(resolved_bob.is_some());
    assert_eq!(resolved_bob.unwrap().access_token, "ya29.bob_unique_token");

    // Unknown user Charlie gets None
    let charlie_caller = CallerContext {
        tenant_id: TenantId::new("acme-corp"),
        subject: "charlie@company.com".to_string(),
        roles: vec!["employee".to_string()],
        department: None,
        client_ip: None,
        session_id: "sess-charlie".to_string(),
    };
    let resolved_charlie = broker
        .resolve_user_token(&charlie_caller, "google")
        .await
        .unwrap();
    assert!(resolved_charlie.is_none());

    // Revocation removes Alice's token
    broker
        .revoke_user_token("alice@company.com", "google")
        .await
        .unwrap();
    let after_revoke = broker
        .resolve_user_token(&alice_caller, "google")
        .await
        .unwrap();
    assert!(after_revoke.is_none());
}

#[tokio::test]
async fn test_virtual_resource_scoper_folder_isolation() {
    let scoper = VirtualResourceScoper::new()
        .with_user_scope("alice@company.com", "/drive/folders/marketing-shared/")
        .with_user_scope("bob@company.com", "/github/repos/acme/backend/");

    let alice_caller = CallerContext {
        tenant_id: TenantId::new("acme-corp"),
        subject: "alice@company.com".to_string(),
        roles: vec!["employee".to_string()],
        department: None,
        client_ip: None,
        session_id: "sess-alice".to_string(),
    };

    // Alice accessing permitted marketing folder
    let allowed = scoper
        .validate_resource_scope(
            &alice_caller,
            "/drive/folders/marketing-shared/q3-budget.gsheet",
            "write",
        )
        .await
        .unwrap();
    assert!(allowed);

    // Alice attempting access to unauthorized finance folder -> blocked!
    let denied = scoper
        .validate_resource_scope(
            &alice_caller,
            "/drive/folders/finance-confidential/salaries.gsheet",
            "read",
        )
        .await
        .unwrap();
    assert!(!denied);
}
