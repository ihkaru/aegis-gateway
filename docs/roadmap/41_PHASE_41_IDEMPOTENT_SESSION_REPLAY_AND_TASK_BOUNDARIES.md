# Phase 41: Idempotent Session Replay & Secure Task Boundaries

> **Milestone Tag**: `v1.41.0-idempotent_session_replay_and_task_boundaries`  
> **Status**: `In Planning`  
> **Target Gaps & Standards**:
> - Resolves `MikkoParkkola/mcp-gateway#2505` (Session-bound idempotency replay tokens)
> - Resolves `MikkoParkkola/mcp-gateway#2524` (Refusal key correlation on pre-auth / empty sessions)
> - Resolves `MikkoParkkola/mcp-gateway#2493` (Era re-probe slot eviction stale answer overwrite prevention)
> - Standard Compliance: RFC 9728, NIST SP 800-218 (SSDF), SOC 2 Type II Non-Repudiation

---

## 1. Problem Statement & Motivation

Community analysis of reference Rust implementations identified critical concurrency and state boundary vulnerabilities:

1. **Cross-Session Idempotency Replay Hijacking (`MikkoParkkola/mcp-gateway#2505`)**:
   - In upstream gateways, idempotency keys were cached globally across all connections without binding to the originating session or tenant. A malicious agent in Session B presenting an idempotency key observed in Session A could retrieve the cached tool response from Session A, bypassing authorization boundaries.
2. **Audit Correlation Drop on Empty Sessions (`MikkoParkkola/mcp-gateway#2524`)**:
   - When a connection was refused at the gateway ingress boundary (e.g., protocol version mismatch, malformed JSON-RPC header, invalid mTLS cert), the audit subsystem attempted to correlate the event with an active session. Because no session had been created, the correlation key was omitted, creating orphan audit logs that violated SOC 2 non-repudiation chain requirements.
3. **Stale Execution Answer Overwrite in Evicted Slots (`MikkoParkkola/mcp-gateway#2493`)**:
   - Under memory pressure, LRU slot evictions recycle task slots. If a slow upstream subprocess finally responds after its slot has been evicted and reassigned to a new user query, the late answer overwrote the new user's slot data, corrupting response payloads.

---

## 2. Technical Architecture & Trait Contracts

Propylon Gateway enforces strict concurrency fences in `src/state/` and `src/audit/`:

### A. Session-Bound Idempotency Scope (`src/state/idempotency.rs`)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdempotencyKey {
    pub tenant_id: String,
    pub session_id: String,
    pub request_hash: [u8; 32],
}

pub trait ScopedIdempotencyStore: Send + Sync {
    async fn check_or_insert(&self, key: &IdempotencyKey) -> PropylonResult<IdempotencyOutcome>;
    async fn record_result(&self, key: &IdempotencyKey, response: &serde_json::Value) -> PropylonResult<()>;
}
```
- Replay verification strictly checks `(tenant_id, session_id, hash)`. Any request matching a hash from a differing session is treated as a distinct execution, eliminating cross-session data leakage.

### B. Genesis Pre-Auth Correlation Engine (`src/audit/correlation.rs`)
```rust
pub struct RefusalCorrelationEnvelope {
    pub correlation_id: String,
    pub fallback_session_id: String,
    pub timestamp_epoch_ms: u64,
    pub cause: RefusalCause,
}
```
- Computes deterministic pseudo-session identifiers (`sess_preauth_<sha256_prefix>`) from connection metadata (client IP, TLS SNI, socket descriptor), ensuring 100% correlation coverage in SOC 2 hash-chain audits.

### C. Epoch Generation Slot Fence (`src/state/epoch_slot.rs`)
```rust
pub struct GenerationSlot<T> {
    pub slot_id: usize,
    pub epoch: u64,
    pub data: Option<T>,
}
```
- When a slot is recycled or evicted, its monotonic `epoch` counter increments. Late responses arriving with an older `epoch` are discarded immediately with a telemetry metric (`propylon_stale_answer_discarded_total`).

---

## 3. Empirical Verification Plan

1. **Unit & Integration Tests (`tests/phase41_idempotent_session_replay_and_task_boundaries_test.rs`)**:
   - `test_phase41_cross_session_idempotency_isolation`: Attempt replay across two distinct sessions; verify Session B triggers fresh execution instead of returning Session A's cached response.
   - `test_phase41_preauth_refusal_correlation_chain`: Trigger an ingress HTTP 400 rejection; verify that audit log records a valid `fallback_session_id` linked into the SHA-256 hash chain.
   - `test_phase41_epoch_generation_fence_discards_stale_answers`: Simulate an evicted slot; deliver a late response and verify it is rejected without corrupting the next epoch's data.
2. **Governance Verification**:
   - Enforce hard constraint `wc -l <= 350` across all new files.
   - Zero mock closures in `src/`.
   - Verified via `bash scripts/governance-check.sh`.
