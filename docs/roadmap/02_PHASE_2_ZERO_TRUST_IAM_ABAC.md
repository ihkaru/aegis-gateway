# Phase 2: Enterprise Zero-Trust IAM & Granular ABAC

> **Milestone Tag**: `v0.3.0-iam-abac`  
> **Status**: `Completed` (OIDC federation, OPA engine, Vault secrets, and SCIM revocation verified)

---

## 1. Objectives

Replace static shared bearer tokens and coarse admin bits with enterprise-grade Zero-Trust Identity and Attribute-Based Access Control (ABAC) capable of inspecting dynamic payload arguments and federating with corporate Identity Providers (IdP).

---

## 2. Architecture & Contracts

Defined in [`src/core/policy.rs`](../../src/core/policy.rs), [`src/core/identity.rs`](../../src/core/identity.rs), [`src/core/secrets.rs`](../../src/core/secrets.rs), and [`src/core/session.rs`](../../src/core/session.rs):

```rust
#[async_trait]
pub trait PolicyEngine: Send + Sync {
    async fn evaluate(&self, ctx: &PolicyContext) -> AegisResult<PolicyDecision>;
    fn eval_payload(&self, tool: &str, arguments: &Value) -> AegisResult<PolicyDecision>;
}

#[async_trait]
pub trait TokenValidator: Send + Sync {
    async fn validate_token(&self, token: &str) -> AegisResult<CallerContext>;
}
```

### Policy Dimensions

1. **Subject Context**: User identity (`sub`), corporate email, assigned department, verified roles (`roles`).
2. **Resource Context**: Target backend MCP server and tool name.
3. **Payload Inspection (Argument Constraints)**:
   - Financial tools (`stripe_refund`, `wire_transfer`) capped by `arguments.amount <= max_amount`.
   - Database query tools restricted to read-only statements, barring `DROP`, `DELETE`, `TRUNCATE`, or `ALTER TABLE`.
4. **Environment Context**: Request timestamp, client IP CIDR, session revocation checks.

---

## 3. Milestones & Checklist

- [x] **2.1 Core ABAC Engine Interface**: Policy evaluation with role checks and payload argument bounds (Empirically verified in `tests/enterprise_governance_test.rs`).
- [x] **2.2 OIDC / SAML 2.0 Federation**: Support JWT token validation against Okta, Microsoft Entra ID, Keycloak via `OidcTokenValidator` (Empirically verified in `tests/phase2_zero_trust_test.rs`).
- [x] **2.3 Open Policy Agent (OPA / Rego) & Cedar Engine**: Pluggable `OpaPolicyEngine` evaluating declarative rules, payload limits, and SQL DDL prevention (Empirically verified in `tests/phase2_zero_trust_test.rs`).
- [x] **2.4 Enterprise Secret Store Integration**: Native secret resolution via `VaultSecretStore` and `EnvSecretStore` with zero unsafe code (Empirically verified in `tests/phase2_zero_trust_test.rs`).
- [x] **2.5 Automated SCIM Provisioning & Deactivation**: Invalidate revoked sessions instantly and deactivate subjects across sessions via `MemoryRevocationRegistry` (Empirically verified in `tests/phase2_zero_trust_test.rs`).

