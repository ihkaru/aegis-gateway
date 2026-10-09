//! Tool Outcome Attestation (TOA) Engine
//! Resolves microsoft/mcp-gateway#102 & docker/mcp-gateway#557
//! Provides tamper-evident cryptographic provenance for MCP tool executions.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolOutcomeAttestation {
    pub tool: String,
    pub arguments_hash_sha256: String,
    pub result_hash_sha256: String,
    pub timestamp_utc: String,
    pub duration_ms: u64,
    pub gateway_identity: String,
    pub attestation_token: String,
}

impl ToolOutcomeAttestation {
    pub fn generate(
        tool: &str,
        arguments: &Value,
        result: &Value,
        duration_ms: u64,
        secret_key: &str,
        gateway_identity: &str,
    ) -> Self {
        let args_hash = Self::hash_json(arguments);
        let res_hash = Self::hash_json(result);
        let timestamp = Utc::now().to_rfc3339();

        let token = Self::compute_token(
            secret_key,
            gateway_identity,
            tool,
            &args_hash,
            &res_hash,
            &timestamp,
            duration_ms,
        );

        Self {
            tool: tool.to_string(),
            arguments_hash_sha256: args_hash,
            result_hash_sha256: res_hash,
            timestamp_utc: timestamp,
            duration_ms,
            gateway_identity: gateway_identity.to_string(),
            attestation_token: token,
        }
    }

    pub fn verify(&self, secret_key: &str) -> bool {
        let expected_token = Self::compute_token(
            secret_key,
            &self.gateway_identity,
            &self.tool,
            &self.arguments_hash_sha256,
            &self.result_hash_sha256,
            &self.timestamp_utc,
            self.duration_ms,
        );
        self.attestation_token == expected_token
    }

    pub fn hash_json(val: &Value) -> String {
        let s = serde_json::to_string(val).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        let result = hasher.finalize();
        result.iter().fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{:02x}", b);
            acc
        })
    }

    fn compute_token(
        secret_key: &str,
        gateway_id: &str,
        tool: &str,
        args_hash: &str,
        res_hash: &str,
        timestamp: &str,
        duration_ms: u64,
    ) -> String {
        let raw = format!("{secret_key}:{gateway_id}:{tool}:{args_hash}:{res_hash}:{timestamp}:{duration_ms}");
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        let result = hasher.finalize();
        result.iter().fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{:02x}", b);
            acc
        })
    }

    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}
