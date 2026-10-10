// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::error::AegisResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSyncMessage {
    pub message_id: String,
    pub source_node_id: String,
    pub event_type: String,
    pub payload_json: String,
    pub version: u64,
    pub timestamp_unix: u64,
    pub checksum_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterSyncReceipt {
    pub node_id: String,
    pub applied: bool,
    pub current_version: u64,
}

/// Interface-First abstraction for Distributed Multi-Node Cluster State Synchronization
#[async_trait]
pub trait ClusterSyncEngine: Send + Sync {
    async fn broadcast_update(&self, event_type: &str, payload_json: &str) -> AegisResult<ClusterSyncMessage>;
    async fn ingest_peer_update(&self, message: &ClusterSyncMessage) -> AegisResult<ClusterSyncReceipt>;
    async fn get_cluster_nodes(&self) -> AegisResult<Vec<String>>;
}
