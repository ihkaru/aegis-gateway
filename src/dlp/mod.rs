// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::core::dlp::{DlpFinding, DlpPipeline, SensitivityLevel};
use crate::core::error::AegisResult;

pub mod matcher;
pub mod presidio;

pub use matcher::FastMatcher;
pub use presidio::PresidioDlpPipeline;

/// Real-time PII & Sensitive Data Loss Prevention Pipeline
pub struct PiiDlpPipeline {

    credit_card_regex: Regex,
    ssn_regex: Regex,
    api_key_regex: Regex,
}

impl Default for PiiDlpPipeline {
    fn default() -> Self {
        Self {
            // Visa, Mastercard, Amex, Discover pattern
            credit_card_regex: Regex::new(r"\b(?:\d{4}[ -]?){3}\d{4}\b").expect("Valid regex"),
            // US SSN / NIK pattern
            ssn_regex: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("Valid regex"),
            // API key / private key probe
            api_key_regex: Regex::new(r"(?i)\b(?:sk_live|ghp_|eyJh)[a-zA-Z0-9_\-]{16,}\b")
                .expect("Valid regex"),
        }
    }
}

impl PiiDlpPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    fn mask_string(&self, input: &str) -> (String, Vec<DlpFinding>) {
        let mut findings = Vec::new();
        let mut text = input.to_string();

        if self.credit_card_regex.is_match(&text) {
            findings.push(DlpFinding {
                category: "PCI_CREDIT_CARD".to_string(),
                field_path: "text".to_string(),
                sensitivity: SensitivityLevel::RestrictedFinancial,
            });
            text = self
                .credit_card_regex
                .replace_all(&text, "[REDACTED_CREDIT_CARD]")
                .to_string();
        }

        if self.ssn_regex.is_match(&text) {
            findings.push(DlpFinding {
                category: "PII_SSN".to_string(),
                field_path: "text".to_string(),
                sensitivity: SensitivityLevel::ConfidentialPii,
            });
            text = self
                .ssn_regex
                .replace_all(&text, "[REDACTED_SSN]")
                .to_string();
        }

        if self.api_key_regex.is_match(&text) {
            findings.push(DlpFinding {
                category: "SECRET_API_KEY".to_string(),
                field_path: "text".to_string(),
                sensitivity: SensitivityLevel::RestrictedFinancial,
            });
            text = self
                .api_key_regex
                .replace_all(&text, "[REDACTED_SECRET]")
                .to_string();
        }

        (text, findings)
    }

    fn recursively_mask(&self, val: Value, findings: &mut Vec<DlpFinding>) -> Value {
        match val {
            Value::String(s) => {
                let (masked, found) = self.mask_string(&s);
                findings.extend(found);
                Value::String(masked)
            }
            Value::Array(arr) => {
                let masked_arr = arr
                    .into_iter()
                    .map(|item| self.recursively_mask(item, findings))
                    .collect();
                Value::Array(masked_arr)
            }
            Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    new_map.insert(k, self.recursively_mask(v, findings));
                }
                Value::Object(new_map)
            }
            primitive => primitive,
        }
    }
}

#[async_trait]
impl DlpPipeline for PiiDlpPipeline {
    async fn sanitize_response(&self, payload: Value) -> AegisResult<(Value, Vec<DlpFinding>)> {
        let mut findings = Vec::new();
        let masked = self.recursively_mask(payload, &mut findings);
        Ok((masked, findings))
    }

    async fn inspect_request_arguments(&self, arguments: &Value) -> AegisResult<Vec<DlpFinding>> {
        let mut findings = Vec::new();
        let _ = self.recursively_mask(arguments.clone(), &mut findings);
        Ok(findings)
    }
}
