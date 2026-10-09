//! Phase 12 Integration Test Suite: Egress Guardrails Refusal Integrity and Outcome Attestation
//! Target Gaps: docker/mcp-gateway#591, docker/mcp-gateway#594, microsoft/mcp-gateway#102, docker/mcp-gateway#581

#![deny(unsafe_code)]

use std::sync::Arc;
use serde_json::json;
use aegis_gateway::audit::ToolOutcomeAttestation;
use aegis_gateway::core::audit::{AuditEvent, AuditSink};
use aegis_gateway::core::error::AegisResult;
use aegis_gateway::core::types::{CallerContext, TenantId, ToolCallRequest, ToolDefinition};
use aegis_gateway::discovery::{NamespacedCatalog, PromptDefinition};
use aegis_gateway::policy::egress::{DefaultEgressGuard, EgressPolicyGuard, EgressRuleConfig};
use aegis_gateway::AegisGateway;
use tokio::sync::Mutex;

/// Test Audit Sink capturing all emitted audit events for refusal verification
#[derive(Default, Clone)]
struct CapturingAuditSink {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

#[async_trait::async_trait]
impl AuditSink for CapturingAuditSink {
    async fn emit(&self, event: &AuditEvent) -> AegisResult<()> {
        self.events.lock().await.push(event.clone());
        Ok(())
    }
}

#[tokio::test]
async fn test_docker_issue_594_and_msft_109_egress_ssrf_and_domain_allowlist() {
    let config = EgressRuleConfig {
        allowed_domains: vec!["api.github.com".to_string(), "*.corp.internal".to_string()],
        allow_all_public: false,
        block_internal_networks: true,
        block_cloud_metadata: true,
    };
    let guard = DefaultEgressGuard::new(config);
    let caller = CallerContext {
        tenant_id: TenantId::new("tenant-fintech"),
        subject: "agent-007".to_string(),
        roles: vec!["developer".to_string()],
        department: Some("eng".to_string()),
        client_ip: None,
        session_id: "sess-007".to_string(),
    };

    // 1. Block Cloud Metadata (AWS, Google, etc.)
    let aws_res = guard.evaluate_url("http://169.254.169.254/latest/meta-data", &caller).await;
    assert!(aws_res.is_err(), "AWS metadata IP must be blocked");

    let gcp_res = guard.evaluate_url("http://metadata.google.internal/computeMetadata/v1", &caller).await;
    assert!(gcp_res.is_err(), "Google metadata host must be blocked");

    // 2. Block RFC-1918 Private Intranet IPs
    assert!(guard.evaluate_url("http://10.0.1.5/admin", &caller).await.is_err());
    assert!(guard.evaluate_url("http://192.168.1.1/router", &caller).await.is_err());
    assert!(guard.evaluate_url("http://172.16.0.10/secrets", &caller).await.is_err());
    assert!(guard.evaluate_url("http://127.0.0.1:8080/internal", &caller).await.is_err());
    assert!(guard.evaluate_url("http://localhost:3000/api", &caller).await.is_err());

    // 3. Block Dangerous Schemes
    assert!(guard.evaluate_url("file:///etc/passwd", &caller).await.is_err());

    // 4. Enforce Domain Allowlists
    let allowed_direct = guard.evaluate_url("https://api.github.com/repos", &caller).await;
    assert!(allowed_direct.is_ok(), "api.github.com must be allowed");

    let allowed_wildcard = guard.evaluate_url("https://payments.corp.internal/charge", &caller).await;
    assert!(allowed_wildcard.is_ok(), "*.corp.internal wildcard must be allowed");

    let blocked_unauthorized = guard.evaluate_url("https://evil-unauthorized.com/payload", &caller).await;
    assert!(blocked_unauthorized.is_err(), "Unauthorized public domain must be blocked");

    // 5. Recursive JSON arguments inspection
    let nested_args_bad = json!({
        "webhook": {
            "endpoint_url": "http://169.254.169.254/creds"
        }
    });
    assert!(guard.inspect_arguments(&nested_args_bad, &caller).await.is_err());

    let nested_args_good = json!({
        "destination": {
            "target_url": "https://api.github.com/webhooks"
        }
    });
    assert!(guard.inspect_arguments(&nested_args_good, &caller).await.is_ok());
}

#[tokio::test]
async fn test_docker_issue_591_refusal_audit_integrity() {
    let sink = CapturingAuditSink::default();
    let gateway = AegisGateway::new(
        Arc::new(aegis_gateway::state::InMemoryStateBackend::new()),
        Arc::new(aegis_gateway::policy::AbacPolicyEngine::new()),
        Arc::new(aegis_gateway::dlp::PiiDlpPipeline::new()),
        Arc::new(sink.clone()),
        Arc::new(aegis_gateway::skills::LocalSkillRegistry::new()),
    );

    let caller = CallerContext {
        tenant_id: TenantId::new("tenant-dev"),
        subject: "intern-01".to_string(),
        roles: vec!["guest".to_string()],
        department: Some("eng".to_string()),
        client_ip: None,
        session_id: "sess-01".to_string(),
    };

    // Attempt unauthorized destructive call (requires admin)
    let req = ToolCallRequest {
        caller: caller.clone(),
        server: "db_server".to_string(),
        tool: "delete_database".to_string(),
        arguments: json!({ "target": "production_users" }),
    };

    let result = gateway.execute_tool(req, || Ok(json!({ "status": "ok" }))).await;
    assert!(result.is_err(), "Unauthorized call must be refused");

    // Verify Audit Refusal Integrity (Issue #591)
    let captured = sink.events.lock().await;
    assert_eq!(captured.len(), 1, "Exactly one refusal audit event must be emitted");

    let event = &captured[0];
    assert_eq!(event.target_resource, "db_server:delete_database");
    assert_eq!(event.caller.subject, "intern-01");

    match &event.action {
        aegis_gateway::core::audit::AuditAction::PolicyEvaluated { allowed, reason } => {
            assert!(!allowed, "Audit log must explicitly record allowed=false (Refusal)");
            assert!(
                reason.as_ref().unwrap().contains("requires 'admin' role"),
                "Audit log must preserve exact refusal reason"
            );
        }
        _ => panic!("Expected PolicyEvaluated refusal audit action"),
    }
}

#[tokio::test]
async fn test_microsoft_issue_102_and_docker_557_tool_outcome_attestation() {
    let secret = "aegis-super-secret-hmac-key";
    let gateway_id = "aegis-gateway-prod-pod-4";
    let tool = "fetch_financial_record";
    let arguments = json!({ "account_id": "ACC-99812", "period": "2026-Q3" });
    let result = json!({ "revenue": 1500000.0, "status": "audited" });
    let duration_ms = 42;

    // 1. Generate valid attestation
    let attestation = ToolOutcomeAttestation::generate(
        tool,
        &arguments,
        &result,
        duration_ms,
        secret,
        gateway_id,
    );

    assert_eq!(attestation.tool, tool);
    assert_eq!(attestation.gateway_identity, gateway_id);
    assert_eq!(attestation.duration_ms, 42);
    assert!(!attestation.attestation_token.is_empty());

    // 2. Cryptographic Verification
    assert!(attestation.verify(secret), "Valid attestation must verify cleanly");

    // 3. Reject Wrong Secret Key
    assert!(!attestation.verify("wrong-secret-key"), "Wrong key must fail verification");

    // 4. Reject Tampered Outcome / Arguments
    let mut tampered_args = attestation.clone();
    tampered_args.arguments_hash_sha256 = "tampered_hash_000".to_string();
    assert!(!tampered_args.verify(secret), "Tampered argument hash must fail");

    let mut tampered_res = attestation.clone();
    tampered_res.result_hash_sha256 = "tampered_res_999".to_string();
    assert!(!tampered_res.verify(secret), "Tampered result hash must fail");
}

#[tokio::test]
async fn test_docker_issue_581_namespaced_prompt_and_tool_collision_prevention() {
    let mut catalog = NamespacedCatalog::new();

    let prompt_a = PromptDefinition {
        name: "code_review".to_string(),
        description: "Python code reviewer".to_string(),
        server: "python_server".to_string(),
        arguments: vec!["file".to_string()],
    };
    let prompt_b = PromptDefinition {
        name: "code_review".to_string(),
        description: "Rust code reviewer".to_string(),
        server: "rust_server".to_string(),
        arguments: vec!["file".to_string()],
    };

    // Register first prompt
    let id_a = catalog.register_prompt(prompt_a, "python_server");
    assert_eq!(id_a, "python_server__code_review");
    assert!(!catalog.has_prompt_collision("code_review"));
    // Unqualified alias works while unambiguous
    assert!(catalog.resolve_prompt("code_review").is_some());

    // Register second prompt with identical name from another server (Issue #581)
    let id_b = catalog.register_prompt(prompt_b, "rust_server");
    assert_eq!(id_b, "rust_server__code_review");

    // Collision must now be detected!
    assert!(catalog.has_prompt_collision("code_review"));

    // Unqualified ambiguous alias is removed to prevent accidental routing
    assert!(
        catalog.resolve_prompt("code_review").is_none(),
        "Ambiguous prompt must not resolve unqualified"
    );

    // Both prompts safely coexist via namespaced IDs
    let resolved_a = catalog.resolve_prompt("python_server__code_review").expect("Server A prompt exists");
    assert_eq!(resolved_a.description, "Python code reviewer");

    let resolved_b = catalog.resolve_prompt("rust_server__code_review").expect("Server B prompt exists");
    assert_eq!(resolved_b.description, "Rust code reviewer");

    // Same guarantee for tools
    let tool_a = ToolDefinition {
        name: "query".to_string(),
        description: "SQL query".to_string(),
        server: "postgres".to_string(),
        input_schema: json!({}),
        required_params: vec![],
        when_to_use: "query DB".to_string(),
        tags: vec!["db".to_string()],
    };
    let tool_b = ToolDefinition {
        name: "query".to_string(),
        description: "Elastic search query".to_string(),
        server: "elastic".to_string(),
        input_schema: json!({}),
        required_params: vec![],
        when_to_use: "query index".to_string(),
        tags: vec!["search".to_string()],
    };

    catalog.register_tool(tool_a, "postgres");
    catalog.register_tool(tool_b, "elastic");

    assert!(catalog.has_tool_collision("query"));
    assert!(catalog.resolve_tool("query").is_none());
    assert!(catalog.resolve_tool("postgres__query").is_some());
    assert!(catalog.resolve_tool("elastic__query").is_some());
}
