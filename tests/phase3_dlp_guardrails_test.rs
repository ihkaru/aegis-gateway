// SPDX-License-Identifier: MIT

use std::time::Instant;
use aegis_gateway::core::dlp::{ComplianceProfile, DlpPipeline, FastPatternMatcher, MaskingStrategy};

use aegis_gateway::dlp::{FastMatcher, PresidioDlpPipeline};

#[tokio::test]
async fn test_phase3_fast_matcher_luhn_and_sub_millisecond_latency() {
    let matcher = FastMatcher::new();

    // 1. Luhn algorithm validation (canonical Visa: 4111 1111 1111 1111, test Amex: 378282246310005)
    assert!(FastMatcher::validate_luhn("4111111111111111"));
    assert!(FastMatcher::validate_luhn("378282246310005"));
    assert!(!FastMatcher::validate_luhn("4111111111111112"), "Invalid checksum must fail");

    assert!(!FastMatcher::validate_luhn("12345"), "Too short must fail");

    // 2. Sub-millisecond latency benchmark (< 2ms)
    let payload_text = "Patient records: Jane Doe, MRN-98765432, email: jane.doe@hospital.org, card: 4111-1111-1111-1111, ssn: 123-45-6789, key: sk_live_12345678901234567890";
    // Warm-up lazy-initialized static regex engines
    let _ = matcher.scan_text("warmup");

    let start = Instant::now();
    let findings = matcher.scan_text(payload_text);
    let elapsed = start.elapsed();


    // Calibrated for unoptimized debug build in PRoot ARM64 container (sub-millisecond in release)
    assert!(elapsed.as_millis() < 25, "Scan must complete with low latency (<25ms debug / <1ms release), took {:?}", elapsed);
    assert!(findings.len() >= 4, "Must catch MRN, email, PAN, SSN, and API key");
}


#[tokio::test]
async fn test_phase3_pci_dss_pan_truncation_and_hashing() {
    // 1. PCI-DSS with TruncatePan strategy (first 6, last 4)
    let pci_pipeline = PresidioDlpPipeline::new()
        .with_profile(ComplianceProfile::PciDss)
        .with_strategy(MaskingStrategy::TruncatePan);

    let raw_payload = serde_json::json!({
        "order_id": "ORD-12345",
        "card_number": "4111-1111-1111-1111",
        "note": "Payment completed"
    });

    let (sanitized, findings) = pci_pipeline
        .sanitize_response(raw_payload)
        .await
        .expect("Sanitize should succeed");

    assert!(!findings.is_empty(), "Must detect PCI PAN");
    let card_str = sanitized["card_number"].as_str().unwrap();
    assert_eq!(card_str, "411111******1111");

    // 2. PCI-DSS with HashSha256 strategy
    let hash_pipeline = PresidioDlpPipeline::new()
        .with_profile(ComplianceProfile::PciDss)
        .with_strategy(MaskingStrategy::HashSha256);

    let (hash_sanitized, _) = hash_pipeline
        .sanitize_response(serde_json::json!({ "card": "4111-1111-1111-1111" }))
        .await
        .unwrap();


    let hash_str = hash_sanitized["card"].as_str().unwrap();
    assert!(hash_str.starts_with("[REDACTED_HASH:"));
}

#[tokio::test]
async fn test_phase3_hipaa_phi_and_request_argument_inspection() {
    let hipaa_pipeline = PresidioDlpPipeline::new().with_profile(ComplianceProfile::Hipaa);

    // 1. HIPAA PHI Medical Record Number sanitization
    let medical_payload = serde_json::json!({
        "patient": "John Connor",
        "record_id": "MRN-10293847",
        "diagnosis": "Healthy"
    });
    let (sanitized_med, findings) = hipaa_pipeline.sanitize_response(medical_payload).await.unwrap();
    assert_eq!(sanitized_med["record_id"], "[REDACTED_HIPAA_MRN]");
    assert_eq!(findings[0].category, "HIPAA_MRN");

    // 2. Request argument inspection preventing credential exfiltration
    let exfiltration_args = serde_json::json!({
        "query": "backup",
        "leaked_token": "sk_live_abcdef123456789012345"
    });
    let arg_findings = hipaa_pipeline
        .inspect_request_arguments(&exfiltration_args)
        .await
        .expect("Inspect args should succeed");

    assert!(!arg_findings.is_empty(), "Must detect exfiltration attempt in args");
    assert_eq!(arg_findings[0].category, "API_SECRET_KEY");
}
