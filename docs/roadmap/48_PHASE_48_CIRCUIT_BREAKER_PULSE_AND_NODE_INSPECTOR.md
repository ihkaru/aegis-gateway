# Phase 48: Live Circuit Breaker Pulse & Interactive Node Inspector Drawer

> **Milestone Tag**: `v1.48.0-circuit_breaker_pulse_inspector`  
> **Status**: `Planned`  
> **Target Enterprise Gaps & Standards**:
> - Visual pulse animations indicating live Circuit Breaker status (`Green = Healthy`, `Amber = Half-Open`, `Red = Tripped/Open`).
> - Interactive Slide-Out Node Inspector Drawer when clicking any node in the Svelte Flow canvas.
> - Diagnostic action triggers: Instant Circuit Breaker Reset, Tool Inventory Inspection, and Channel Attestation logs.

---

## 1. Problem Statement & Motivation

Visual graphs become actionable tools only when operators can diagnose issues and take remediation steps directly from the canvas:
1. When a downstream Python MCP subprocess or SaaS API fails, the SRE must see the node flash RED immediately.
2. Clicking the degraded node must open an Inspector Drawer with exact error traces and a one-click "Reset Circuit Breaker" trigger.
3. Edge lines must visually pulse to indicate real traffic throughput volume.

---

## 2. Technical Architecture & Component Flow

```text
User Click (Node) ──> SelectedNode Store (Svelte 5 Rune)
                            │
                            ▼
              NodeInspectorDrawer.svelte
              ├── Diagnostic Status Badge
              ├── P99 Latency & Error Rate Gauge
              ├── Registered Tools List (with L0 purpose)
              └── Remediation Actions:
                  ├── POST /api/v1/backends/:name/circuit/reset
                  └── POST /api/v1/hitl/flush-queue
```

### Key Guarantees:
- **Instant MTTD (Mean Time to Detect)**: Visual alert within 1 polling interval of circuit trip.
- **Direct Remediation**: Immediate recovery actions without navigating away from the topology view.
- **Zero Mock Policy**: Action triggers execute real REST endpoints against the active control plane.
