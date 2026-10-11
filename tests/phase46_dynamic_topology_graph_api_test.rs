use std::sync::RwLock;
use std::time::Instant;

use propylon::control::web_api::WebApiDispatcher;
use propylon::core::topology::TopologyGraphResponse;

#[tokio::test]
async fn test_phase46_topology_graph_api_endpoint() {
    let start_time = Instant::now();
    let current_tier = RwLock::new("Hybrid".to_string());
    let recent_invocations = RwLock::new(Vec::new());
    let recent_dlp_events = RwLock::new(Vec::new());

    let (status, body_bytes, content_type) = WebApiDispatcher::dispatch(
        "GET",
        "/api/v1/topology/graph",
        &[],
        start_time,
        None,
        None,
        &recent_invocations,
        &recent_dlp_events,
        &current_tier,
    )
    .await;

    assert_eq!(status, 200);
    assert_eq!(content_type, "application/json");

    let response: TopologyGraphResponse =
        serde_json::from_slice(&body_bytes).expect("Failed to parse TopologyGraphResponse");

    assert!(response.total_nodes >= 6, "Expected at least 6 topology nodes");
    assert!(response.total_edges >= 5, "Expected at least 5 topology edges");

    // Verify key nodes exist
    let node_types: Vec<_> = response.nodes.iter().map(|n| n.node_type.as_str()).collect();
    assert!(node_types.contains(&"client"), "Missing client node");
    assert!(node_types.contains(&"ingress"), "Missing ingress node");
    assert!(node_types.contains(&"filter"), "Missing security filter node");
    assert!(node_types.contains(&"router"), "Missing tool router node");
    assert!(node_types.contains(&"backend"), "Missing backend nodes");
    assert!(node_types.contains(&"channel"), "Missing integration channel nodes");

    // Verify directed edges connect the flow
    let edge_ids: Vec<_> = response.edges.iter().map(|e| e.id.as_str()).collect();
    assert!(edge_ids.contains(&"edge-agents-ingress"));
    assert!(edge_ids.contains(&"edge-ingress-filter"));
    assert!(edge_ids.contains(&"edge-filter-router"));

    // Verify channel edges
    assert!(edge_ids.contains(&"edge-router-hitl"));
    assert!(edge_ids.contains(&"edge-filter-siem"));
    assert!(edge_ids.contains(&"edge-ingress-sync"));
}

#[tokio::test]
async fn test_phase46_topology_graph_reflects_policy_tier_swap() {
    let start_time = Instant::now();
    let current_tier = RwLock::new("Strict".to_string());
    let recent_invocations = RwLock::new(Vec::new());
    let recent_dlp_events = RwLock::new(Vec::new());

    let (status, body_bytes, _) = WebApiDispatcher::dispatch(
        "GET",
        "/api/v1/topology/graph",
        &[],
        start_time,
        None,
        None,
        &recent_invocations,
        &recent_dlp_events,
        &current_tier,
    )
    .await;

    assert_eq!(status, 200);
    let response: TopologyGraphResponse =
        serde_json::from_slice(&body_bytes).expect("Failed to parse response");

    let filter_node = response
        .nodes
        .iter()
        .find(|n| n.id == "security-filter")
        .expect("security-filter node must exist");

    assert!(
        filter_node.label.contains("Strict"),
        "Filter node label must reflect current active policy tier"
    );
    assert_eq!(
        filter_node.metadata.get("tier").map(|s| s.as_str()),
        Some("Strict")
    );
}
