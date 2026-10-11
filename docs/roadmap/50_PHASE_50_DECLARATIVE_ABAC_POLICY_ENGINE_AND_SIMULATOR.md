# Phase 50: Declarative ABAC Policy-as-Code Engine & Dry-Run Simulator

> **Milestone Tag**: `v1.50.0-abac_policy_simulator`  
> **Status**: `In-Progress`  
> **Target Enterprise Gaps & Standards**:
> - 4-Dimensional evaluation: Principal (claims/roles), Action (tool names/verbs), Resource (backend/table classification), Environment (time/subnet/MFA).
> - Declarative YAML/JSON Policy-as-Code contract with atomic zero-restart hot-reloading via RCU in-memory swap.
> - Embedded Control Plane Dry-Run Simulator endpoint (`POST /api/v1/policy/simulate`).
> - Cryptographic downstream assertion propagation (`X-Aegis-User-Id`, `X-Aegis-Tenant-Id`, `X-Aegis-Signature`).

---

## 1. Problem Statement & Motivation

Static Role-Based Access Control (RBAC) is insufficient for AI agents:
1. "Admin vs User" cannot distinguish between benign database `SELECT` and destructive `DROP TABLE`.
2. SecOps teams must audit and test access policies in Git before deployment.
3. SOC 2 Type II compliance demands dry-run simulation to prove access policies behave predictably before activating them in production.

---

## 2. Technical Architecture & Trait Contracts

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbacContext {
    pub principal: PrincipalAttributes,
    pub action: ActionAttributes,
    pub resource: ResourceAttributes,
    pub environment: EnvironmentAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbacDecision {
    Allow { reason: String },
    Deny { reason: String },
    RequireHitl { risk_tier: String, reason: String },
}

#[async_trait]
pub trait AbacPolicyEngine: Send + Sync {
    /// Evaluates a 4-dimensional context against active rules
    async fn evaluate(&self, ctx: &AbacContext) -> AegisResult<AbacDecision>;

    /// Simulates decision for dry-run testing in Embedded UI
    async fn simulate(&self, ctx: &AbacContext) -> AegisResult<PolicySimulationResult>;

    /// Atomically replaces policy rules without restart
    async fn reload_rules(&self, rules: Vec<AbacRule>) -> AegisResult<()>;
}
```

### Key Guarantees:
- **Default Deny Invariant**: Unmatched invocations are strictly denied.
- **Microsecond Evaluation**: Policy evaluation completes in <50 microseconds using compiled in-memory matcher trees.
- **Cryptographic Downstream Provenance**: Downstream backends receive HMAC-SHA256 signed assertions.
