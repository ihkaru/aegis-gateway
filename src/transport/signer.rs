//! MCP Wire Message Signing & Integrity Verification (MCPS)
//! Resolves docker/mcp-gateway#453 ("MCP messages pass through the gateway unsigned")
//! Implements non-repudiation message signing and verification for MCP transports.

use chrono::Utc;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct McpMessageSigner {
    secret_key: String,
    gateway_id: String,
}

impl McpMessageSigner {
    pub const DEFAULT_MAX_CLOCK_SKEW_SECS: u64 = 300; // 5 minutes

    pub fn new(secret_key: impl Into<String>, gateway_id: impl Into<String>) -> Self {
        Self {
            secret_key: secret_key.into(),
            gateway_id: gateway_id.into(),
        }
    }

    /// Sign an MCP wire message payload returning `t=<timestamp>,v1=<signature>`
    pub fn sign_message(&self, payload: &str) -> String {
        let timestamp = Utc::now().timestamp();
        let sig = self.compute_signature(timestamp, payload);
        format!("t={},v1={}", timestamp, sig)
    }

    /// Verify an `x-mcp-signature` header against a payload
    pub fn verify_signature(&self, payload: &str, header_val: &str, max_skew_secs: u64) -> bool {
        let (ts, sig) = match self.parse_header(header_val) {
            Some(res) => res,
            None => return false,
        };

        // Check clock skew
        let now = Utc::now().timestamp();
        if (now - ts).unsigned_abs() > max_skew_secs {
            return false;
        }

        let expected_sig = self.compute_signature(ts, payload);
        sig == expected_sig
    }

    fn compute_signature(&self, timestamp: i64, payload: &str) -> String {
        let raw = format!("{}:{}:{}:{}", self.secret_key, self.gateway_id, timestamp, payload);
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        let result = hasher.finalize();
        result.iter().fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{:02x}", b);
            acc
        })
    }

    fn parse_header(&self, header_val: &str) -> Option<(i64, String)> {
        let mut timestamp = None;
        let mut signature = None;

        for part in header_val.split(',') {
            let part = part.trim();
            if let Some(ts_str) = part.strip_prefix("t=") {
                timestamp = ts_str.parse::<i64>().ok();
            } else if let Some(sig_str) = part.strip_prefix("v1=") {
                signature = Some(sig_str.to_string());
            }
        }

        match (timestamp, signature) {
            (Some(ts), Some(sig)) => Some((ts, sig)),
            _ => None,
        }
    }
}
