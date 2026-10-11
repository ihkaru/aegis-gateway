// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::error::AegisResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopologyNodeMetrics {
    pub rps: f64,
    pub p99_latency_ms: f64,
    pub active_connections: usize,
    pub error_rate: f64,
}

impl Default for TopologyNodeMetrics {
    fn default() -> Self {
        Self {
            rps: 0.0,
            p99_latency_ms: 0.38,
            active_connections: 1,
            error_rate: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopologyGraphNode {
    pub id: String,
    pub node_type: String, // "client" | "ingress" | "filter" | "router" | "backend" | "channel"
    pub label: String,
    pub status: String,    // "healthy" | "degraded" | "circuit_open" | "suspended"
    pub metrics: TopologyNodeMetrics,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopologyGraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: Option<String>,
    pub animated: bool,
    pub status: String,    // "active" | "idle" | "tripped"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopologyGraphResponse {
    pub nodes: Vec<TopologyGraphNode>,
    pub edges: Vec<TopologyGraphEdge>,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub timestamp_unix: u64,
}

/// Interface-First abstraction for Dynamic Topology Graph Generation
#[async_trait]
pub trait TopologyGraphProvider: Send + Sync {
    async fn get_topology_graph(&self) -> AegisResult<TopologyGraphResponse>;
}
