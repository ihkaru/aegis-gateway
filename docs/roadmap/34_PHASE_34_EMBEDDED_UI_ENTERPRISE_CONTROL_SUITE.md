# Phase 34: Embedded Web UI Enterprise Parity & Interactive Control Suite

## Executive Summary
Phase 34 expands the Single-Binary Embedded Admin Web Dashboard (Phase 33) into an enterprise-grade control suite. While Phase 33 established the Svelte 5/Bun foundation, multi-theme architecture, and basic telemetry, Phase 34 bridges the gap with the underlying Rust engine by providing interactive backend topology management, dynamic policy switching, a browser-based MCP tool playground, cryptographic SIEM audit inspection, and SAML/SCIM identity visualizers.

---

## 1. Feature Gap Matrix & Expansion Plan

| Capability | Phase 33 Baseline | Phase 34 Target (Enterprise Parity) |
| :--- | :--- | :--- |
| **Server Listener** | In-memory router only | Dedicated standalone daemon (`aegis-gateway ui`) + Axum HTTP server on `:8485` |
| **Backends Management** | None (CLI only) | Interactive `BackendsTab`: list stdio/sse backends, circuit breaker status, add/remove via UI |
| **Tool Catalog** | None | L0 Purpose (~50 tokens), L1 Signature, and L2 Schema viewer for all registered tools |
| **Policy Tiers** | Static badge | Interactive `PolicyTab`: hot-swap `dev` / `hybrid` / `strict` in 0ms via atomic RCU swap |
| **Tool Playground** | None | In-browser `PlaygroundTab`: execute `tools/call` with JSON editor & DLP diff visualizer |
| **SIEM Audit Trail** | Raw JSON file | Interactive `AuditTab`: SHA-256 cryptographic hash-chain explorer & SOC 2 export |
| **HITL Approvals** | Local state removal | Connected to `/api/v1/hitl/:id/decision` dispatching HMAC-signed approvals to engine |
| **FinOps Actions** | Static gauges | Interactive tenant management: adjust token quotas and manual unfreeze buttons |
| **Telemetry Pipeline** | Static polling | Live Server-Sent Events (SSE) telemetry stream on `/api/v1/telemetry/stream` |

---

## 2. Architectural Pillars & Implementation Roadmap

### Pillar 1: Dedicated Standalone & Concurrent UI Daemon
- **CLI Subcommand**: First-class `aegis-gateway ui --host 0.0.0.0 --port 8485` command.
- **Concurrent Serve Mode**: `aegis-gateway serve` launches both the MCP Wire Data Plane (`:8484`) and the Admin Control Plane (`:8485`).
- **Zero-Downtime Hot-Reload**: Control plane actions update the shared `Arc<RwLock<TopologyConfig>>` without dropping active agent streams.

### Pillar 2: Dynamic Backends & Tool Registry Control (`BackendsTab.svelte`)
- Real-time list of configured backend servers (`stdio`, `sse`, `streamable_http`).
- Visual circuit breaker state indicators (`CLOSED` green, `OPEN` red, `HALF_OPEN` yellow).
- Accordion view of all discovered tools with progressive disclosure tokens.
- Modal dialog to register a new backend server dynamically.

### Pillar 3: Zero-Trust Policy & ABAC Visualizer (`PolicyTab.svelte`)
- One-click tier hot-swap (`dev` -> `hybrid` -> `strict`) with live validation.
- Visual inspection of active OPA payload constraints (e.g., maximum transaction limits, table restrictions).
- Toggle controls for inline DLP masking and Threat Evidence enrichers.

### Pillar 4: Interactive MCP Tool Playground (`PlaygroundTab.svelte`)
- Dropdown selector for available backend servers and their published tools.
- Real-time JSON argument editor with Monaco/Geist Mono formatting.
- Side-by-side diff pane: Raw execution output vs. DLP-redacted output delivered to LLMs.

### Pillar 5: Immutable SIEM Audit Explorer & SOC 2 Reporter (`AuditTab.svelte`)
- Visual timeline of hash-chained audit events (`payload_hash_sha256` and `parent_hash`).
- Cryptographic verification badge showing whether the tamper-evident chain is unbroken.
- Instant "Export SOC 2 Type II Report" button generating verifiable audit compliance JSON.

### Pillar 6: Connected REST API Endpoints in Pure-Rust Core
- `GET /api/v1/backends` & `POST /api/v1/backends`: Query and register backend servers.
- `DELETE /api/v1/backends/:name`: Remove backend server and reap orphaned subprocesses.
- `GET /api/v1/tools`: List all tools across registered backends with token estimates.
- `POST /api/v1/tools/call`: Proxied tool execution endpoint for the playground.
- `POST /api/v1/policy/tier`: Atomic update of runtime policy tier.
- `POST /api/v1/hitl/:id/decision`: Submit cryptographic approval/rejection for suspended tasks.
- `POST /api/v1/finops/unfreeze`: Restore frozen tenant access after budget reconciliation.

---

## 3. Verification & Governance Criteria
- [ ] UI files adhere strictly to Svelte 5 Runes (`$state`, `$derived`, `$props`) with TypeScript.
- [ ] Strict adherence to zero-emoji and anti-slop guidelines with monochrome Lucide icons.
- [ ] All Rust source files and Svelte components remain `<= 350` lines.
- [ ] Zero unsafe code (`#![deny(unsafe_code)]`) and zero mock dependencies in production paths.
- [ ] 100% passing tests via `cargo-nextest run`.
