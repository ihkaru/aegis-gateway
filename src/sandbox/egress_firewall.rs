// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use regex::Regex;
use std::sync::OnceLock;

use crate::core::error::{AegisError, AegisResult};
use crate::core::sandbox::EgressFirewall;

static URL_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_url_regex() -> &'static Regex {
    URL_REGEX.get_or_init(|| {
        Regex::new(r#"https?://[a-zA-Z0-9.\-_~:/?#\[\]@!$&'()*+,;%=]+"#)
            .expect("Invalid URL regex pattern")
    })
}

/// Production Default-Deny Egress Firewall for Sandbox Execution
pub struct EgressFilterEngine {
    allowed_domains: Vec<String>,
    block_cloud_metadata: bool,
    block_internal_networks: bool,
}

impl Default for EgressFilterEngine {
    fn default() -> Self {
        Self {
            allowed_domains: vec![
                "googleapis.com".to_string(),
                "google.com".to_string(),
                "github.com".to_string(),
                "raw.githubusercontent.com".to_string(),
                "pypi.org".to_string(),
                "pythonhosted.org".to_string(),
                "huggingface.co".to_string(),
            ],
            block_cloud_metadata: true,
            block_internal_networks: true,
        }
    }
}

impl EgressFilterEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_allowed_domains(domains: Vec<String>) -> Self {
        Self {
            allowed_domains: domains,
            block_cloud_metadata: true,
            block_internal_networks: true,
        }
    }

    pub fn add_allowed_domain(&mut self, domain: impl Into<String>) {
        self.allowed_domains.push(domain.into());
    }

    fn extract_host_from_url<'a>(&self, url: &'a str) -> Option<&'a str> {
        let after_scheme = if let Some(idx) = url.find("://") {
            &url[idx + 3..]
        } else {
            return None;
        };

        let host_part = if let Some(slash_idx) = after_scheme.find('/') {
            &after_scheme[..slash_idx]
        } else if let Some(q_idx) = after_scheme.find('?') {
            &after_scheme[..q_idx]
        } else {
            after_scheme
        };

        let host = if let Some(colon_idx) = host_part.find(':') {
            &host_part[..colon_idx]
        } else {
            host_part
        };

        if host.is_empty() {
            None
        } else {
            Some(host)
        }
    }

    fn is_cloud_metadata(&self, host: &str) -> bool {
        let h = host.to_lowercase();
        h == "169.254.169.254"
            || h == "metadata.google.internal"
            || h == "100.100.100.200"
            || h.ends_with(".internal")
    }

    fn is_private_network(&self, host: &str) -> bool {
        let h = host.to_lowercase();
        if h == "localhost"
            || h == "127.0.0.1"
            || h == "0.0.0.0"
            || h == "::1"
            || h.starts_with("10.")
            || h.starts_with("192.168.")
        {
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

    fn is_domain_permitted(&self, host: &str) -> bool {
        let h = host.to_lowercase();
        for pattern in &self.allowed_domains {
            let pat = pattern.to_lowercase();
            if pat.starts_with("*.") {
                let suffix = &pat[1..];
                if h.ends_with(suffix) || h == &pat[2..] {
                    return true;
                }
            } else if h == pat || h.ends_with(&format!(".{pat}")) {
                return true;
            }
        }
        false
    }

    fn evaluate_host(&self, host: &str) -> AegisResult<()> {
        if self.block_cloud_metadata && self.is_cloud_metadata(host) {
            return Err(AegisError::PolicyDenied(format!(
                "SSRF violation: destination host '{host}' is prohibited cloud metadata"
            )));
        }

        if self.block_internal_networks && self.is_private_network(host) {
            return Err(AegisError::PolicyDenied(format!(
                "SSRF violation: destination host '{host}' is prohibited private/loopback network"
            )));
        }

        if !self.is_domain_permitted(host) {
            return Err(AegisError::PolicyDenied(format!(
                "Egress firewall violation: destination host '{host}' is not in allowed domain whitelist"
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl EgressFirewall for EgressFilterEngine {
    async fn check_code_egress(&self, code: &str) -> AegisResult<()> {
        // Direct metadata/private IP string search
        if code.contains("169.254.169.254") || code.contains("metadata.google.internal") {
            return Err(AegisError::PolicyDenied(
                "SSRF violation: source code contains prohibited cloud metadata references".to_string(),
            ));
        }

        // Regex scan for URLs
        let re = get_url_regex();
        for mat in re.find_iter(code) {
            let url = mat.as_str();
            if let Some(host) = self.extract_host_from_url(url) {
                self.evaluate_host(host)?;
            }
        }

        Ok(())
    }

    async fn check_destination(&self, host: &str, _port: u16) -> AegisResult<()> {
        self.evaluate_host(host)
    }
}
