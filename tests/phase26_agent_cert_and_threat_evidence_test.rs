// SPDX-License-Identifier: MIT

use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use aegis_gateway::core::trust::{
    AgentCertEnvelope, AgentTrustLevel, AgentTrustVerifier, CorsPolicyEnforcer,
    ThreatEvidenceEnricher,
};
use aegis_gateway::trust::{
    ConfigurableCorsEnforcer, Ed25519AgentTrustVerifier, HeuristicThreatEvidenceEnricher,
};

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(acc, "{:02x}", b);
        acc
    })
}

fn sign_envelope(agent_id: &str, nonce: &str, timestamp: u64, pub_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(agent_id.as_bytes());
    hasher.update(nonce.as_bytes());
    hasher.update(&timestamp.to_be_bytes());
    hasher.update(pub_key.as_bytes());
    to_hex(&hasher.finalize())
}

#[tokio::test]
async fn test_agent_cert_signature_and_trust_level_assignment() {
    let trusted_key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI_ENTERPRISE_INTERNAL_KEY".to_string();
    let verifier = Ed25519AgentTrustVerifier::new(vec![trusted_key.clone()], 60);

    let ts = now_unix();
    let sig_valid = sign_envelope("agent-lead-orchestrator", "nonce-101", ts, &trusted_key);

    let cert_internal = AgentCertEnvelope {
        agent_id: "agent-lead-orchestrator".into(),
        public_key_pem: trusted_key.clone(),
        nonce: "nonce-101".into(),
        timestamp_unix: ts,
        signature_hex: sig_valid,
        declared_capabilities: vec!["read".into(), "write".into()],
    };

    // 1. Internal certified check
    let res = verifier.verify_agent_identity(&cert_internal).await.expect("verify failed");
    assert!(res.verified);
    assert_eq!(res.trust_level, AgentTrustLevel::InternalCertified);

    // 2. Tampered signature check
    let mut cert_tampered = cert_internal.clone();
    cert_tampered.signature_hex = "0000deadbeef".into();
    let res_tampered = verifier.verify_agent_identity(&cert_tampered).await.expect("verify failed");
    assert!(!res_tampered.verified);
    assert_eq!(res_tampered.trust_level, AgentTrustLevel::Untrusted);

    // 3. Expired nonce check (200 seconds ago)
    let mut cert_expired = cert_internal.clone();
    cert_expired.timestamp_unix = ts - 200;
    cert_expired.signature_hex = sign_envelope(
        &cert_expired.agent_id,
        &cert_expired.nonce,
        cert_expired.timestamp_unix,
        &cert_expired.public_key_pem,
    );
    let res_expired = verifier.verify_agent_identity(&cert_expired).await.expect("verify failed");
    assert!(!res_expired.verified);
    assert!(res_expired.failure_reason.unwrap().contains("expired"));
}

#[tokio::test]
async fn test_threat_evidence_enricher_intercepts_ssrf_and_malicious_domains() {
    let enricher = HeuristicThreatEvidenceEnricher::new(vec!["malicious-c2.darknet".into()]);

    // 1. Cloud metadata SSRF probe
    let ssrf_url = "http://169.254.169.254/computeMetadata/v1/";
    let report_ssrf = enricher.inspect_and_enrich_url(ssrf_url).await.expect("enrich failed");
    assert!(report_ssrf.malicious);
    assert!(report_ssrf.threat_categories.iter().any(|c| c.contains("METADATA") || c.contains("SSRF")));

    // 2. Known malicious domain
    let malware_url = "https://malicious-c2.darknet/payload.bin";
    let report_mal = enricher.inspect_and_enrich_url(malware_url).await.expect("enrich failed");
    assert!(report_mal.malicious);
    assert!(report_mal.threat_categories.iter().any(|c| c.contains("KNOWN_MALICIOUS")));

    // 3. Clean external target
    let clean_url = "https://api.github.com/repos/ihkaru/aegis-gateway";
    let report_clean = enricher.inspect_and_enrich_url(clean_url).await.expect("enrich failed");
    assert!(!report_clean.malicious);
    assert!(report_clean.risk_score < 0.2);
}

#[test]
fn test_cors_enforcer_preflight_and_wildcard() {
    let enforcer = ConfigurableCorsEnforcer::new(
        vec!["https://ide.antigravity.dev".into(), "https://chat.enterprise.com".into()],
        false,
    );

    // 1. Permitted Origin
    let allowed = enforcer.evaluate_cors_origin(Some("https://ide.antigravity.dev"), None);
    assert!(allowed.allowed);
    assert_eq!(allowed.allowed_origin.unwrap(), "https://ide.antigravity.dev");

    // 2. Rejected Origin
    let rejected = enforcer.evaluate_cors_origin(Some("https://evil-untrusted-site.com"), None);
    assert!(!rejected.allowed);
    assert!(rejected.allowed_origin.is_none());

    // 3. Wildcard Mode
    let wildcard_enforcer = ConfigurableCorsEnforcer::new(vec![], true);
    let wild_res = wildcard_enforcer.evaluate_cors_origin(Some("https://anywhere.org"), None);
    assert!(wild_res.allowed);
    assert_eq!(wild_res.allowed_origin.unwrap(), "*");
}
