// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::error::AegisResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCertEnvelope {
    pub agent_id: String,
    pub public_key_pem: String,
    pub nonce: String,
    pub timestamp_unix: u64,
    pub signature_hex: String,
    pub declared_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentTrustLevel {
    Untrusted,
    Standard,
    VerifiedPartner,
    InternalCertified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentVerificationOutcome {
    pub verified: bool,
    pub trust_level: AgentTrustLevel,
    pub tenant_id: String,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThreatEvidenceReport {
    pub url: String,
    pub domain: String,
    pub risk_score: f32,
    pub malicious: bool,
    pub threat_categories: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorsEvaluationResult {
    pub allowed: bool,
    pub allowed_origin: Option<String>,
    pub allowed_headers: Option<String>,
    pub max_age_secs: u32,
}

/// Interface-First abstraction for Cryptographic Agent Trust Verification
#[async_trait]
pub trait AgentTrustVerifier: Send + Sync {
    async fn verify_agent_identity(&self, cert: &AgentCertEnvelope) -> AegisResult<AgentVerificationOutcome>;
}

/// Interface-First abstraction for Dynamic URL Threat Evidence Enrichment
#[async_trait]
pub trait ThreatEvidenceEnricher: Send + Sync {
    async fn inspect_and_enrich_url(&self, target_url: &str) -> AegisResult<ThreatEvidenceReport>;
}

/// Interface-First abstraction for Granular Streamable HTTP CORS Policy
pub trait CorsPolicyEnforcer: Send + Sync {
    fn evaluate_cors_origin(&self, origin: Option<&str>, requested_headers: Option<&str>) -> CorsEvaluationResult;
}
