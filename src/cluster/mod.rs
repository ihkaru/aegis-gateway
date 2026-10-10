// SPDX-License-Identifier: MIT

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::core::cluster_sync::{ClusterSyncEngine, ClusterSyncMessage, ClusterSyncReceipt};
use crate::core::error::{AegisError, AegisResult};

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(acc, "{:02x}", b);
        acc
    })
}

/// Production Distributed Pub/Sub Cluster State Synchronization Engine
pub struct DistributedPubSubClusterSync {
    local_node_id: String,
    known_nodes: Arc<RwLock<HashSet<String>>>,
    current_version: Arc<AtomicU64>,
}

impl DistributedPubSubClusterSync {
    pub fn new(local_node_id: impl Into<String>, initial_version: u64) -> Self {
        let node_id = local_node_id.into();
        let mut nodes = HashSet::new();
        nodes.insert(node_id.clone());

        Self {
            local_node_id: node_id,
            known_nodes: Arc::new(RwLock::new(nodes)),
            current_version: Arc::new(AtomicU64::new(initial_version)),
        }
    }

    fn now_unix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn compute_checksum(event_type: &str, payload_json: &str, version: u64, node_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(event_type.as_bytes());
        hasher.update(b":");
        hasher.update(payload_json.as_bytes());
        hasher.update(b":");
        hasher.update(&version.to_be_bytes());
        hasher.update(b":");
        hasher.update(node_id.as_bytes());
        to_hex(&hasher.finalize())
    }
}

#[async_trait]
impl ClusterSyncEngine for DistributedPubSubClusterSync {
    async fn broadcast_update(&self, event_type: &str, payload_json: &str) -> AegisResult<ClusterSyncMessage> {
        let new_ver = self.current_version.fetch_add(1, Ordering::SeqCst) + 1;
        let ts = Self::now_unix();
        let checksum = Self::compute_checksum(event_type, payload_json, new_ver, &self.local_node_id);
        let msg_id = format!("sync-{}-{}-{}", self.local_node_id, new_ver, ts);

        Ok(ClusterSyncMessage {
            message_id: msg_id,
            source_node_id: self.local_node_id.clone(),
            event_type: event_type.to_string(),
            payload_json: payload_json.to_string(),
            version: new_ver,
            timestamp_unix: ts,
            checksum_sha256: checksum,
        })
    }

    async fn ingest_peer_update(&self, message: &ClusterSyncMessage) -> AegisResult<ClusterSyncReceipt> {
        // 1. Checksum validation against tampering
        let expected_checksum = Self::compute_checksum(
            &message.event_type,
            &message.payload_json,
            message.version,
            &message.source_node_id,
        );

        if message.checksum_sha256 != expected_checksum {
            return Err(AegisError::SecurityRefusal(format!(
                "Cluster sync message {} checksum mismatch: calculated {}, expected {}",
                message.message_id, expected_checksum, message.checksum_sha256
            )));
        }

        // 2. Anti-regression & idempotence check
        let cur = self.current_version.load(Ordering::SeqCst);
        if message.version <= cur {
            // Already seen or older version: safe no-op
            return Ok(ClusterSyncReceipt {
                node_id: self.local_node_id.clone(),
                applied: false,
                current_version: cur,
            });
        }

        // 3. Update version & track peer node
        self.current_version.store(message.version, Ordering::SeqCst);
        let mut nodes_lock = self.known_nodes.write().await;
        nodes_lock.insert(message.source_node_id.clone());

        Ok(ClusterSyncReceipt {
            node_id: self.local_node_id.clone(),
            applied: true,
            current_version: message.version,
        })
    }

    async fn get_cluster_nodes(&self) -> AegisResult<Vec<String>> {
        let lock = self.known_nodes.read().await;
        let mut list: Vec<String> = lock.iter().cloned().collect();
        list.sort();
        Ok(list)
    }
}
