// SPDX-License-Identifier: MIT

use std::sync::Arc;
use aegis_gateway::audit::{DatadogAuditSink, HashChainSequencer, MultiplexedAuditSink, OtelAuditSink, SplunkHecSink};
use aegis_gateway::core::audit::{AuditAction, AuditChainVerifier, AuditEvent, AuditSink};
use aegis_gateway::core::types::{CallerContext, TenantId};

fn create_sample_event(id: &str, tool: &str) -> AuditEvent {
    AuditEvent {
        event_id: id.to_string(),
        timestamp: chrono::Utc::now(),
        caller: CallerContext {
            tenant_id: TenantId::new("enterprise-corp"),
            subject: "service-agent-01".to_string(),
            roles: vec!["operator".to_string()],
            department: Some("Infrastructure".to_string()),
            client_ip: None,
            session_id: "s-abc-123".to_string(),
        },
        action: AuditAction::ToolInvoked,
        target_resource: format!("k8s:{}", tool),
        payload_hash_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        metadata: serde_json::json!({ "status": "ok" }),
    }
}

#[tokio::test]
async fn test_phase4_cryptographic_hash_chain_integrity_and_soc2_report() {
    let sequencer = HashChainSequencer::new();

    // 1. Record a sequence of 4 chained events
    for i in 0..4 {
        let ev = create_sample_event(&format!("evt-{}", i), &format!("tool-{}", i));
        sequencer.record(ev).await;
    }

    assert_eq!(sequencer.len().await, 4);

    let chain = sequencer.get_chain().await;
    assert!(
        sequencer.verify_chain(&chain).expect("Chain verification"),
        "Untampered chain must pass verification"
    );

    // 2. Export and verify SOC 2 Type II audit report
    let soc2_report = sequencer.export_soc2_report().await;
    assert_eq!(soc2_report["chain_length"], 4);
    assert_eq!(soc2_report["cryptographic_integrity_verified"], true);

    // 3. Tampering Detection: Modify a previous event and assert failure
    let mut tampered_chain = chain.clone();
    tampered_chain[1].event.target_resource = "tampered:unauthorized_action".to_string();

    let tampering_result = sequencer.verify_chain(&tampered_chain);
    assert!(
        tampering_result.is_err(),
        "Tampering with past event must trigger cryptographic verification failure"
    );
}

#[tokio::test]
async fn test_phase4_otel_w3c_traceparent_propagation() {
    let otel_sink = OtelAuditSink::new("aegis-production-gateway");

    let event = create_sample_event("trace-evt-123", "deploy_service");
    otel_sink.emit(&event).await.expect("Emit span");

    let spans = otel_sink.recorded_spans().await;
    assert_eq!(spans.len(), 1);

    let traceparent = spans[0]["traceparent"].as_str().unwrap();
    assert!(traceparent.starts_with("00-"), "W3C traceparent must begin with version '00-'");
    assert!(traceparent.ends_with("-01"), "Sampled flag '-01' must be present");
    assert_eq!(spans[0]["attributes"]["tenant.id"], "enterprise-corp");
}

#[tokio::test]
async fn test_phase4_multiplexed_siem_streaming_sinks() {
    let splunk = Arc::new(SplunkHecSink::new("https://splunk.internal:8088/services/collector", "token-xyz"));
    let datadog = Arc::new(DatadogAuditSink::new("https://http-intake.logs.datadoghq.com/api/v2/logs"));

    let multiplexer = MultiplexedAuditSink::new(vec![splunk.clone(), datadog.clone()]);

    let event = create_sample_event("siem-evt-999", "payment_audit");
    multiplexer.emit(&event).await.expect("Multiplexed emit");

    assert_eq!(splunk.endpoint(), "https://splunk.internal:8088/services/collector");
    assert_eq!(splunk.token(), "token-xyz");
    assert_eq!(datadog.endpoint(), "https://http-intake.logs.datadoghq.com/api/v2/logs");
}
