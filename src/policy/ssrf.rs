// SPDX-License-Identifier: MIT

use crate::core::error::{AegisError, AegisResult};
use serde::{Deserialize, Serialize};

/// Security posture level for SSRF validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityPosture {
    Public,
    Standard,
    Hardened,
}

impl Default for SecurityPosture {
    fn default() -> Self {
        SecurityPosture::Hardened
    }
}

/// Evaluates redirect hops and outbound URLs against SSRF policies (MikkoParkkola/mcp-gateway#2508)
#[derive(Debug, Clone)]
pub struct SsrfRedirectValidator {
    posture: SecurityPosture,
    allowed_schemes: Vec<String>,
}

impl Default for SsrfRedirectValidator {
    fn default() -> Self {
        Self {
            posture: SecurityPosture::Hardened,
            allowed_schemes: vec!["http".to_string(), "https".to_string()],
        }
    }
}

impl SsrfRedirectValidator {
    pub fn new(posture: SecurityPosture) -> Self {
        Self {
            posture,
            allowed_schemes: vec!["http".to_string(), "https".to_string()],
        }
    }

    /// Extract host or IP from an absolute URL string
    pub fn extract_host(url_str: &str) -> Option<(String, String)> {
        let scheme_split: Vec<&str> = url_str.split("://").collect();
        if scheme_split.len() < 2 {
            return None;
        }
        let scheme = scheme_split[0].to_lowercase();
        let remainder = scheme_split[1];
        let host_part = remainder.split(['/', '?', '#']).next()?;
        
        let host = if host_part.starts_with('[') {
            if let Some(end) = host_part.find(']') {
                host_part[1..end].to_string()
            } else {
                host_part.to_string()
            }
        } else if let Some(colon) = host_part.find(':') {
            host_part[..colon].to_string()
        } else {
            host_part.to_string()
        };

        Some((scheme, host.to_lowercase()))
    }

    /// Check if target host is a private, loopback, or cloud metadata destination
    pub fn is_prohibited_target(&self, host: &str) -> bool {
        let h = host.trim().to_lowercase();

        // Cloud Metadata Endpoints
        if h == "169.254.169.254"
            || h == "metadata.google.internal"
            || h == "100.100.100.200"
            || h == "instance-data"
        {
            return true;
        }

        // Loopback Endpoints
        if h == "localhost" || h == "127.0.0.1" || h == "0.0.0.0" || h == "::1" || h.starts_with("127.") {
            return true;
        }

        // RFC 1918 Private Ranges
        if h.starts_with("10.") || h.starts_with("192.168.") {
            return true;
        }

        if let Some(second) = h.strip_prefix("172.") {
            if let Some(octet) = second.split('.').next() {
                if let Ok(num) = octet.parse::<u8>() {
                    if (16..=31).contains(&num) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Validates an OAuth redirect hop or outbound target.
    /// Under Hardened posture, private/internal destinations return typed `AegisError::SsrfBlocked`.
    pub fn validate_redirect_target(&self, target_url: &str) -> AegisResult<()> {
        let (scheme, host) = Self::extract_host(target_url).ok_or_else(|| {
            AegisError::SsrfBlocked(format!("Malformed redirect URL '{}'", target_url))
        })?;

        if !self.allowed_schemes.contains(&scheme) {
            return Err(AegisError::SsrfBlocked(format!(
                "Scheme '{}' not permitted for redirect hop: '{}'",
                scheme, target_url
            )));
        }

        if self.posture == SecurityPosture::Hardened && self.is_prohibited_target(&host) {
            return Err(AegisError::SsrfBlocked(format!(
                "Redirect hop to private/metadata destination '{}' refused under {:?} posture",
                target_url, self.posture
            )));
        }

        Ok(())
    }
}
