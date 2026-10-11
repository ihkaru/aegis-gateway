// SPDX-License-Identifier: MIT

use std::sync::Arc;
use serde_json::json;

use propylon::control::abac_engine::LocalAbacEngine;
use propylon::control::auth_api::AuthApiDispatcher;
use propylon::control::catalog_scoper::AbacCatalogScoper;
use propylon::control::downstream_signer::DownstreamIdentitySigner;
use propylon::control::identity_broker::LocalIdentityBroker;
use propylon::core::abac::{
    AbacContext, AbacEffect, AbacPolicyEngine, ActionAttributes, EnvironmentAttributes,
    PrincipalAttributes, ResourceAttributes,
};
use propylon::core::identity::{
    AssertionExchangeRequest, DeviceAuthRequest, DeviceCodeStatus, IdentityFederationBroker,
};
use propylon::core::scoping::CatalogScopingProvider;
use propylon::core::types::{CallerContext, TenantId, ToolDefinition};

#[tokio::test]
async fn test_phase49_oauth_device_flow_lifecycle_and_assertion_exchange() {
    let broker = LocalIdentityBroker::new("https://propylon.corp.internal");

    // 1. Initiate Device Flow (RFC 8628)
    let init_req = DeviceAuthRequest {
        client_id: "cursor-ai-agent".to_string(),
        scopes: vec!["mcp:tools:read".to_string(), "mcp:tools:execute".to_string()],
    };
    let init_res = broker.start_device_flow(&init_req).await.expect("Failed to start device flow");
    assert!(init_res.device_code.starts_with("dev_"));
    assert_eq!(init_res.user_code.len(), 9); // XXXX-XXXX
    assert_eq!(init_res.verification_uri, "https://propylon.corp.internal/activate");

    // 2. Poll while pending
    let status_pending = broker.poll_device_flow(&init_res.device_code).await.expect("Poll failed");
    assert_eq!(status_pending, DeviceCodeStatus::Pending);

    // 3. User approves in browser SSO
    let claims = CallerContext {
        tenant_id: TenantId::new("fintech-prod"),
        subject: "alice@corp.internal".to_string(),
        roles: vec!["data-scientist".to_string()],
        department: Some("DataScience".to_string()),
        client_ip: Some("10.0.4.15".to_string()),
        session_id: "sess_test_123".to_string(),
    };
    broker.approve_device_flow(&init_res.user_code, claims.clone()).await.expect("Approve failed");

    // 4. Poll after approval -> Approved
    let status_approved = broker.poll_device_flow(&init_res.device_code).await.expect("Poll failed");
    match status_approved {
        DeviceCodeStatus::Approved { claims: c } => {
            assert_eq!(c.subject, "alice@corp.internal");
            assert_eq!(c.tenant_id.0, "fintech-prod");
        }
        _ => panic!("Expected Approved state"),
    }

    // 5. RFC 8693 Token Exchange / SAML 2.0 Ingestion
    let exchange_req = AssertionExchangeRequest {
        assertion_type: "urn:ietf:params:oauth:grant-type:saml2-bearer".to_string(),
        assertion: "<saml:Assertion ID='_test'>EnterpriseSAML2Payload</saml:Assertion>".to_string(),
        resource: Some("urn:mcp:server:analytics".to_string()),
    };
    let token_res = broker.exchange_assertion(&exchange_req).await.expect("Exchange failed");
    assert!(token_res.access_token.starts_with("propylon_jwt_"));
    assert_eq!(token_res.token_type, "Bearer");
    assert_eq!(token_res.scope, "urn:mcp:server:analytics");
}

#[tokio::test]
async fn test_phase50_abac_policy_engine_evaluation_and_hot_reload() {
    let engine = LocalAbacEngine::with_default_rules();

    // Context 1: Engineering L2 reading analytics -> Allow
    let ctx_allow = AbacContext {
        principal: PrincipalAttributes {
            user_id: "bob".to_string(),
            department: "Engineering".to_string(),
            clearance_level: "L2".to_string(),
            roles: vec!["dev".to_string()],
            groups: vec![],
            extra: Default::default(),
        },
        action: ActionAttributes {
            tool_name: "remote_analytics_query_sales".to_string(),
            operation_type: "read".to_string(),
        },
        resource: ResourceAttributes {
            backend_name: "analytics".to_string(),
            data_classification: "internal".to_string(),
        },
        environment: EnvironmentAttributes::default(),
    };
    let decision = engine.evaluate(&ctx_allow).await.expect("Evaluation failed");
    assert_eq!(decision.effect, AbacEffect::Allow);

    // Context 2: Sensitive mutation -> RequireHitl
    let ctx_hitl = AbacContext {
        principal: PrincipalAttributes {
            user_id: "bob".to_string(),
            department: "Engineering".to_string(),
            clearance_level: "L2".to_string(),
            roles: vec!["dev".to_string()],
            groups: vec![],
            extra: Default::default(),
        },
        action: ActionAttributes {
            tool_name: "execute_sql_mutation".to_string(),
            operation_type: "write".to_string(),
        },
        resource: ResourceAttributes {
            backend_name: "db".to_string(),
            data_classification: "internal".to_string(),
        },
        environment: EnvironmentAttributes::default(),
    };
    let decision_hitl = engine.evaluate(&ctx_hitl).await.expect("Evaluation failed");
    assert_eq!(decision_hitl.effect, AbacEffect::RequireHitl);

    // Context 3: Destructive mutation -> Deny
    let ctx_deny = AbacContext {
        principal: ctx_allow.principal.clone(),
        action: ActionAttributes {
            tool_name: "drop_table".to_string(),
            operation_type: "delete".to_string(),
        },
        resource: ResourceAttributes {
            backend_name: "db".to_string(),
            data_classification: "internal".to_string(),
        },
        environment: EnvironmentAttributes::default(),
    };
    let decision_deny = engine.evaluate(&ctx_deny).await.expect("Evaluation failed");
    assert_eq!(decision_deny.effect, AbacEffect::Deny);

    // Dry-run simulation
    let sim = engine.simulate(&ctx_allow).await.expect("Simulation failed");
    assert_eq!(sim.decision.effect, AbacEffect::Allow);
    assert!(!sim.matched_rules.is_empty());
}

#[tokio::test]
async fn test_phase50_downstream_identity_signing() {
    let secret = "enterprise-vault-hmac-key";
    let (user_id, tenant_id, timestamp, sig) =
        DownstreamIdentitySigner::sign_headers("emp_9921", "tenant_apac", secret);

    assert_eq!(user_id, "emp_9921");
    assert_eq!(tenant_id, "tenant_apac");
    assert_eq!(sig.len(), 16);

    // Valid signature verification
    let valid = DownstreamIdentitySigner::verify_signature(
        &user_id, &tenant_id, &timestamp, &sig, secret,
    );
    assert!(valid);

    // Tampered verification fails
    let tampered = DownstreamIdentitySigner::verify_signature(
        "attacker", &tenant_id, &timestamp, &sig, secret,
    );
    assert!(!tampered);
}

#[tokio::test]
async fn test_phase51_catalog_scoping_least_privilege() {
    let engine = Arc::new(LocalAbacEngine::with_default_rules());
    let scoper = AbacCatalogScoper::new(engine);

    let tools = vec![
        ToolDefinition {
            name: "remote_analytics_report".to_string(),
            server: "analytics".to_string(),
            description: "Analytics".to_string(),
            input_schema: json!({}),
            required_params: vec![],
            when_to_use: "Querying analytics".to_string(),
            tags: vec!["analytics".to_string()],
        },
        ToolDefinition {
            name: "drop_table".to_string(),
            server: "db".to_string(),
            description: "Dangerous".to_string(),
            input_schema: json!({}),
            required_params: vec![],
            when_to_use: "Dropping table".to_string(),
            tags: vec!["db".to_string()],
        },
    ];

    let principal = PrincipalAttributes {
        user_id: "engineer_1".to_string(),
        department: "Engineering".to_string(),
        clearance_level: "L2".to_string(),
        roles: vec!["dev".to_string()],
        groups: vec![],
        extra: Default::default(),
    };

    let scoped = scoper.scope_catalog(&principal, &tools).await.expect("Scoping failed");
    assert_eq!(scoped.len(), 1);
    assert_eq!(scoped[0].name, "remote_analytics_report");
}

#[tokio::test]
async fn test_auth_api_dispatcher_endpoints() {
    // 1. Device Code
    let body = json!({ "client_id": "claude-code-cli", "scopes": ["mcp:all"] });
    let (code, res, _) = AuthApiDispatcher::dispatch(
        "POST",
        "/api/v1/auth/device/code",
        &serde_json::to_vec(&body).unwrap(),
    ).await;
    assert_eq!(code, 200);
    let val: serde_json::Value = serde_json::from_slice(&res).unwrap();
    let device_code = val["device_code"].as_str().unwrap();
    let user_code = val["user_code"].as_str().unwrap();

    // 2. Poll Pending
    let poll_body = json!({ "device_code": device_code });
    let (code, _, _) = AuthApiDispatcher::dispatch(
        "POST",
        "/api/v1/auth/device/token",
        &serde_json::to_vec(&poll_body).unwrap(),
    ).await;
    assert_eq!(code, 400); // authorization_pending

    // 3. Approve
    let approve_body = json!({ "user_code": user_code, "subject": "carol@corp.internal" });
    let (code, _, _) = AuthApiDispatcher::dispatch(
        "POST",
        "/api/v1/auth/device/approve",
        &serde_json::to_vec(&approve_body).unwrap(),
    ).await;
    assert_eq!(code, 200);

    // 4. Poll Approved -> Token issued
    let (code, res, _) = AuthApiDispatcher::dispatch(
        "POST",
        "/api/v1/auth/device/token",
        &serde_json::to_vec(&poll_body).unwrap(),
    ).await;
    assert_eq!(code, 200);
    let token_val: serde_json::Value = serde_json::from_slice(&res).unwrap();
    assert!(token_val["access_token"].as_str().unwrap().contains("carol@corp.internal"));

    // 5. Policy Simulation Endpoint
    let ctx = AbacContext {
        principal: PrincipalAttributes {
            user_id: "carol".to_string(),
            department: "Engineering".to_string(),
            clearance_level: "L2".to_string(),
            roles: vec![],
            groups: vec![],
            extra: Default::default(),
        },
        action: ActionAttributes {
            tool_name: "remote_analytics_query".to_string(),
            operation_type: "read".to_string(),
        },
        resource: ResourceAttributes {
            backend_name: "analytics".to_string(),
            data_classification: "internal".to_string(),
        },
        environment: EnvironmentAttributes::default(),
    };
    let (code, sim_res, _) = AuthApiDispatcher::dispatch(
        "POST",
        "/api/v1/policy/simulate",
        &serde_json::to_vec(&ctx).unwrap(),
    ).await;
    assert_eq!(code, 200);
    let sim_val: serde_json::Value = serde_json::from_slice(&sim_res).unwrap();
    assert_eq!(sim_val["decision"]["effect"], "Allow");
}
