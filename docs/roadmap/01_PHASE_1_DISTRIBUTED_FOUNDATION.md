# Phase 1: Distributed State & High Availability Foundation

> **Milestone Tag**: `v0.2.0-distributed`  
> **Status**: `In Progress` (Trait abstractions & InMemory mock verified; Redis backend planned)

---

## 1. Objectives

Eliminate the single-process in-memory bottleneck present in traditional MCP proxies. Enable `aegis-gateway` to run as stateless horizontal replicas across Kubernetes pods behind cloud load balancers (AWS ALB, GCP Cloud Load Balancing, Envoy Ingress).

---

## 2. Architecture & Contracts

All state operations are decoupled via asynchronous Rust traits in [`src/core/state.rs`](../../src/core/state.rs):

```rust
pub trait DistributedState: DistributedCache + DistributedRateLimiter + DistributedCircuitBreaker + QuotaEngine {}
```

### Key Components

1. **Distributed Cache (`DistributedCache`)**:
   - Cross-pod response caching with TTL expiration.
   - Cache key normalization: `tenant:server:tool:args_sha256`.
2. **Distributed Rate Limiting (`DistributedRateLimiter`)**:
   - Cluster-wide sliding window / token bucket algorithm.
   - Prevents noisy neighbor pods from overwhelming shared backend servers.
3. **Distributed Circuit Breaker (`DistributedCircuitBreaker`)**:
   - Shared failure counters across all replicas.
   - If Pod A encounters 5 consecutive backend timeouts, the circuit trips to `OPEN` cluster-wide, protecting downstream MCP servers immediately.

---

## 3. Milestones & Checklist

- [x] **1.1 Trait Contract Abstraction**: Decouple `DistributedCache`, `DistributedRateLimiter`, and `DistributedCircuitBreaker` into `core::state` (Empirically verified in `tests/enterprise_governance_test.rs`).
- [x] **1.2 InMemory Reference Engine**: Provide thread-safe `InMemoryStateBackend` for localized developer workflow and CI test suites.
- [ ] **1.3 Redis Cluster State Driver**: Implement `RedisStateBackend` with connection pooling (`bb8-redis`), Lua-scripted atomic rate limiting, and pub/sub circuit breaker notifications.
- [ ] **1.4 Kubernetes Health & Readiness Probes**: Implement `/healthz` (liveness) and `/readyz` (readiness probing distributed state backend).
- [ ] **1.5 Graceful Shutdown & Drain**: Handle `SIGTERM` / `SIGINT` signals with inflight tool call completion draining.
