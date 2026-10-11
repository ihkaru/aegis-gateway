---
name: enterprise-integration-patterns
description: Standardized Enterprise Integration Patterns (EIP - Hohpe & Woolf) enforcing Channel Adapters, Message Endpoints, Pipes & Filters, Control Bus, and Zero Direct Vendor Coupling in Aegis Gateway.
---

# Enterprise Integration Patterns (EIP) Skill

This skill governs the integration architecture of Aegis Gateway based on the canonical patterns from Gregor Hohpe and Bobby Woolf (*Enterprise Integration Patterns*, 2003).

## 1. Core Architectural Separation

In Aegis Gateway, outbound connections are partitioned into two strictly separated planes:

| Dimension | MCP Tool Execution Plane | Enterprise Integration Channels |
| :--- | :--- | :--- |
| **Protocol** | Model Context Protocol JSON-RPC 2.0 | Vendor-specific wire formats & SDKs |
| **Invocability** | Directly invoked by LLM AI Agents (`tools/call`) | Invoked internally by Gateway subsystems |
| **Examples** | Stdio subprocesses, Remote SSE/HTTP servers | Slack, MS Teams, Redis, NATS, Splunk, Vault |
| **Contract Location** | `src/core/backend.rs` (`BackendTransport`) | `src/core/` Channel traits (`ApprovalChannel`, etc.) |

## 2. The EIP Pattern Catalog in Aegis Gateway

### Pattern 1: Channel Adapter & Message Endpoint
- **Rule**: Domain and orchestrator logic must NEVER depend directly on third-party vendor SDKs or raw HTTP endpoints.
- **Implementation**:
  - **Message Endpoint**: Defined as an asynchronous trait in `src/core/` (e.g., `ApprovalNotificationDispatcher`, `AuditSink`, `ClusterSyncEngine`, `SecretStore`).
  - **Channel Adapter**: Concrete struct in `src/policy/`, `src/audit/`, or `src/cluster/` that implements the trait and encapsulates formatting, retries, and HTTP/socket transports.

### Pattern 2: Pipes and Filters
- **Rule**: Data plane request processing must proceed through independent, single-responsibility filter stages.
- **Lifecycle**:
  `Ingress (TLS 1.3/mTLS)` -> `Auth (OIDC/SAML)` -> `ABAC (Policy)` -> `DLP (PCI/HIPAA)` -> `Router (MCP Dispatch)` -> `Execution`.

### Pattern 3: Content-Based Router
- **Rule**: Incoming MCP tool requests are inspected via wire headers (`Mcp-Method`, `Mcp-Name`) and routed to the corresponding tool backend without sticky session dependencies.

### Pattern 4: Control Bus
- **Rule**: Administrative management traffic MUST be strictly isolated from user request wire traffic.
- **Implementation**:
  - Data Plane operates on port `:8484` with sub-millisecond execution loops.
  - Control Plane operates on port `:8485` and cluster sync buses (Redis/NATS), performing atomic in-memory state swaps without restarting data plane streams.

### Pattern 5: Dead Letter Channel & Suspended HITL Gate
- **Rule**: High-risk or destructive actions are not rejected silently; they are suspended into a durable queue, dispatching attestation cards to approval channels with HMAC signatures.

## 3. Static Analysis & Anti-Bypass Enforcement

Automated static analysis (`scripts/audit_callgraph_integrity.sh`) verifies the following rules using `code-graph-mcp`:
1. Every Channel trait defined in `src/core/` must have at least one verified live Channel Adapter implementation (`code-graph refs <Trait> --relation implements`).
2. Core orchestrators (`src/lib.rs`, `src/main.rs`, `src/policy/action_gate.rs`) must contain zero direct vendor endpoints or raw un-abstracted clients.
3. Any attempt to introduce direct vendor coupling fails the automated gauntlet.
