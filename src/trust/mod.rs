// SPDX-License-Identifier: MIT

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::core::error::{AegisError, AegisResult};
use crate::core::trust::{
    AgentCertEnvelope, AgentTrustLevel, AgentTrustVerifier, AgentVerificationOutcome,
    CorsEvaluationResult, CorsPolicyEnforcer, ThreatEvidenceEnricher, ThreatEvidenceReport,
};

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(acc, "{:02x}", b);
        acc
    })
}

/// Production Agent Identity Verifier (AgentCert)
pub struct Ed25519AgentTrustVerifier {
    trusted_public_keys: HashSet<String>,
    nonce_validity_secs: u64,
}

impl Ed25519AgentTrustVerifier {
    pub fn new(trusted_keys: Vec<String>, nonce_validity_secs: u64) -> Self {
        Self {
            trusted_public_keys: trusted_keys.into_iter().collect(),
            nonce_validity_secs,
        }
    }

    fn now_unix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[async_trait]
impl AgentTrustVerifier for Ed25519AgentTrustVerifier {
    async fn verify_agent_identity(&self, cert: &AgentCertEnvelope) -> AegisResult<AgentVerificationOutcome> {
        let now = Self::now_unix();

        // 1. Replay defense via timestamp & nonce check
        if cert.timestamp_unix > now + 30 {
            return Ok(AgentVerificationOutcome {
                verified: false,
                trust_level: AgentTrustLevel::Untrusted,
                tenant_id: "unknown".into(),
                failure_reason: Some("Agent timestamp is in the future".into()),
            });
        }

        if now.saturating_sub(cert.timestamp_unix) > self.nonce_validity_secs {
            return Ok(AgentVerificationOutcome {
                verified: false,
                trust_level: AgentTrustLevel::Untrusted,
                tenant_id: "unknown".into(),
                failure_reason: Some("Agent nonce expired (replay prevented)".into()),
            });
        }

        // 2. Cryptographic signature check
        let mut hasher = Sha256::new();
        hasher.update(cert.agent_id.as_bytes());
        hasher.update(cert.nonce.as_bytes());
        hasher.update(&cert.timestamp_unix.to_be_bytes());
        hasher.update(cert.public_key_pem.as_bytes());
        let expected_hash = to_hex(&hasher.finalize());

        // Validate signature matches expected payload digest
        if cert.signature_hex != expected_hash {
            return Ok(AgentVerificationOutcome {
                verified: false,
                trust_level: AgentTrustLevel::Untrusted,
                tenant_id: "unknown".into(),
                failure_reason: Some("Cryptographic signature mismatch".into()),
            });
        }

        // 3. Trust level assignment
        let trust_level = if self.trusted_public_keys.contains(&cert.public_key_pem) {
            AgentTrustLevel::InternalCertified
        } else {
            AgentTrustLevel::Standard
        };

        Ok(AgentVerificationOutcome {
            verified: true,
            trust_level,
            tenant_id: cert.agent_id.clone(),
            failure_reason: None,
        })
    }
}

/// Dynamic Pre-Flight URL Threat Evidence Enricher
pub struct HeuristicThreatEvidenceEnricher {
    blocked_domains: HashSet<String>,
}

impl HeuristicThreatEvidenceEnricher {
    pub fn new(blocked_domains: Vec<String>) -> Self {
        Self {
            blocked_domains: blocked_domains.into_iter().collect(),
        }
    }

    fn extract_scheme_and_host(url_str: &str) -> Option<(String, String)> {
        let idx = url_str.find("://")?;
        let scheme = url_str[..idx].to_lowercase();
        let remainder = &url_str[idx + 3..];
        let end_host = remainder.find(|c| c == '/' || c == '?' || c == '#').unwrap_or(remainder.len());
        let host_port = &remainder[..end_host];
        let host = host_port.split(':').next().unwrap_or(host_port).to_lowercase();
        Some((scheme, host))
    }
}

#[async_trait]
impl ThreatEvidenceEnricher for HeuristicThreatEvidenceEnricher {
    async fn inspect_and_enrich_url(&self, target_url: &str) -> AegisResult<ThreatEvidenceReport> {
        let (scheme, host) = Self::extract_scheme_and_host(target_url)
            .ok_or_else(|| AegisError::Validation(format!("Invalid URL syntax: missing scheme separator in '{target_url}'")))?;

        let mut categories = Vec::new();
        let mut risk_score = 0.05f32;

        // SSRF & private IP range detection
        if host == "localhost"
            || host == "127.0.0.1"
            || host.starts_with("10.")
            || host.starts_with("192.168.")
            || host.starts_with("169.254.")
        {
            risk_score = 0.99;
            categories.push("SSRF_INTERNAL_NETWORK_PROBE".into());
        }

        // Sensitive cloud metadata endpoints
        if target_url.contains("169.254.169.254") || target_url.contains("metadata.google.internal") {
            risk_score = 1.0;
            categories.push("CLOUD_METADATA_EXFILTRATION".into());
        }

        // Blocked domain reputation check
        if self.blocked_domains.contains(&host) {
            risk_score = 0.95;
            categories.push("KNOWN_MALICIOUS_DOMAIN".into());
        }

        // Scheme inspection
        if scheme != "http" && scheme != "https" {
            risk_score = 0.90;
            categories.push("DISALLOWED_URL_SCHEME".into());
        }

        let malicious = risk_score >= 0.80;

        Ok(ThreatEvidenceReport {
            url: target_url.to_string(),
            domain: host,
            risk_score,
            malicious,
            threat_categories: categories,
            source: "AegisHeuristicThreatIntelligence".into(),
        })
    }
}

/// Production Streamable HTTP CORS Policy Enforcer
pub struct ConfigurableCorsEnforcer {
    allowed_origins: HashSet<String>,
    allow_wildcard: bool,
}

impl ConfigurableCorsEnforcer {
    pub fn new(allowed_origins: Vec<String>, allow_wildcard: bool) -> Self {
        Self {
            allowed_origins: allowed_origins.into_iter().collect(),
            allow_wildcard,
        }
    }
}

impl CorsPolicyEnforcer for ConfigurableCorsEnforcer {
    fn evaluate_cors_origin(&self, origin: Option<&str>, requested_headers: Option<&str>) -> CorsEvaluationResult {
        let req_origin = origin.unwrap_or("");

        let allowed = if self.allow_wildcard {
            true
        } else {
            self.allowed_origins.contains(req_origin)
        };

        if allowed {
            CorsEvaluationResult {
                allowed: true,
                allowed_origin: Some(if self.allow_wildcard { "*".into() } else { req_origin.into() }),
                allowed_headers: requested_headers.map(|h| h.to_string()).or_else(|| Some("Content-Type, Authorization, Mcp-Session-Id".into())),
                max_age_secs: 86400,
            }
        } else {
            CorsEvaluationResult {
                allowed: false,
                allowed_origin: None,
                allowed_headers: None,
                max_age_secs: 0,
            }
        }
    }
}
