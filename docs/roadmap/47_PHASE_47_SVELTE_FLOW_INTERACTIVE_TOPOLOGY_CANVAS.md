# Phase 47: Svelte Flow (@xyflow/svelte) Interactive Topology Canvas

> **Milestone Tag**: `v1.47.0-svelte_flow_topology_canvas`  
> **Status**: `Planned`  
> **Target Enterprise Gaps & Standards**:
> - Integrates `@xyflow/svelte` into Embedded Admin Web UI (`ui/`).
> - Dedicated visual **Topology Canvas** tab replacing static representations with an interactive DAG.
> - Custom Node Renderers matching shadcn-svelte design system and supporting 4 enterprise themes (`Zinc`, `Slate`, `Neutral`, `OLED`).
> - Interactive Pan, Zoom, FitView, MiniMap, and reactive edge animations.

---

## 1. Problem Statement & Motivation

Enterprise administrators managing 20,000+ users require immediate visual intuition of gateway architecture and traffic routing:
1. Static tables require manual mental mapping to trace how a request navigates through Ingress, DLP, Router, and Backends.
2. Svelte Flow (`@xyflow/svelte`) provides industry-standard node graph visualization with full reactivity (Svelte 5 Runes) and smooth WebGL/SVG rendering.
3. The visual canvas must preserve Propylon Gateway's anti-slop aesthetic, Lucide SVG iconography, and zero-emoji policy.

---

## 2. Technical Architecture & Components

```text
ui/src/lib/
├── TopologyTab.svelte               (Main Svelte Flow Canvas Controller)
├── topology/
│   ├── ClientNode.svelte            (AI Agent Sessions Node)
│   ├── IngressNode.svelte           (TLS 1.3/mTLS Port :8484 Node)
│   ├── FilterNode.svelte            (ABAC & DLP Firewall Stage Node)
│   ├── RouterNode.svelte            (Stateless Tool Router Node)
│   ├── BackendNode.svelte           (MCP Upstream Subprocess / SSE Node)
│   ├── ChannelNode.svelte           (Slack/Teams/Redis/Splunk Channel Node)
│   └── topologyLayout.ts            (Topological Layering & Coordinate Solver)
```

### Key Guarantees:
- **Zero Mock Binding**: Polling `GET /api/v1/topology/graph` every 2000ms with real-time updates.
- **Bundle Efficiency**: Built cleanly with Bun; static assets bundled into binary via `rust-embed` without server runtime overhead.
- **Strict File Limit**: Every Svelte and TypeScript file strictly adheres to `wc -l <= 350`.
