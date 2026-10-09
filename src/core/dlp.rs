// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use crate::core::error::AegisResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitivityLevel {
    Public,
    Internal,
    ConfidentialPii,
    RestrictedFinancial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceProfile {
    PciDss,
    Hipaa,
    Gdpr,
    Comprehensive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskingStrategy {
    RedactPlaceholder,
    TruncatePan,
    HashSha256,
}

#[derive(Debug, Clone)]
pub struct DlpFinding {
    pub category: String,
    pub field_path: String,
    pub sensitivity: SensitivityLevel,
}

/// Fast pattern matcher abstraction for sub-millisecond scanning
pub trait FastPatternMatcher: Send + Sync {
    fn scan_text(&self, text: &str) -> Vec<DlpFinding>;
}

/// Data Loss Prevention (DLP) pipeline for inspecting and redacting sensitive data
#[async_trait]
pub trait DlpPipeline: Send + Sync {
    /// Scan and mask PII/PCI/PHI from response value before returning to agent
    async fn sanitize_response(&self, payload: Value) -> AegisResult<(Value, Vec<DlpFinding>)>;

    /// Scan input argument payload to prevent data exfiltration
    async fn inspect_request_arguments(&self, arguments: &Value) -> AegisResult<Vec<DlpFinding>>;
}

