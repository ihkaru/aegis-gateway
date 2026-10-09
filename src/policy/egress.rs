use crate::core::CallerContext;
use crate::core::error::AegisError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressRuleConfig {
    pub allowed_domains: Vec<String>,
    pub allow_all_public: bool,
    pub block_internal_networks: bool,
    pub block_cloud_metadata: bool,
}

impl Default for EgressRuleConfig {
    fn default() -> Self {
        Self {
            allowed_domains: Vec::new(),
            allow_all_public: true,
            block_internal_networks: true,
            block_cloud_metadata: true,
        }
    }
}

#[async_trait]
pub trait EgressPolicyGuard: Send + Sync {
    async fn evaluate_url(&self, target_url: &str, caller: &CallerContext) -> Result<(), AegisError>;
    async fn inspect_arguments(&self, args: &serde_json::Value, caller: &CallerContext) -> Result<(), AegisError>;
}

pub struct DefaultEgressGuard {
    config: EgressRuleConfig,
}

impl DefaultEgressGuard {
    pub fn new(config: EgressRuleConfig) -> Self {
        Self { config }
    }

    pub fn with_allowed_domains(domains: Vec<String>) -> Self {
        Self {
            config: EgressRuleConfig {
                allowed_domains: domains,
                allow_all_public: false,
                block_internal_networks: true,
                block_cloud_metadata: true,
            },
        }
    }

    fn is_ip_or_host_blocked(&self, host: &str) -> bool {
        let host_lower = host.to_lowercase();

        if self.config.block_cloud_metadata {
            if host_lower == "169.254.169.254"
                || host_lower == "metadata.google.internal"
                || host_lower == "100.100.100.200"
            {
                return true;
            }
        }

        if self.config.block_internal_networks {
            if host_lower == "localhost"
                || host_lower == "127.0.0.1"
                || host_lower == "0.0.0.0"
                || host_lower.starts_with("10.")
                || host_lower.starts_with("192.168.")
            {
                return true;
            }
            if let Some(second) = host_lower.strip_prefix("172.") {
                if let Some(octet) = second.split('.').next() {
                    if let Ok(num) = octet.parse::<u8>() {
                        if (16..=31).contains(&num) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn is_domain_allowed(&self, host: &str) -> bool {
        if self.config.allow_all_public {
            return true;
        }

        let host_lower = host.to_lowercase();
        for pattern in &self.config.allowed_domains {
            let pat_lower = pattern.to_lowercase();
            if pat_lower.starts_with("*.") {
                let suffix = &pat_lower[1..]; // e.g. ".github.com"
                if host_lower.ends_with(suffix) || host_lower == &pat_lower[2..] {
                    return true;
                }
            } else if host_lower == pat_lower {
                return true;
            }
        }

        false
    }

    fn scan_urls_recursive(&self, val: &serde_json::Value, urls: &mut Vec<String>) {
        match val {
            serde_json::Value::String(s) => {
                if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("file://") {
                    urls.push(s.clone());
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    self.scan_urls_recursive(item, urls);
                }
            }
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    let k_lower = k.to_lowercase();
                    if (k_lower.contains("url") || k_lower.contains("endpoint") || k_lower.contains("uri"))
                        && v.is_string()
                    {
                        if let Some(s) = v.as_str() {
                            urls.push(s.to_string());
                        }
                    } else {
                        self.scan_urls_recursive(v, urls);
                    }
                }
            }
            _ => {}
        }
    }
}

#[async_trait]
impl EgressPolicyGuard for DefaultEgressGuard {
    async fn evaluate_url(&self, target_url: &str, _caller: &CallerContext) -> Result<(), AegisError> {
        let (scheme, rest) = if let Some(idx) = target_url.find("://") {
            (&target_url[..idx], &target_url[idx + 3..])
        } else {
            return Err(AegisError::PolicyDenied(format!(
                "Invalid URL syntax in egress check: missing scheme in '{}'",
                target_url
            )));
        };

        let scheme_lower = scheme.to_lowercase();
        if scheme_lower != "http" && scheme_lower != "https" {
            return Err(AegisError::PolicyDenied(format!(
                "Blocked prohibited URL scheme '{}': only http/https allowed",
                scheme_lower
            )));
        }

        let host_part = if let Some(slash_idx) = rest.find('/') {
            &rest[..slash_idx]
        } else if let Some(q_idx) = rest.find('?') {
            &rest[..q_idx]
        } else {
            rest
        };

        let host = if let Some(colon_idx) = host_part.find(':') {
            &host_part[..colon_idx]
        } else {
            host_part
        };

        if host.is_empty() {
            return Err(AegisError::PolicyDenied(
                "Missing host in egress target URL".to_string(),
            ));
        }

        if self.is_ip_or_host_blocked(host) {
            return Err(AegisError::PolicyDenied(format!(
                "SSRF/Egress policy violation: destination host '{}' is blocked (private/metadata network)",
                host
            )));
        }

        if !self.is_domain_allowed(host) {
            return Err(AegisError::PolicyDenied(format!(
                "Source-rights policy violation: domain '{}' is not in allowed egress domain list",
                host
            )));
        }

        Ok(())
    }

    async fn inspect_arguments(&self, args: &serde_json::Value, caller: &CallerContext) -> Result<(), AegisError> {
        let mut urls = Vec::new();
        self.scan_urls_recursive(args, &mut urls);

        for u in urls {
            self.evaluate_url(&u, caller).await?;
        }

        Ok(())
    }
}
