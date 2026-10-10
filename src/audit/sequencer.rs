// SPDX-License-Identifier: MIT

use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::audit::{AuditChainVerifier, AuditEvent, HashChainedEvent};
use crate::core::error::{AegisError, AegisResult};

/// Cryptographically chained tamper-evident audit log sequencer (SOC 2 Type II compliant)
pub struct HashChainSequencer {
    genesis_hash: String,
    chain: Arc<RwLock<Vec<HashChainedEvent>>>,
}

impl HashChainSequencer {
    pub fn new() -> Self {
        Self {
            genesis_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            chain: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_genesis(genesis: impl Into<String>) -> Self {
        Self {
            genesis_hash: genesis.into(),
            chain: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Compute cryptographic SHA-256 hash for an event chained to previous hash
    pub fn compute_event_hash(prev_hash: &str, seq: u64, event: &AuditEvent) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(seq.to_be_bytes());
        hasher.update(event.event_id.as_bytes());
        hasher.update(event.timestamp.to_rfc3339().as_bytes());
        hasher.update(event.caller.subject.as_bytes());
        hasher.update(event.caller.tenant_id.as_str().as_bytes());
        hasher.update(event.target_resource.as_bytes());
        hasher.update(event.payload_hash_sha256.as_bytes());
        let result = hasher.finalize();
        result.iter().fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{:02x}", b);
            acc
        })
    }


    /// Record and append a new audit event to the tamper-evident chain
    pub async fn record(&self, event: AuditEvent) -> HashChainedEvent {
        let mut chain = self.chain.write().await;
        let seq = chain.len() as u64;
        let prev_hash = if let Some(last) = chain.last() {
            last.event_hash.clone()
        } else {
            self.genesis_hash.clone()
        };

        let event_hash = Self::compute_event_hash(&prev_hash, seq, &event);
        let chained = HashChainedEvent {
            sequence: seq,
            previous_hash: prev_hash,
            event_hash,
            event,
        };

        chain.push(chained.clone());
        chained
    }

    pub async fn len(&self) -> usize {
        self.chain.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.chain.read().await.is_empty()
    }

    pub async fn get_chain(&self) -> Vec<HashChainedEvent> {
        self.chain.read().await.clone()
    }

    /// Export verifiable SOC 2 Type II audit proof report
    pub async fn export_soc2_report(&self) -> serde_json::Value {
        let chain = self.chain.read().await;
        let is_valid = self.verify_chain(&chain).unwrap_or(false);
        let latest_root = chain.last().map(|e| e.event_hash.clone()).unwrap_or_else(|| self.genesis_hash.clone());

        serde_json::json!({
            "standard": "SOC 2 Type II / ISO 27001",
            "chain_length": chain.len(),
            "genesis_root": self.genesis_hash,
            "latest_checkpoint_hash": latest_root,
            "cryptographic_integrity_verified": is_valid,
            "generated_at_utc": chrono::Utc::now().to_rfc3339(),
        })
    }
}

impl Default for HashChainSequencer {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditChainVerifier for HashChainSequencer {
    fn verify_chain(&self, events: &[HashChainedEvent]) -> AegisResult<bool> {
        if events.is_empty() {
            return Ok(true);
        }

        let mut expected_prev = self.genesis_hash.clone();

        for (idx, item) in events.iter().enumerate() {
            if item.sequence != idx as u64 {
                return Err(AegisError::AuditError(format!(
                    "Chain sequence discontinuity at index {}: item has sequence {}",
                    idx, item.sequence
                )));
            }

            if item.previous_hash != expected_prev {
                return Err(AegisError::AuditError(format!(
                    "Tampering detected at sequence {}: previous_hash mismatch! expected '{}', found '{}'",
                    item.sequence, expected_prev, item.previous_hash
                )));
            }

            let expected_hash = Self::compute_event_hash(&expected_prev, item.sequence, &item.event);
            if item.event_hash != expected_hash {
                return Err(AegisError::AuditError(format!(
                    "Tampering detected at sequence {}: hash payload mismatch! expected '{}', found '{}'",
                    item.sequence, expected_hash, item.event_hash
                )));
            }

            expected_prev = item.event_hash.clone();
        }

        Ok(true)
    }
}

#[async_trait::async_trait]
impl crate::core::audit::AuditSink for HashChainSequencer {
    async fn emit(&self, event: &AuditEvent) -> AegisResult<()> {
        self.record(event.clone()).await;
        Ok(())
    }
}
