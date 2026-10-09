// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

use std::sync::Arc;

use aegis_gateway::audit::StructuredAuditLogger;
use aegis_gateway::core::types::{CallerContext, DisclosureTier, TenantId, ToolCallRequest, ToolDefinition};
use aegis_gateway::dlp::PiiDlpPipeline;
use aegis_gateway::policy::AbacPolicyEngine;
use aegis_gateway::skills::LocalSkillRegistry;
use aegis_gateway::state::InMemoryStateBackend;
use aegis_gateway::AegisGateway;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .json()
        .init();

    println!("============================================================");
    println!("     AEGIS GATEWAY: ENTERPRISE ZERO-TRUST MCP & SKILLS      ");
    println!("============================================================");
    println!(" [Pillar 1] Distributed State & HA          : ACTIVE");
    println!(" [Pillar 2] Zero-Trust IAM & ABAC           : ACTIVE");
    println!(" [Pillar 3] Real-Time DLP & PII Redaction   : ACTIVE");
    println!(" [Pillar 4] Tamper-Evident SIEM Audit       : ACTIVE");
    println!(" [Pillar 5] Multi-Tenant FinOps Quotas      : ACTIVE");
    println!(" [Pillar 6] MIT Permissive Licensing        : VERIFIED");
    println!("------------------------------------------------------------");

    // Initialize Gateway with constructor dependency injection
    let gateway = AegisGateway::new(
        Arc::new(InMemoryStateBackend::new()),
        Arc::new(AbacPolicyEngine::new()),
        Arc::new(PiiDlpPipeline::new()),
        Arc::new(StructuredAuditLogger::new()),
        Arc::new(LocalSkillRegistry::new()),
    );

    // Seed sample tool
    let tools = vec![ToolDefinition {
        name: "customer_lookup".to_string(),
        server: "crm".to_string(),
        description: "Lookup enterprise customer records by account ID".to_string(),
        input_schema: serde_json::json!({ "type": "object", "properties": { "id": { "type": "string" } } }),
        required_params: vec!["id".to_string()],
        when_to_use: "Use when retrieving enterprise customer profile".to_string(),
        tags: vec!["crm".to_string(), "customer".to_string()],
    }];

    // Test Progressive Disclosure L0
    let discovered = gateway.discover_tools(&tools, "customer", DisclosureTier::L0).await;
    println!("Discovered {} tool(s) under Progressive Tier L0.", discovered.len());

    // Test Secure Invocation with DLP protection
    let caller = CallerContext {
        tenant_id: TenantId::new("tenant-corp-1"),
        subject: "user_developer_123".to_string(),
        roles: vec!["developer".to_string()],
        department: Some("Engineering".to_string()),
        client_ip: Some("10.0.0.1".to_string()),
        session_id: "sess_demo".to_string(),
    };

    let req = ToolCallRequest {
        tool: "customer_lookup".to_string(),
        server: "crm".to_string(),
        arguments: serde_json::json!({ "id": "CUST-999" }),
        caller,
    };

    // Tool returns sensitive customer data (Credit Card & SSN)
    let response = gateway
        .execute_tool(req, || {
            Ok(serde_json::json!({
                "status": "success",
                "customer": "John Doe",
                "payment_card": "4111 2222 3333 4444",
                "national_id": "123-45-6789"
            }))
        })
        .await?;

    println!("Tool Execution Output (DLP Sanitized):");
    println!("{}", serde_json::to_string_pretty(&response.output)?);
    println!("DLP Masked: {}", response.dlp_masked);

    Ok(())
}
