---
name: zero-downtime-control-plane-auditor
description: Continuous architectural governance and static analysis skill verifying Zero-Downtime Hot-Reload, atomic state swapping, Dynamic Control Plane integrity, and distributed cluster synchronization invariants without service restarts.
---

# Zero-Downtime & Dynamic Control Plane Auditor Skill

This skill enforces strict cloud-native operational invariants across the Aegis Gateway codebase to ensure that the core value proposition—**100% Zero-Downtime Dynamic Runtime Control Plane**—never suffers architectural regression or accidental degradation into static restart-required configurations.

---

## 🛡️ Core Architectural Invariants

### 1. Zero-Restart Policy
- **No Service Restarts for Configuration Changes**: Tool addition/removal, policy tier adjustments (`dev` <-> `hybrid` <-> `strict`), rate limit quota tuning, secret rotation, and admission rules MUST apply immediately at runtime with 0ms downtime.
- **Connection Preservation**: Active Server-Sent Events (SSE) and Streamable HTTP agent streams must never be terminated by operational reconfiguration.
- **Task Durability**: Suspended tasks awaiting Human-in-the-Loop (HITL) approval must remain durable across config updates.

### 2. Atomic In-Memory State Swapping
- All runtime state transitions must use lock-free or read-optimized atomic pointer swapping (`RwLock`, `Arc<RwLock<T>>`, or atomic sequence counters).
- Write locks must be held only for microsecond dictionary insertions or pointer exchanges, never across network I/O or disk operations.

### 3. Fail-Closed Validation Guardrails
- In-memory updates must be validated *prior* to application.
- Invalid transport URLs, unsupported policy tiers, or corrupt payloads must fail closed atomically, leaving active running state 100% uncorrupted.

### 4. Anti-Drift Distributed Cluster Sync
- Multi-node deployments must synchronize dynamic updates across nodes via the Pub/Sub bus.
- Every sync message must enforce rolling SHA-256 checksum attestation and anti-regression version numbers.

---

## 🚫 Detected Anti-Patterns (Fails Audit)

1. **Static Frozen State**: Using `lazy_static!` or `OnceLock` for values that should be runtime-configurable.
2. **Process Termination Traps**: Introducing `std::process::exit` or hard panic calls within dynamic runtime handlers.
3. **Blocking I/O Inside State Locks**: Awaiting asynchronous socket or disk calls while holding `RwLock::write()`.
4. **Unsynchronized Node Drift**: Mutating local node state without publishing to the `ClusterSyncEngine` bus.

---

## 🔍 Automated Audit Execution

To run the full automated zero-downtime integrity audit:

```bash
bash .agents/skills/zero-downtime-control-plane-auditor/scripts/audit_zero_downtime.sh
```
