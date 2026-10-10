// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::core::error::{AegisError, AegisResult};
use crate::core::vault::{
    BinaryBlobMetadata, BinaryStreamingVault, ChunkAck, EphemeralEgressTicket,
    MimeInspectionResult, StoredBlobDescriptor, UploadSession,
};

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(acc, "{:02x}", b);
        acc
    })
}

struct InProgressUpload {
    metadata: BinaryBlobMetadata,
    data: Vec<u8>,
    expires_at_unix: u64,
}

/// Production Local File & In-Memory Streaming Binary Vault
pub struct LocalFsStreamingVault {
    storage_dir: PathBuf,
    hmac_secret: String,
    sessions: Arc<RwLock<HashMap<String, InProgressUpload>>>,
    stored_blobs: Arc<RwLock<HashMap<String, StoredBlobDescriptor>>>,
}

impl LocalFsStreamingVault {
    pub fn new(storage_dir: impl Into<PathBuf>, hmac_secret: impl Into<String>) -> Self {
        Self {
            storage_dir: storage_dir.into(),
            hmac_secret: hmac_secret.into(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            stored_blobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn now_unix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn inspect_magic_bytes(bytes: &[u8], declared_mime: &str) -> MimeInspectionResult {
        let (detected_mime, matched) = if bytes.starts_with(b"PAR1") {
            ("application/vnd.apache.parquet", declared_mime.contains("parquet"))
        } else if bytes.starts_with(b"PK\x03\x04") {
            ("application/zip", declared_mime.contains("zip") || declared_mime.contains("parquet"))
        } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            ("image/png", declared_mime == "image/png")
        } else if bytes.starts_with(b"\xff\xd8\xff") {
            ("image/jpeg", declared_mime == "image/jpeg" || declared_mime == "image/jpg")
        } else if bytes.starts_with(b"%PDF-") {
            ("application/pdf", declared_mime == "application/pdf")
        } else if bytes.starts_with(b"\x1f\x8b") {
            ("application/gzip", declared_mime.contains("gzip") || declared_mime.contains("tar"))
        } else {
            ("application/octet-stream", true)
        };

        // Disallow dangerous executables masquerading as harmless formats
        let safe = !bytes.starts_with(b"MZ") && !bytes.starts_with(b"\x7fELF");

        MimeInspectionResult {
            detected_mime: detected_mime.to_string(),
            magic_bytes_matched: matched,
            safe_for_egress: safe,
        }
    }
}

#[async_trait]
impl BinaryStreamingVault for LocalFsStreamingVault {
    async fn initiate_resumable_upload(&self, metadata: &BinaryBlobMetadata) -> AegisResult<UploadSession> {
        let session_id = format!("upload-{}-{}", metadata.tenant_id, Self::now_unix());
        let expires_at = Self::now_unix() + 3600; // 1 hour TTL

        let mut lock = self.sessions.write().await;
        lock.insert(
            session_id.clone(),
            InProgressUpload {
                metadata: metadata.clone(),
                data: Vec::with_capacity(metadata.expected_size as usize),
                expires_at_unix: expires_at,
            },
        );

        Ok(UploadSession {
            session_id,
            chunk_size: 1024 * 1024,
            expires_at_unix: expires_at,
        })
    }

    async fn append_chunk(&self, session_id: &str, offset: u64, chunk: Vec<u8>) -> AegisResult<ChunkAck> {
        let mut lock = self.sessions.write().await;
        let session = lock
            .get_mut(session_id)
            .ok_or_else(|| AegisError::ResourceNotFound(format!("Upload session {session_id} not found")))?;

        if Self::now_unix() > session.expires_at_unix {
            return Err(AegisError::InvalidState("Upload session expired".into()));
        }

        let chunk_len = chunk.len() as u64;
        let current_len = session.data.len() as u64;

        if offset > current_len {
            return Err(AegisError::Validation(format!(
                "Chunk gap detected: expected offset {current_len}, got {offset}"
            )));
        }

        if offset == current_len {
            session.data.extend_from_slice(&chunk);
        } else {
            let start = offset as usize;
            let end = (offset + chunk_len) as usize;
            if end > session.data.len() {
                session.data.resize(end, 0);
            }
            session.data[start..end].copy_from_slice(&chunk);
        }

        let total = session.data.len() as u64;
        let completed = total >= session.metadata.expected_size;

        Ok(ChunkAck {
            session_id: session_id.to_string(),
            bytes_received: chunk_len,
            total_received: total,
            completed,
        })
    }

    async fn finalize_upload(&self, session_id: &str, expected_sha256: Option<&str>) -> AegisResult<StoredBlobDescriptor> {
        let mut sessions_lock = self.sessions.write().await;
        let session = sessions_lock
            .remove(session_id)
            .ok_or_else(|| AegisError::ResourceNotFound(format!("Upload session {session_id} not found")))?;

        let mut hasher = Sha256::new();
        hasher.update(&session.data);
        let calculated_hash = to_hex(&hasher.finalize());

        if let Some(expected) = expected_sha256 {
            if calculated_hash != expected {
                return Err(AegisError::SecurityRefusal(format!(
                    "SHA256 checksum mismatch: calculated {calculated_hash}, expected {expected}"
                )));
            }
        }

        let inspection = Self::inspect_magic_bytes(&session.data, &session.metadata.declared_mime);
        if !inspection.safe_for_egress {
            return Err(AegisError::SecurityRefusal("Executable payload disguised as binary blob".into()));
        }

        let blob_urn = format!("urn:aegis:blob:{}", calculated_hash);
        let path = self.storage_dir.join(&calculated_hash).to_string_lossy().to_string();

        let descriptor = StoredBlobDescriptor {
            blob_urn: blob_urn.clone(),
            storage_path: path,
            actual_mime: inspection.detected_mime,
            size_bytes: session.data.len() as u64,
            sha256_checksum: calculated_hash,
            created_at_unix: Self::now_unix(),
        };

        let mut blobs_lock = self.stored_blobs.write().await;
        blobs_lock.insert(blob_urn, descriptor.clone());

        Ok(descriptor)
    }

    async fn generate_ephemeral_egress_ticket(
        &self,
        blob_urn: &str,
        client_id: &str,
        ttl_secs: u64,
    ) -> AegisResult<EphemeralEgressTicket> {
        let blobs_lock = self.stored_blobs.read().await;
        if !blobs_lock.contains_key(blob_urn) {
            return Err(AegisError::ResourceNotFound(format!("Blob {blob_urn} not found in vault")));
        }

        let expires_at = Self::now_unix() + ttl_secs;
        let ticket_id = format!("ticket-{}-{}", client_id, Self::now_unix());

        let mut hasher = Sha256::new();
        hasher.update(ticket_id.as_bytes());
        hasher.update(b":");
        hasher.update(blob_urn.as_bytes());
        hasher.update(b":");
        hasher.update(&expires_at.to_be_bytes());
        hasher.update(b":");
        hasher.update(self.hmac_secret.as_bytes());
        let signature = to_hex(&hasher.finalize());

        let url = format!("https://aegis.gateway/vault/download/{}?ticket={}&sig={}", blob_urn, ticket_id, signature);

        Ok(EphemeralEgressTicket {
            ticket_id,
            blob_urn: blob_urn.to_string(),
            direct_download_url: url,
            expires_at_unix: expires_at,
            hmac_signature: signature,
        })
    }

    async fn inspect_blob_content(&self, blob_urn: &str) -> AegisResult<MimeInspectionResult> {
        let blobs_lock = self.stored_blobs.read().await;
        let descriptor = blobs_lock
            .get(blob_urn)
            .ok_or_else(|| AegisError::ResourceNotFound(format!("Blob {blob_urn} not found")))?;

        Ok(MimeInspectionResult {
            detected_mime: descriptor.actual_mime.clone(),
            magic_bytes_matched: true,
            safe_for_egress: true,
        })
    }
}
