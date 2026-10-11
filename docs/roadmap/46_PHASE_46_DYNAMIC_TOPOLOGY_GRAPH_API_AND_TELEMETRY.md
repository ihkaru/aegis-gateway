# Phase 46: Dynamic Topology Graph API & Telemetry State Aggregator

> **Milestone Tag**: `v1.46.0-topology_graph_api`  
> **Status**: `Planned`  
> **Target Enterprise Gaps & Standards**:
> - Exposes unified graph topology API (`GET /api/v1/topology/graph`) returning DAG nodes (Clients, Ingress, ABAC/DLP Filters, Router, MCP Backends, Integration Channels) and directed edges.
> - Computes live telemetry per node: throughput RPS, P99 latency, circuit breaker state (`CLOSED`, `OPEN`, `HALF_OPEN`), and pending HITL queue depth.
> - Pure-Rust Zero-Mock Aggregation: Binds directly to `AegisGateway` runtime state and `BackendRegistry`.

---

## 1. Problem Statement & Motivation

At high concurrency (20,000+ employees), administrators need holistic visibility into how requests flow from clients through security filters into MCP servers and enterprise channels:
1. Tabular listings obscure dependencies and bottleneck locations.
2. SREs require an authoritative DAG model describing upstream servers and downstream integration channels in a single query.
3. The topology API must supply structured nodes and edges compatible with graph visualization engines (`@xyflow/svelte`).

---

## 2. Technical Architecture & Trait Contracts

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyGraphNode {
    pub id: String,
    pub node_type: String, // "client" | "ingress" | "filter" | "router" | "backend" | "channel"
    pub label: String,
    pub status: String,    // "healthy" | "degraded" | "circuit_open" | "suspended"
    pub metrics: TopologyNodeMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyGraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: Option<String>,
    pub animated: bool,
    pub status: String,    // "active" | "idle" | "tripped"
}

#[async_trait]
pub trait TopologyGraphProvider: Send + Sync {
    async fn get_topology_graph(&self) -> AegisResult<TopologyGraphResponse>;
}
```

### Key Guarantees:
- **Zero Mock Data**: Node statuses reflect real registered backends and live circuit breaker states.
- **Fast Execution**: In-memory synthesis completes in <1ms without blocking active request threads.
