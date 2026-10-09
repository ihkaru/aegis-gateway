# Phase 5: FinOps, Multi-Tenancy & Hard Budget Freezes

> **Milestone Tag**: `v0.6.0-finops-multitenancy`  
> **Status**: `Planned` (QuotaEngine trait abstracted; Metering and hard freeze engine planned)

---

## 1. Objectives

Prevent runaway cloud and LLM bills caused by infinite agent loops or unconstrained downstream API invocations. Enable clean multi-tenant isolation, precise departmental chargeback, and deterministic hard budget cutoffs.

---

## 2. Architecture & Contracts

Defined in [`src/core/state.rs`](../../src/core/state.rs):

```rust
#[async_trait]
pub trait QuotaEngine: Send + Sync {
    async fn check_budget(&self, tenant: &TenantId) -> AegisResult<bool>;
    async fn record_spend(&self, tenant: &TenantId, cost_usd: f64) -> AegisResult<()>;
}
```

### FinOps Principles

1. **Pre-Invocation Quota Check**:
   - Before executing an expensive tool (e.g., complex SQL queries, third-party data enrichment APIs, image generation), the gateway queries `check_budget`.
   - If the tenant's allocation is exhausted, the gateway immediately returns `AegisError::BudgetExceeded` without invoking downstream infrastructure.
2. **Post-Invocation Metering**:
   - Upstream API costs and compute durations are calculated and debited against the tenant's envelope via `record_spend`.
3. **Multi-Tenant Partitioning**:
   - Every cached entry, rate limit bucket, and policy rule is strictly scoped by `TenantId`.
   - Cross-tenant data leakage is fundamentally impossible at the trait boundary.

---

## 3. Milestones & Checklist

- [x] **5.1 QuotaEngine Abstraction**: Define `QuotaEngine` trait and `TenantId` type boundary in `src/core/state.rs`.
- [ ] **5.2 Real-Time Cost Metering Engine**: Track tool execution duration, network egress, and provider API costs in atomic Redis counters.
- [ ] **5.3 Automated Hard Budget Freezes**:
  - Soft warning trigger at 80% quota (emits audit event and notification webhook).
  - Hard cutoff trigger at 100% quota (blocks further tool execution).
- [ ] **5.4 Departmental Chargeback Reporting**: Export monthly cost accounting metrics broken down by department, team, project, and tool category.
- [ ] **5.5 Prometheus FinOps Exporter**: Expose `/metrics` endpoint with `aegis_tenant_cost_usd_total` and `aegis_tenant_tokens_total` for corporate Grafana dashboards.
