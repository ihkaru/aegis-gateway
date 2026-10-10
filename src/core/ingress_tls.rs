use crate::core::error::AegisError;
use serde::{Deserialize, Serialize};

/// Status of a TLS certificate in the ingress pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TlsCertificateStatus {
    Active,
    ExpiringSoon,
    Expired,
    Revoked,
}

/// Metadata and cryptographic identity of a TLS certificate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsCertificateDetails {
    pub domain: String,
    pub issuer: String,
    pub serial_number: String,
    pub valid_until_epoch: u64,
    pub sha256_fingerprint: String,
    pub status: TlsCertificateStatus,
    pub alpn_protocols: Vec<String>,
}

/// Outcome of a Mutual TLS (mTLS) client certificate verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MtlsValidationResult {
    Verified {
        client_id: String,
        common_name: String,
        san: String,
        organization: String,
    },
    MissingClientCertificate,
    UntrustedCertificateAuthority,
    ExpiredCertificate,
    RevokedCertificate,
}

/// Interface contract for TLS ingress termination and ALPN negotiation.
pub trait TlsIngressEngine: Send + Sync {
    /// Inspect the currently active server certificate.
    fn inspect_certificate(&self) -> Result<TlsCertificateDetails, AegisError>;

    /// Check if a specific ALPN protocol (e.g. "h2", "http/1.1") is supported.
    fn is_alpn_supported(&self, protocol: &str) -> bool;

    /// Swap certificate material at runtime without interrupting active connections.
    fn swap_certificate(&self, cert_pem: &str, key_pem: &str) -> Result<(), AegisError>;
}

/// Interface contract for strict Zero-Trust Mutual TLS (mTLS) client verification.
pub trait MtlsClientVerifier: Send + Sync {
    /// Verify a client-presented X.509 certificate.
    fn verify_client_certificate(&self, cert_pem: &str) -> Result<MtlsValidationResult, AegisError>;

    /// Register an enterprise root or intermediate CA certificate as trusted.
    fn add_trusted_ca(&mut self, ca_pem: &str) -> Result<(), AegisError>;
}

/// Interface contract for automated ACME (RFC 8555) HTTP-01 challenge management.
pub trait AcmeCertificateManager: Send + Sync {
    /// Look up an ACME HTTP-01 key authorization for an incoming token.
    fn handle_http01_challenge(&self, token: &str) -> Option<String>;

    /// Register an active ACME challenge token and its key authorization.
    fn register_challenge(&self, token: String, key_authorization: String);

    /// Apply an automated certificate renewal received from the ACME CA.
    fn apply_renewed_certificate(&self, cert_pem: &str, key_pem: &str) -> Result<(), AegisError>;
}
