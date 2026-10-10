# Phase 35: Live Data Plane Control Binding, Zero-Mock UI & Embedded State Engine

## Executive Summary
Phase 35 eliminates all static fixtures, dummy data, and disconnected mock placeholders across the Aegis Gateway Control Plane and Web UI. Rather than introducing a heavy external database (which breaks the single-binary, sub-30MB RAM, zero-dependency model required by enterprise edge proxies), Phase 35 connects the embedded administration server directly to the live pure-Rust runtime engine via Dependency Injection. The Svelte 5 frontend fetches, renders, and mutates real operational state—real discovered MCP backends, live process RSS memory, dynamic HITL approval tickets, cryptographic SIEM audit trails, and multi-tenant token counters.

---

## 1. Architectural Analysis: Does an Enterprise Gateway Need an External Database?

### The Enterprise Gateway Invariant
High-performance API gateways (Envoy, Caddy, Traefik, Kong, Cloudflare Workers) strictly avoid mandatory relational databases (PostgreSQL/MySQL) on their operational data plane:
1. **Stateless Core & Resilience**: Database downtime must never take down tool proxying between AI models and internal services.
2. **Sub-Millisecond Wire Overhead**: Querying an external database on every tool execution adds 5–20ms of network latency. Pure-Rust in-memory state (`Arc<RwLock<...>>`) guarantees sub-0.5ms evaluation.
3. **Pluggable Persistence Architecture**:
   - **Standalone / Local Development**: In-memory state backed by declarative files (`aegis.yaml`), atomic file mutations, and append-only hash-chained audit logs.
   - **Kubernetes Production Clusters**: High-availability distributed state via `RedisStateBackend` cluster driver with Lua script atomicity.

**Conclusion**: Aegis Gateway does NOT require a heavy external database. Instead, the UI must bind directly to the Gateway's live in-memory Control Plane and state engines.

---

## 2. Feature Gaps & Elimination Matrix

| Component | Previous Baseline | Phase 35 Live Enterprise Target |
| :--- | :--- | :--- |
| **Admin Server Wiring** | Isolated `EmbeddedAdminServer` instance | Constructor accepts `Arc<AegisGateway>` & `Arc<BackendRegistry>` |
| **System Telemetry** | Hardcoded JSON constants (1420 rps) | Live `ru_maxrss` OS memory, real active backend count, uptime |
| **Backends & Tools** | None in UI | Live discovery from `BackendRegistry` with tool descriptions |
| **HITL Approvals** | Hardcoded Svelte state array | Live queue from `ActionApprovalGate`; real POST decisions |
| **DLP Security Stream** | Hardcoded event list | Live events streamed from `AuditSink` and `DlpPipeline` |
| **Tenant FinOps** | Hardcoded tenant records | Live token quotas from `QuotaEngine` with real unfreeze action |
| **UI Data Fetching** | Static arrays in component source | Reactive `onMount()` + `fetch()` with loading & empty states |
| **Zero-Mock Audit** | Backend `src/` scanning only | Full-stack AST & regex scan blocking hardcoded mock data in UI |

---

## 3. Implementation Blueprint

### Pillar 1: Dependency Injection in `EmbeddedAdminServer`
`EmbeddedAdminServer` receives:
- `gateway: Option<Arc<AegisGateway>>`
- `registry: Option<Arc<dyn BackendRegistry>>`
- `config: Option<Arc<RwLock<AegisTopologyConfig>>>`

When bound, `/api/v1/*` endpoints inspect and mutate the actual living instances:
- `/api/v1/overview`: Real system RSS via `libc::getrusage` / `/proc/self/statm`, backend count, active tools.
- `/api/v1/backends`: Real backends from registry.
- `/api/v1/hitl/queue` & `/api/v1/hitl/:id/decision`: Real suspended tickets and HMAC verification.
- `/api/v1/finops`: Real tenant consumption and freeze state.

### Pillar 2: Dynamic Svelte 5 Components with Zero-Mock Enforcement
All `.svelte` components in `ui/src/lib/` replace static fixtures with reactive state:
```typescript
let loading = $state(true);
let items = $state<Item[]>([]);

onMount(async () => {
  try {
    const res = await fetch('/api/v1/endpoint');
    if (res.ok) items = await res.json();
  } finally {
    loading = false;
  }
});
```

### Pillar 3: Full-Stack Zero-Mock Governance Scanner
Upgrade `scripts/audit_mock_detection.sh` to fail builds if:
- Static mock fixture arrays (e.g. `const recentCalls = [...]`, `let pendingTasks = [...]`) exist in `ui/src/`.
- UI components lack real `fetch()` calls to backend endpoints.

---

## 4. Verification & Criteria
- [x] Full-stack zero-mock audit passes across both Rust and Svelte/TypeScript sources.
- [x] `EmbeddedAdminServer` serves live telemetry dynamically measured from the process runtime.
- [x] Svelte 5 frontend displays real backend services, active tools, and real quota state.
- [x] All 151+ tests pass in parallel via `cargo-nextest run`.
- [x] All source files remain `<= 350` lines.
