// SPDX-License-Identifier: MIT

use aegis_gateway::cluster::DistributedPubSubClusterSync;
use aegis_gateway::core::cluster_sync::ClusterSyncEngine;
use aegis_gateway::core::error::AegisError;

#[tokio::test]
async fn test_multi_node_broadcast_and_ingestion() {
    let node_a = DistributedPubSubClusterSync::new("aegis-pod-01", 1);
    let node_b = DistributedPubSubClusterSync::new("aegis-pod-02", 1);

    // 1. Node A publishes an update
    let payload = r#"{"tenant_id":"global-fintech","rate_limit":500}"#;
    let msg = node_a
        .broadcast_update("policy_updated", payload)
        .await
        .expect("broadcast failed");
    assert_eq!(msg.version, 2);
    assert_eq!(msg.source_node_id, "aegis-pod-01");
    assert!(!msg.checksum_sha256.is_empty());

    // 2. Node B ingests the update
    let receipt = node_b.ingest_peer_update(&msg).await.expect("ingest failed");
    assert!(receipt.applied);
    assert_eq!(receipt.current_version, 2);

    // 3. Cluster node membership updated
    let nodes = node_b.get_cluster_nodes().await.expect("nodes query failed");
    assert!(nodes.contains(&"aegis-pod-01".to_string()));
    assert!(nodes.contains(&"aegis-pod-02".to_string()));
}

#[tokio::test]
async fn test_checksum_mismatch_fails_closed() {
    let node_a = DistributedPubSubClusterSync::new("aegis-pod-01", 1);
    let node_b = DistributedPubSubClusterSync::new("aegis-pod-02", 1);

    let msg = node_a
        .broadcast_update("backend_registered", r#"{"name":"test"}"#)
        .await
        .expect("broadcast failed");

    // Tamper with payload
    let mut tampered_msg = msg.clone();
    tampered_msg.payload_json = r#"{"name":"tampered_payload_injected"}"#.into();

    let res = node_b.ingest_peer_update(&tampered_msg).await;
    match res {
        Err(AegisError::SecurityRefusal(err)) => {
            assert!(err.contains("checksum mismatch"));
        }
        other => panic!("Expected SecurityRefusal error, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_anti_regression_and_idempotence() {
    let node_b = DistributedPubSubClusterSync::new("aegis-pod-02", 5);

    let mut stale_msg = DistributedPubSubClusterSync::new("aegis-pod-01", 2)
        .broadcast_update("sync", "{}")
        .await
        .expect("broadcast failed");
    // Force version 3 (older than node_b's current version 5)
    stale_msg.version = 3;

    let receipt = node_b.ingest_peer_update(&stale_msg).await.expect("ingest failed");
    assert!(!receipt.applied);
    assert_eq!(receipt.current_version, 5);
}
