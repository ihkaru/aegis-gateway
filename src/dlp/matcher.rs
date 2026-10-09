// SPDX-License-Identifier: MIT

use regex::Regex;
use std::sync::LazyLock;
use crate::core::dlp::{DlpFinding, FastPatternMatcher, SensitivityLevel};

static EMAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}").expect("Valid regex")
});

static PAN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:\d{4}[ -]?){3}\d{4}\b|\b\d{15,16}\b").expect("Valid regex")
});

static SSN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("Valid regex")
});

static SECRET_KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:sk_live_[a-z0-9]{20,}|ghp_[a-z0-9]{30,}|xoxb-[a-z0-9-]{20,})\b").expect("Valid regex")
});

static MRN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bMRN-[0-9]{6,10}\b").expect("Valid regex")
});

/// High-performance compiled pattern matcher with Luhn algorithm validation
#[derive(Default)]
pub struct FastMatcher;

impl FastMatcher {
    pub fn new() -> Self {
        Self
    }

    /// Validates numeric credit card string via Luhn algorithm (mod 10)
    pub fn validate_luhn(pan: &str) -> bool {
        let digits: Vec<u32> = pan.chars().filter_map(|c| c.to_digit(10)).collect();
        if digits.len() < 13 || digits.len() > 19 {
            return false;
        }

        let mut sum = 0;
        let mut double = false;
        for &digit in digits.iter().rev() {
            if double {
                let d = digit * 2;
                sum += if d > 9 { d - 9 } else { d };
            } else {
                sum += digit;
            }
            double = !double;
        }
        sum % 10 == 0
    }
}

impl FastPatternMatcher for FastMatcher {
    fn scan_text(&self, text: &str) -> Vec<DlpFinding> {
        let mut findings = Vec::new();

        if EMAIL_RE.is_match(text) {
            findings.push(DlpFinding {
                category: "EMAIL_ADDRESS".to_string(),
                field_path: "text".to_string(),
                sensitivity: SensitivityLevel::ConfidentialPii,
            });
        }

        if SSN_RE.is_match(text) {
            findings.push(DlpFinding {
                category: "US_SSN".to_string(),
                field_path: "text".to_string(),
                sensitivity: SensitivityLevel::ConfidentialPii,
            });
        }

        if MRN_RE.is_match(text) {
            findings.push(DlpFinding {
                category: "HIPAA_MRN".to_string(),
                field_path: "text".to_string(),
                sensitivity: SensitivityLevel::ConfidentialPii,
            });
        }

        for cap in PAN_RE.find_iter(text) {
            let candidate = cap.as_str().replace([' ', '-'], "");
            if Self::validate_luhn(&candidate) {
                findings.push(DlpFinding {
                    category: "PCI_PAN".to_string(),
                    field_path: "text".to_string(),
                    sensitivity: SensitivityLevel::RestrictedFinancial,
                });
                break;
            }
        }

        if SECRET_KEY_RE.is_match(text) {
            findings.push(DlpFinding {
                category: "API_SECRET_KEY".to_string(),
                field_path: "text".to_string(),
                sensitivity: SensitivityLevel::RestrictedFinancial,
            });
        }

        findings
    }
}
