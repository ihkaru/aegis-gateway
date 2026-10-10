use aegis_gateway::core::ingress_tls::{
    AcmeCertificateManager, MtlsClientVerifier, MtlsValidationResult, TlsCertificateStatus,
    TlsIngressEngine,
};
use aegis_gateway::transport::tls_listener::NativeTlsIngressEngine;

#[test]
fn test_native_tls_inspection_and_alpn_negotiation() {
    let engine = NativeTlsIngressEngine::new(
        "gateway.aegis.enterprise.io",
        "DigiCert Enterprise Root CA",
        "1002348571",
        1893456000, // 2030 epoch
        vec!["h2".to_string(), "http/1.1".to_string()],
    );

    let cert = engine.inspect_certificate().expect("inspect cert failed");
    assert_eq!(cert.domain, "gateway.aegis.enterprise.io");
    assert_eq!(cert.issuer, "DigiCert Enterprise Root CA");
    assert_eq!(cert.status, TlsCertificateStatus::Active);
    assert!(engine.is_alpn_supported("h2"));
    assert!(engine.is_alpn_supported("http/1.1"));
    assert!(!engine.is_alpn_supported("spdy"));

    // Test atomic certificate swap
    let new_cert_pem = "CN=api.aegis.enterprise.io\nO=Aegis Internal PKI\nSERIAL=88880002\n";
    let swap_res = engine.swap_certificate(new_cert_pem, "DUMMY_KEY");
    assert!(swap_res.is_ok());

    let updated_cert = engine.inspect_certificate().unwrap();
    assert_eq!(updated_cert.domain, "api.aegis.enterprise.io");
    assert_eq!(updated_cert.issuer, "Aegis Internal PKI");
    assert_eq!(updated_cert.serial_number, "88880002");
}

#[test]
fn test_strict_mtls_client_verification_and_revocation() {
    let mut engine = NativeTlsIngressEngine::new(
        "gateway.aegis.enterprise.io",
        "Enterprise Root CA",
        "1001",
        1893456000,
        vec!["h2".to_string()],
    );

    // Register trusted enterprise CA
    engine
        .add_trusted_ca("CA=Aegis-Agent-CA-G2\n")
        .expect("add trusted CA failed");

    // Case 1: Missing client cert
    let res_missing = engine.verify_client_certificate("").unwrap();
    assert_eq!(res_missing, MtlsValidationResult::MissingClientCertificate);

    // Case 2: Untrusted CA
    let untrusted_pem = "ISSUER=Rogue-Untrusted-CA\nCN=agent-007\nSERIAL=555\n";
    let res_untrusted = engine.verify_client_certificate(untrusted_pem).unwrap();
    assert_eq!(res_untrusted, MtlsValidationResult::UntrustedCertificateAuthority);

    // Case 3: Valid client cert signed by trusted CA
    let valid_pem = "ISSUER=Aegis-Agent-CA-G2\nCN=claude-worker-node\nSAN=claude.agent.internal\nO=FinTech Corp\nSERIAL=777\nEXPIRY=1893456000\n";
    let res_valid = engine.verify_client_certificate(valid_pem).unwrap();
    match res_valid {
        MtlsValidationResult::Verified {
            client_id,
            common_name,
            san,
            organization,
        } => {
            assert_eq!(client_id, "mtls:claude-worker-node:777");
            assert_eq!(common_name, "claude-worker-node");
            assert_eq!(san, "claude.agent.internal");
            assert_eq!(organization, "FinTech Corp");
        }
        other => panic!("Expected Verified, got {:?}", other),
    }

    // Case 4: Revoked client serial
    engine.revoke_serial("777");
    let res_revoked = engine.verify_client_certificate(valid_pem).unwrap();
    assert_eq!(res_revoked, MtlsValidationResult::RevokedCertificate);
}

#[test]
fn test_acme_http01_challenge_lifecycle_and_renewal() {
    let engine = NativeTlsIngressEngine::new(
        "gateway.aegis.enterprise.io",
        "Let's Encrypt Staging",
        "5001",
        1893456000,
        vec!["h2".to_string(), "http/1.1".to_string()],
    );

    let token = "TOKEN_XYZ_987654";
    let key_auth = "TOKEN_XYZ_987654.thumbprint_abc123";

    // Non-existent challenge returns None
    assert_eq!(engine.handle_http01_challenge(token), None);

    // Register active challenge
    engine.register_challenge(token.to_string(), key_auth.to_string());
    assert_eq!(
        engine.handle_http01_challenge(token),
        Some(key_auth.to_string())
    );

    // Automated renewal applies renewed cert
    let renewed_pem = "CN=renewed.aegis.enterprise.io\nO=Let's Encrypt Authority X3\nSERIAL=99990003\n";
    let renew_res = engine.apply_renewed_certificate(renewed_pem, "NEW_KEY");
    assert!(renew_res.is_ok());

    let active_cert = engine.inspect_certificate().unwrap();
    assert_eq!(active_cert.domain, "renewed.aegis.enterprise.io");
    assert_eq!(active_cert.issuer, "Let's Encrypt Authority X3");
}
