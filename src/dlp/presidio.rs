// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::core::dlp::{
    ComplianceProfile, DlpFinding, DlpPipeline, FastPatternMatcher, MaskingStrategy,
};
use crate::core::error::AegisResult;
use crate::dlp::matcher::FastMatcher;


/// Enterprise Presidio-compatible DLP Pipeline with compliance regulatory profiles
pub struct PresidioDlpPipeline {
    profile: ComplianceProfile,
    strategy: MaskingStrategy,
    matcher: FastMatcher,
    presidio_endpoint: Option<String>,
}

impl PresidioDlpPipeline {
    pub fn new() -> Self {
        Self {
            profile: ComplianceProfile::Comprehensive,
            strategy: MaskingStrategy::RedactPlaceholder,
            matcher: FastMatcher::new(),
            presidio_endpoint: None,
        }
    }

    pub fn with_profile(mut self, profile: ComplianceProfile) -> Self {
        self.profile = profile;
        self
    }

    pub fn with_strategy(mut self, strategy: MaskingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.presidio_endpoint = Some(endpoint.into());
        self
    }

    fn mask_pan(&self, pan_str: &str) -> String {
        match self.strategy {
            MaskingStrategy::TruncatePan => {
                let clean = pan_str.replace([' ', '-'], "");
                if clean.len() >= 10 {
                    let first6 = &clean[..6];
                    let last4 = &clean[clean.len() - 4..];
                    format!("{}******{}", first6, last4)
                } else {
                    "[REDACTED_PCI_PAN]".to_string()
                }
            }
            MaskingStrategy::HashSha256 => {
                let digest = Sha256::digest(pan_str.as_bytes());
                let hash = digest.iter().fold(String::with_capacity(64), |mut acc, b| {
                    use std::fmt::Write;
                    let _ = write!(acc, "{:02x}", b);
                    acc
                });
                format!("[REDACTED_HASH:{}]", &hash[..12])
            }

            MaskingStrategy::RedactPlaceholder => "[REDACTED_PCI_PAN]".to_string(),
        }
    }

    fn sanitize_str(&self, text: &str) -> (String, Vec<DlpFinding>) {
        let findings = self.matcher.scan_text(text);
        if findings.is_empty() {
            return (text.to_string(), findings);
        }

        let mut out = text.to_string();

        for finding in &findings {
            match finding.category.as_str() {
                "PCI_PAN" if self.profile == ComplianceProfile::PciDss || self.profile == ComplianceProfile::Comprehensive => {
                    // Extract PAN candidate and mask
                    out = self.mask_pan(&out);
                }
                "HIPAA_MRN" if self.profile == ComplianceProfile::Hipaa || self.profile == ComplianceProfile::Comprehensive => {
                    out = "[REDACTED_HIPAA_MRN]".to_string();
                }
                "US_SSN" => {
                    out = "[REDACTED_SSN]".to_string();
                }
                "API_SECRET_KEY" => {
                    out = "[REDACTED_SECRET]".to_string();
                }
                "EMAIL_ADDRESS" if self.profile == ComplianceProfile::Gdpr || self.profile == ComplianceProfile::Comprehensive => {
                    out = "[REDACTED_EMAIL]".to_string();
                }
                _ => {}
            }
        }

        (out, findings)
    }

    fn recursive_sanitize(&self, value: Value, path: &str) -> (Value, Vec<DlpFinding>) {
        let mut total_findings = Vec::new();

        match value {
            Value::String(s) => {
                let (masked, mut findings) = self.sanitize_str(&s);
                for f in &mut findings {
                    f.field_path = path.to_string();
                }
                total_findings.append(&mut findings);
                (Value::String(masked), total_findings)
            }
            Value::Array(arr) => {
                let mut new_arr = Vec::new();
                for (idx, item) in arr.into_iter().enumerate() {
                    let subpath = format!("{}[{}]", path, idx);
                    let (sanitized_item, mut item_findings) = self.recursive_sanitize(item, &subpath);
                    total_findings.append(&mut item_findings);
                    new_arr.push(sanitized_item);
                }
                (Value::Array(new_arr), total_findings)
            }
            Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    let subpath = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", path, k)
                    };
                    let (sanitized_val, mut val_findings) = self.recursive_sanitize(v, &subpath);
                    total_findings.append(&mut val_findings);
                    new_map.insert(k, sanitized_val);
                }
                (Value::Object(new_map), total_findings)
            }
            other => (other, total_findings),
        }
    }
}

impl Default for PresidioDlpPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DlpPipeline for PresidioDlpPipeline {
    async fn sanitize_response(&self, payload: Value) -> AegisResult<(Value, Vec<DlpFinding>)> {
        let (sanitized, findings) = self.recursive_sanitize(payload, "");
        Ok((sanitized, findings))
    }

    async fn inspect_request_arguments(&self, arguments: &Value) -> AegisResult<Vec<DlpFinding>> {
        let (_, findings) = self.recursive_sanitize(arguments.clone(), "args");
        Ok(findings)
    }
}
