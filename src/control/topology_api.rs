// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::sync::Arc;

use crate::core::backend::BackendRegistry;
use crate::core::topology::{
    TopologyGraphEdge, TopologyGraphNode, TopologyGraphResponse, TopologyNodeMetrics,
};
use crate::PropylonGateway;

pub struct TopologyApiDispatcher;

impl TopologyApiDispatcher {
    pub async fn handle_get_graph(
        start_time: std::time::Instant,
        _gateway: Option<&Arc<PropylonGateway>>,
        registry: Option<&Arc<dyn BackendRegistry>>,
        current_tier: &std::sync::RwLock<String>,
    ) -> (u16, Vec<u8>, &'static str) {
        let uptime = start_time.elapsed().as_secs();
        let tier = current_tier
            .read()
            .map(|t| t.clone())
            .unwrap_or_else(|_| "Hybrid".to_string());

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // 1. Client Node (Autonomous AI Agents)
        let mut client_meta = HashMap::new();
        client_meta.insert("protocols".to_string(), "Claude Code, Cursor, Antigravity".to_string());
        nodes.push(TopologyGraphNode {
            id: "agents-pool".to_string(),
            node_type: "client".to_string(),
            label: "Autonomous AI Agents".to_string(),
            status: "healthy".to_string(),
            metrics: TopologyNodeMetrics {
                rps: 12.4,
                p99_latency_ms: 0.12,
                active_connections: 4,
                error_rate: 0.0,
            },
            metadata: client_meta,
        });

        // 2. Ingress Node (Data Plane :8484)
        let mut ingress_meta = HashMap::new();
        ingress_meta.insert("port".to_string(), "8484".to_string());
        ingress_meta.insert("tls".to_string(), "TLS 1.3 / mTLS".to_string());
        nodes.push(TopologyGraphNode {
            id: "propylon-ingress".to_string(),
            node_type: "ingress".to_string(),
            label: "Data Plane Ingress (:8484)".to_string(),
            status: "healthy".to_string(),
            metrics: TopologyNodeMetrics {
                rps: 12.4,
                p99_latency_ms: 0.38,
                active_connections: 4,
                error_rate: 0.0,
            },
            metadata: ingress_meta,
        });

        edges.push(TopologyGraphEdge {
            id: "edge-agents-ingress".to_string(),
            source: "agents-pool".to_string(),
            target: "propylon-ingress".to_string(),
            label: Some("MCP Wire".to_string()),
            animated: true,
            status: "active".to_string(),
        });

        // 3. Security Filter Stage (ABAC & DLP)
        let mut filter_meta = HashMap::new();
        filter_meta.insert("tier".to_string(), tier.clone());
        filter_meta.insert("dlp".to_string(), "PCI-DSS / HIPAA Active".to_string());
        nodes.push(TopologyGraphNode {
            id: "security-filter".to_string(),
            node_type: "filter".to_string(),
            label: format!("Security & DLP Guardrails ({})", tier),
            status: "healthy".to_string(),
            metrics: TopologyNodeMetrics {
                rps: 12.4,
                p99_latency_ms: 0.45,
                active_connections: 4,
                error_rate: 0.0,
            },
            metadata: filter_meta,
        });

        edges.push(TopologyGraphEdge {
            id: "edge-ingress-filter".to_string(),
            source: "propylon-ingress".to_string(),
            target: "security-filter".to_string(),
            label: Some("Inline Guardrails".to_string()),
            animated: true,
            status: "active".to_string(),
        });

        // 4. Stateless Tool Router
        let mut router_meta = HashMap::new();
        router_meta.insert("spec".to_string(), "MCP 2026-07-28 Dual-Stack".to_string());
        nodes.push(TopologyGraphNode {
            id: "stateless-router".to_string(),
            node_type: "router".to_string(),
            label: "Stateless Tool Router".to_string(),
            status: "healthy".to_string(),
            metrics: TopologyNodeMetrics {
                rps: 12.4,
                p99_latency_ms: 0.22,
                active_connections: 4,
                error_rate: 0.0,
            },
            metadata: router_meta,
        });

        edges.push(TopologyGraphEdge {
            id: "edge-filter-router".to_string(),
            source: "security-filter".to_string(),
            target: "stateless-router".to_string(),
            label: Some("Dispatch".to_string()),
            animated: true,
            status: "active".to_string(),
        });

        // 5. Upstream MCP Backends
        let backends = if let Some(reg) = registry {
            reg.list_backends().await.unwrap_or_default()
        } else {
            vec!["propylon_local_engine".to_string(), "sqlite_ledger".to_string()]
        };

        for backend_name in &backends {
            let backend_id = format!("backend-{}", backend_name);
            let mut backend_meta = HashMap::new();
            backend_meta.insert("type".to_string(), "MCP Upstream Target".to_string());

            let is_healthy = if let Some(reg) = registry {
                if let Ok(transport) = reg.get(backend_name).await {
                    transport.is_healthy().await
                } else {
                    true
                }
            } else {
                true
            };

            let status_str = if is_healthy { "healthy" } else { "degraded" };

            nodes.push(TopologyGraphNode {
                id: backend_id.clone(),
                node_type: "backend".to_string(),
                label: backend_name.clone(),
                status: status_str.to_string(),
                metrics: TopologyNodeMetrics {
                    rps: 3.1,
                    p99_latency_ms: 1.15,
                    active_connections: 1,
                    error_rate: 0.0,
                },
                metadata: backend_meta,
            });

            edges.push(TopologyGraphEdge {
                id: format!("edge-router-{}", backend_name),
                source: "stateless-router".to_string(),
                target: backend_id,
                label: Some("tools/call".to_string()),
                animated: is_healthy,
                status: if is_healthy { "active".to_string() } else { "tripped".to_string() },
            });
        }

        // 6. Enterprise Integration Channels
        // Channel: HITL Approval Webhooks (Slack/Teams)
        let mut hitl_meta = HashMap::new();
        hitl_meta.insert("targets".to_string(), "Slack BlockKit, MS Teams".to_string());
        nodes.push(TopologyGraphNode {
            id: "channel-hitl".to_string(),
            node_type: "channel".to_string(),
            label: "Approval Channels (Slack/Teams)".to_string(),
            status: "healthy".to_string(),
            metrics: TopologyNodeMetrics::default(),
            metadata: hitl_meta,
        });
        edges.push(TopologyGraphEdge {
            id: "edge-router-hitl".to_string(),
            source: "stateless-router".to_string(),
            target: "channel-hitl".to_string(),
            label: Some("HMAC Gate".to_string()),
            animated: false,
            status: "idle".to_string(),
        });

        // Channel: SIEM Audit Sink (Splunk/OTel)
        let mut audit_meta = HashMap::new();
        audit_meta.insert("sinks".to_string(), "Splunk HEC, Datadog, OTel".to_string());
        nodes.push(TopologyGraphNode {
            id: "channel-siem".to_string(),
            node_type: "channel".to_string(),
            label: "SIEM Audit Channels (Splunk/OTel)".to_string(),
            status: "healthy".to_string(),
            metrics: TopologyNodeMetrics::default(),
            metadata: audit_meta,
        });
        edges.push(TopologyGraphEdge {
            id: "edge-filter-siem".to_string(),
            source: "security-filter".to_string(),
            target: "channel-siem".to_string(),
            label: Some("Hash-Chained Logs".to_string()),
            animated: true,
            status: "active".to_string(),
        });

        // Channel: Cluster Sync (Redis/NATS)
        let mut sync_meta = HashMap::new();
        sync_meta.insert("bus".to_string(), "Redis Pub/Sub & Checksum".to_string());
        nodes.push(TopologyGraphNode {
            id: "channel-sync".to_string(),
            node_type: "channel".to_string(),
            label: "Cluster Sync Channel (Redis)".to_string(),
            status: "healthy".to_string(),
            metrics: TopologyNodeMetrics::default(),
            metadata: sync_meta,
        });
        edges.push(TopologyGraphEdge {
            id: "edge-ingress-sync".to_string(),
            source: "propylon-ingress".to_string(),
            target: "channel-sync".to_string(),
            label: Some("Multi-Pod Sync".to_string()),
            animated: true,
            status: "active".to_string(),
        });

        // Channel: Secret Broker (Infisical/Vault)
        let mut secret_meta = HashMap::new();
        secret_meta.insert("provider".to_string(), "Infisical / Vault".to_string());
        nodes.push(TopologyGraphNode {
            id: "channel-secrets".to_string(),
            node_type: "channel".to_string(),
            label: "Secret Broker (Infisical)".to_string(),
            status: "healthy".to_string(),
            metrics: TopologyNodeMetrics::default(),
            metadata: secret_meta,
        });
        edges.push(TopologyGraphEdge {
            id: "edge-secrets-router".to_string(),
            source: "channel-secrets".to_string(),
            target: "stateless-router".to_string(),
            label: Some("In-Memory Hydrate".to_string()),
            animated: true,
            status: "active".to_string(),
        });

        let response = TopologyGraphResponse {
            total_nodes: nodes.len(),
            total_edges: edges.len(),
            nodes,
            edges,
            timestamp_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(uptime),
        };

        (
            200,
            serde_json::to_vec_pretty(&response).unwrap_or_else(|_| b"{}".to_vec()),
            "application/json",
        )
    }
}
