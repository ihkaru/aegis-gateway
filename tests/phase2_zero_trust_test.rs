// SPDX-License-Identifier: MIT

use aegis_gateway::core::identity::TokenValidator;
use aegis_gateway::core::policy::{PolicyContext, PolicyDecision, PolicyEngine};
use aegis_gateway::core::secrets::SecretStore;
use aegis_gateway::core::session::SessionRevocationRegistry;
use aegis_gateway::core::types::{CallerContext, TenantId};
use aegis_gateway::policy::{
    EnvSecretStore, MemoryRevocationRegistry, OidcClaims, OidcTokenValidator, OpaPolicyEngine,
    VaultSecretStore,
};

#[tokio::test]
async fn test_phase2_oidc_jwt_token_validation() {
    let secret = "enterprise-super-secret-key-32bytes!";
    let validator = OidcTokenValidator::new()
        .with_issuer("https://auth.enterprise.acme.com")
        .with_hmac_secret(secret);

    // 1. Valid Token
    let valid_claims = OidcClaims {
        sub: "user_sarah_connor".to_string(),
        iss: Some("https://auth.enterprise.acme.com".to_string()),
        exp: Some(chrono::Utc::now().timestamp() + 3600),
        tenant_id: Some("tenant-cyberdyne".to_string()),
        department: Some("Security".to_string()),
        roles: Some(vec!["auditor".to_string(), "developer".to_string()]),
    };
    let token = OidcTokenValidator::generate_test_token(&valid_claims, secret);
    let caller = validator.validate_token(&token).await.expect("Token must validate");
    assert_eq!(caller.subject, "user_sarah_connor");
    assert_eq!(caller.department.as_deref(), Some("Security"));
    assert_eq!(caller.tenant_id.as_str(), "tenant-cyberdyne");

    // 2. Expired Token
    let expired_claims = OidcClaims {
        sub: "user_sarah_connor".to_string(),
        iss: Some("https://auth.enterprise.acme.com".to_string()),
        exp: Some(chrono::Utc::now().timestamp() - 60), // Expired 1 minute ago
        tenant_id: Some("tenant-cyberdyne".to_string()),
        department: Some("Security".to_string()),
        roles: Some(vec!["auditor".to_string()]),
    };
    let expired_token = OidcTokenValidator::generate_test_token(&expired_claims, secret);
    let res = validator.validate_token(&expired_token).await;
    assert!(res.is_err(), "Expired token must be rejected");

    // 3. Forged Signature Token
    let forged_token = OidcTokenValidator::generate_test_token(&valid_claims, "wrong-forged-secret");
    let res_forged = validator.validate_token(&forged_token).await;
    assert!(res_forged.is_err(), "Forged signature must fail validation");
}

#[tokio::test]
async fn test_phase2_opa_payload_bounds_and_ddl_prevention() {
    let opa = OpaPolicyEngine::with_enterprise_defaults();

    let finance_caller = CallerContext {
        tenant_id: TenantId::new("fin-1"),
        subject: "cfo_user".to_string(),
        roles: vec!["finance_admin".to_string()],
        department: Some("Finance".to_string()),
        client_ip: None,
        session_id: "s1".to_string(),
    };

    let dev_caller = CallerContext {
        tenant_id: TenantId::new("dev-1"),
        subject: "dev_user".to_string(),
        roles: vec!["developer".to_string()],
        department: Some("Engineering".to_string()),
        client_ip: None,
        session_id: "s2".to_string(),
    };

    // 1. Finance Admin allowed for <= $5000
    let ctx_finance = PolicyContext {
        caller: finance_caller.clone(),
        server: "billing".to_string(),
        tool: "wire_transfer".to_string(),
        arguments: serde_json::json!({ "amount": 2500.0, "recipient": "ACME" }),
        requested_at: chrono::Utc::now(),
    };
    let decision = opa.evaluate(&ctx_finance).await.unwrap();
    assert_eq!(decision, PolicyDecision::Allow);
    let payload_check = opa.eval_payload(&ctx_finance.tool, &ctx_finance.arguments).unwrap();
    assert_eq!(payload_check, PolicyDecision::Allow);

    // 2. Finance Admin blocked if amount > $5000 (Issue #555 protection)
    let payload_exceeded = serde_json::json!({ "amount": 7500.0, "recipient": "ACME" });
    let decision_exceeded = opa.eval_payload("wire_transfer", &payload_exceeded).unwrap();
    match decision_exceeded {
        PolicyDecision::Deny { reason } => assert!(reason.contains("exceeds policy limit")),
        PolicyDecision::Allow => panic!("Should have denied $7500 transfer"),
    }

    // 3. Developer blocked from wire_transfer tool
    let ctx_dev_wire = PolicyContext {
        caller: dev_caller.clone(),
        server: "billing".to_string(),
        tool: "wire_transfer".to_string(),
        arguments: serde_json::json!({ "amount": 100.0 }),
        requested_at: chrono::Utc::now(),
    };
    let decision_dev = opa.evaluate(&ctx_dev_wire).await.unwrap();
    assert!(matches!(decision_dev, PolicyDecision::Deny { .. }));

    // 4. SQL Injection / Destructive DDL statement prevention
    let safe_sql = serde_json::json!({ "query": "SELECT * FROM users WHERE id = 1" });
    assert_eq!(opa.eval_payload("query_database", &safe_sql).unwrap(), PolicyDecision::Allow);

    let drop_sql = serde_json::json!({ "query": "DROP TABLE audit_records;" });
    let drop_decision = opa.eval_payload("query_database", &drop_sql).unwrap();
    match drop_decision {
        PolicyDecision::Deny { reason } => assert!(reason.contains("forbidden keyword 'DROP'")),
        PolicyDecision::Allow => panic!("Must deny DROP TABLE statement"),
    }
}

#[tokio::test]
async fn test_phase2_secret_stores_and_scim_revocation() {
    // 1. Vault Secret Store
    let vault = VaultSecretStore::new("https://vault.internal.net:8200")
        .with_seed_secret("database/prod_password", "sup3r_s3cr3t_pass");
    assert!(vault.health_check().await.unwrap());
    let secret = vault.get_secret("database/prod_password").await.unwrap();
    assert_eq!(secret.as_deref(), Some("sup3r_s3cr3t_pass"));

    // 2. Env Secret Store
    let env_store = EnvSecretStore::new();
    env_store.set_secret("STRIPE_API_KEY", "sk_test_12345").await.unwrap();
    let retrieved = env_store.get_secret("STRIPE_API_KEY").await.unwrap();
    assert_eq!(retrieved.as_deref(), Some("sk_test_12345"));

    // 3. SCIM Instant Session Deactivation
    let scim = MemoryRevocationRegistry::new();
    assert!(!scim.is_revoked("sess-100", "employee_42").await.unwrap());

    // Revoke specific session
    scim.revoke_session("sess-100", "Token leaked").await.unwrap();
    assert!(scim.is_revoked("sess-100", "employee_42").await.unwrap());
    assert!(!scim.is_revoked("sess-101", "employee_42").await.unwrap());

    // SCIM terminate employee across all sessions
    scim.deactivate_subject("employee_42", "Terminated").await.unwrap();
    assert!(scim.is_revoked("sess-101", "employee_42").await.unwrap());
}
