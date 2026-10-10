// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::error::AegisResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryBlobMetadata {
    pub tenant_id: String,
    pub file_name: String,
    pub declared_mime: String,
    pub expected_size: u64,
    pub sha256_checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadSession {
    pub session_id: String,
    pub chunk_size: u32,
    pub expires_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkAck {
    pub session_id: String,
    pub bytes_received: u64,
    pub total_received: u64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredBlobDescriptor {
    pub blob_urn: String,
    pub storage_path: String,
    pub actual_mime: String,
    pub size_bytes: u64,
    pub sha256_checksum: String,
    pub created_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemeralEgressTicket {
    pub ticket_id: String,
    pub blob_urn: String,
    pub direct_download_url: String,
    pub expires_at_unix: u64,
    pub hmac_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimeInspectionResult {
    pub detected_mime: String,
    pub magic_bytes_matched: bool,
    pub safe_for_egress: bool,
}

/// Interface-First abstraction for large multi-modal and chunked binary streaming
#[async_trait]
pub trait BinaryStreamingVault: Send + Sync {
    async fn initiate_resumable_upload(&self, metadata: &BinaryBlobMetadata) -> AegisResult<UploadSession>;
    async fn append_chunk(&self, session_id: &str, offset: u64, chunk: Vec<u8>) -> AegisResult<ChunkAck>;
    async fn finalize_upload(&self, session_id: &str, expected_sha256: Option<&str>) -> AegisResult<StoredBlobDescriptor>;
    async fn generate_ephemeral_egress_ticket(&self, blob_urn: &str, client_id: &str, ttl_secs: u64) -> AegisResult<EphemeralEgressTicket>;
    async fn inspect_blob_content(&self, blob_urn: &str) -> AegisResult<MimeInspectionResult>;
}
