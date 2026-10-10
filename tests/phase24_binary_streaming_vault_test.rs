// SPDX-License-Identifier: MIT

use aegis_gateway::core::error::AegisError;
use aegis_gateway::core::vault::{BinaryBlobMetadata, BinaryStreamingVault};
use aegis_gateway::vault::LocalFsStreamingVault;

#[tokio::test]
async fn test_resumable_chunk_upload_and_sha256_verification() {
    let vault = LocalFsStreamingVault::new("/tmp/aegis_vault_test", "super_secret_hmac_key");

    let parquet_data = b"PAR1fake_parquet_table_data_stream_bytes_herePAR1";
    let metadata = BinaryBlobMetadata {
        tenant_id: "tenant-analytics-01".into(),
        file_name: "export_parquet.parquet".into(),
        declared_mime: "application/vnd.apache.parquet".into(),
        expected_size: parquet_data.len() as u64,
        sha256_checksum: None,
    };

    // 1. Initiate upload session
    let session = vault.initiate_resumable_upload(&metadata).await.expect("initiate session failed");
    assert!(session.session_id.starts_with("upload-tenant-analytics-01-"));

    // 2. Append chunks (in two parts)
    let mid = parquet_data.len() / 2;
    let chunk1 = parquet_data[..mid].to_vec();
    let chunk2 = parquet_data[mid..].to_vec();

    let ack1 = vault.append_chunk(&session.session_id, 0, chunk1).await.expect("chunk 1 failed");
    assert_eq!(ack1.bytes_received, mid as u64);
    assert!(!ack1.completed);

    let ack2 = vault.append_chunk(&session.session_id, mid as u64, chunk2).await.expect("chunk 2 failed");
    assert_eq!(ack2.total_received, parquet_data.len() as u64);
    assert!(ack2.completed);

    // 3. Finalize upload
    let descriptor = vault.finalize_upload(&session.session_id, None).await.expect("finalize failed");
    assert_eq!(descriptor.size_bytes, parquet_data.len() as u64);
    assert!(descriptor.blob_urn.starts_with("urn:aegis:blob:"));
    assert_eq!(descriptor.actual_mime, "application/vnd.apache.parquet");

    // 4. Generate ephemeral egress ticket
    let ticket = vault
        .generate_ephemeral_egress_ticket(&descriptor.blob_urn, "client-ide-01", 300)
        .await
        .expect("ticket generation failed");
    assert!(ticket.direct_download_url.contains(&descriptor.blob_urn));
    assert!(!ticket.hmac_signature.is_empty());
}

#[tokio::test]
async fn test_magic_byte_inspection_detects_executable_spoofing() {
    let vault = LocalFsStreamingVault::new("/tmp/aegis_vault_test", "super_secret_hmac_key");

    // MZ header simulates executable payload disguised as PDF
    let malicious_data = b"MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00\xff\xff";
    let metadata = BinaryBlobMetadata {
        tenant_id: "tenant-untrusted".into(),
        file_name: "invoice.pdf".into(),
        declared_mime: "application/pdf".into(),
        expected_size: malicious_data.len() as u64,
        sha256_checksum: None,
    };

    let session = vault.initiate_resumable_upload(&metadata).await.expect("session initiate failed");
    vault
        .append_chunk(&session.session_id, 0, malicious_data.to_vec())
        .await
        .expect("append chunk failed");

    // Finalize must reject with SecurityRefusal
    let res = vault.finalize_upload(&session.session_id, None).await;
    match res {
        Err(AegisError::SecurityRefusal(msg)) => {
            assert!(msg.contains("Executable payload disguised as binary blob"));
        }
        other => panic!("Expected SecurityRefusal error, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_checksum_mismatch_fails_closed() {
    let vault = LocalFsStreamingVault::new("/tmp/aegis_vault_test", "super_secret_hmac_key");

    let payload = b"legitimate_data_blob_content";
    let metadata = BinaryBlobMetadata {
        tenant_id: "tenant-fintech".into(),
        file_name: "payload.bin".into(),
        declared_mime: "application/octet-stream".into(),
        expected_size: payload.len() as u64,
        sha256_checksum: Some("0000000000000000000000000000000000000000000000000000000000000000".into()),
    };

    let session = vault.initiate_resumable_upload(&metadata).await.expect("session initiate failed");
    vault.append_chunk(&session.session_id, 0, payload.to_vec()).await.expect("append failed");

    let res = vault
        .finalize_upload(&session.session_id, Some("0000000000000000000000000000000000000000000000000000000000000000"))
        .await;

    match res {
        Err(AegisError::SecurityRefusal(msg)) => {
            assert!(msg.contains("SHA256 checksum mismatch"));
        }
        other => panic!("Expected SecurityRefusal due to checksum mismatch, got: {other:?}"),
    }
}
