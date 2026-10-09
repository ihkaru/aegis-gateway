# Phase 2: Enterprise Zero-Trust IAM & Granular ABAC

> **Milestone Tag**: `v0.3.0-iam-abac`  
> **Status**: `Planned` (Core ABAC engine implemented; Enterprise IdP federation planned)

---

## 1. Objectives

Replace static shared bearer tokens and coarse admin bits with enterprise-grade Zero-Trust Identity and Attribute-Based Access Control (ABAC) capable of inspecting dynamic payload arguments and federating with corporate Identity Providers (IdP).

---

## 2. Architecture & Contracts

Defined in [`src/core/policy.rs`](../../src/core/policy.rs):

```rust
#[async_trait]
pub trait PolicyEngine: Send + Sync {
    async fn evaluate(&self, ctx: &PolicyContext) -> AegisResult<PolicyDecision>;
    fn eval_payload(&self, tool: &str, arguments: &Value) -> AegisResult<PolicyDecision>;
}
```

### Policy Dimensions

1. **Subject Context**: User identity (`sub`), corporate email, assigned department, verified roles (`roles`).
2. **Resource Context**: Target backend MCP server and tool name.
3. **Payload Inspection (Argument Constraints)**:
   - Example 1: Financial tools (`stripe_refund`, `wire_transfer`) strictly capped by `arguments.amount <= threshold`.
   - Example 2: Database query tools restricted to read-only statements, barring `DROP`, `DELETE`, or `ALTER`.
4. **Environment Context**: Request timestamp, client IP CIDR, session authentication freshness.

---

## 3. Milestones & Checklist

- [x] **2.1 Core ABAC Engine Interface**: Policy evaluation with role checks and payload argument bounds (Empirically verified in `tests/enterprise_governance_test.rs`).
- [ ] **2.2 OIDC / SAML 2.0 Federation**: Support JWT token validation against Okta, Microsoft Entra ID, Keycloak, and AWS IAM Identity Center via JWKS endpoint caching.
- [ ] **2.3 Open Policy Agent (OPA / Rego) & Cedar Engine**: Pluggable external policy evaluator executing custom enterprise security rules.
- [ ] **2.4 Enterprise Secret Store Integration**: Native secret resolution via HashiCorp Vault AppRole / K8s auth and AWS Secrets Manager (replacing local `.env` and OS keychains).
- [ ] **2.5 Automated SCIM Provisioning**: Invalidate revoked sessions instantly upon user deactivation in corporate directory.
