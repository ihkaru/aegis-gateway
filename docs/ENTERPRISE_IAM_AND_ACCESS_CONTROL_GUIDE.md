# Enterprise IAM, SSO Federation & ABAC Access Control Guide

> **Authoritative Technical Specification & Operational Runbook**  
> **Applicable Version**: Propylon Gateway v1.49.0+  
> **Security Standards Compliance**: NIST SP 800-207 Zero Trust, OAuth 2.1, RFC 8628, RFC 8693, RFC 8707, SOC 2 Type II, ISO 27001  

---

## 1. Executive Summary & Architectural Invariant

As enterprises deploy Model Context Protocol (MCP) servers across departments, direct agent-to-tool connections create severe security liabilities:
- **Credential Sprawl**: Distributing API keys and database credentials to individual developers or AI clients.
- **Audit Blind Spots**: Lack of centralized, immutable logging for tool calls and sensitive data exfiltration.
- **The Confused Deputy Vulnerability**: AI agents manipulated via prompt injection to run privileged actions.

**Propylon Gateway enforces the NIST Zero Trust Policy Enforcement Point (PEP) & Policy Decision Point (PDP) model**:
1. AI clients authenticate **once** to Propylon Gateway via enterprise Single Sign-On (SAML 2.0 / OIDC) using **OAuth 2.1 Device Flow (RFC 8628)** or mTLS.
2. The Gateway evaluates every single JSON-RPC tool invocation against declarative **Attribute-Based Access Control (ABAC)** rules.
3. Downstream MCP servers remain **pure workers** isolated in private networks, receiving cryptographically signed user claims.

```
+------------------+          +--------------------------------------------+          +-------------------+
|  Developer CLI / |          |          PROPYLON GATEWAY (PEP + PDP)         |          | Downstream MCP    |
|  AI Agent Client | -------> | 1. OAuth 2.1 / SAML Assertion Exchange     | -------> | Pure Workers      |
|  (Cursor/Claude) |          | 2. Declarative ABAC Policy Evaluation      |  (mTLS)  | (Isolated Subnet) |
+------------------+          | 3. Dynamic Catalog Scoping (Least-Priv)   |          +-------------------+
                              | 4. Bi-directional DLP & Cryptographic SIEM |
                              +--------------------------------------------+
```

---

## 2. Authentication Flow: Interactive vs Autonomous Agents

### A. Interactive CLI / Desktop Agent Flow (RFC 8628 Device Authorization)

CLI agents cannot render SAML browser forms directly. Propylon Gateway bridges this via the OAuth 2.1 Device Authorization Grant:

1. **Initiation**: The agent issues a connection request to `POST /api/v1/auth/device/code`.
2. **Challenge**: Propylon responds with a `verification_uri` and a user code:
   ```json
   {
     "device_code": "dev_9f81a7b2c0",
     "user_code": "WDJB-9532",
     "verification_uri": "https://propylon.corp.internal/activate",
     "expires_in": 900,
     "interval": 5
   }
   ```
3. **SSO Authentication**: The developer opens the URL in their standard corporate browser and completes enterprise login via **Okta / Microsoft Entra ID / Keycloak** (including corporate MFA / Passkey).
4. **Token Issuance**: The CLI polls `POST /api/v1/auth/device/token` and receives a short-lived, audience-bound JWT upon user confirmation.

### B. Autonomous Agent / Machine-to-Machine Flow

For non-interactive background workers (e.g., automated ETL, orchestrators):
- **Mutual TLS (mTLS)** with X.509 certificates and SPIFFE ID validation.
- Or **OAuth 2.1 Client Credentials Flow** with machine identities managed in Infisical / HashiCorp Vault.

---

## 3. Legacy Enterprise Integration Patterns

Legacy enterprises (banking, healthcare, manufacturing) often maintain on-premises Active Directory or SAML-only IdPs without modern OIDC endpoints.

### Pattern 1: SAML 2.0 Assertion Exchange Broker
Propylon Gateway acts as an RFC 8693 Token Exchange Broker:
- Propylon consumes raw XML SAML 2.0 assertions signed by on-premises ADFS.
- Propylon validates the X.509 signature against the IdP certificate keystore.
- Propylon maps SAML AttributeStatements (`Department`, `SecurityClearance`, `CostCenter`) into normalized claims within the scoped access token.

### Pattern 2: Air-Gapped / Perimeter DMZ Deployment
In air-gapped environments where downstream MCP tools cannot reach the internet:
- Propylon Gateway is positioned as the single ingress proxy in the DMZ.
- Public IdP signing keys (JWKS) are pre-cached locally with configurable offline TTL.
- Downstream backends do not make outbound network calls.

---

## 4. Declarative ABAC Policy-as-Code Specification

Access control is defined declaratively and version-controlled in Git. Policies evaluate four distinct dimensions:
1. **Principal**: Identity, Department, Security Clearance, Roles.
2. **Action**: Tool Name, Operation Verb (`read`, `write`, `execute`, `mutate`).
3. **Resource**: Target Backend, Data Classification (`public`, `internal`, `pci_dss`, `phi`).
4. **Environment**: Time window, source subnet CIDR, device health posture.

### Example Policy Contract (`policies/abac.yaml`)

```yaml
version: propylon.gateway/v1alpha1
kind: AccessPolicy
metadata:
  name: corporate-data-access
spec:
  default_action: Deny
  rules:
    - name: allow-engineering-readonly-analytics
      effect: Allow
      principals:
        departments: ["Engineering", "DataScience"]
        clearance_level: ">= L2"
      actions:
        tools: ["remote_analytics_query_*", "read_metrics"]
        operation_type: "read"
      resources:
        backends: ["remote_analytics", "timescale_cluster"]
        classification: ["internal"]
      environment:
        time_window: "06:00-22:00 UTC"

    - name: require-hitl-for-database-mutations
      effect: RequireHitl
      risk_tier: "critical"
      actions:
        tools: ["execute_sql_mutation", "drop_table", "grant_permission"]
      reason: "Destructive mutations require explicit human dual-custody approval"

    - name: strict-deny-unauthorized-financial-exports
      effect: Deny
      actions:
        tools: ["export_pan_data", "dump_payroll"]
      principals_not:
        departments: ["FinanceAuditors"]
      reason: "PCI-DSS / SOX compliance boundary violation"
```

---

## 5. Embedded UI Simulator & Downstream Identity Propagation

### A. Dry-Run Simulator (`POST /api/v1/policy/simulate`)
SecOps teams test policies prior to deployment without risking operational downtime:
- Simulates evaluation for arbitrary Principal, Action, and Resource inputs.
- Returns evaluation trace, matched rule names, and resulting decision (`Allow`, `Deny`, `RequireHitl`).

### B. Downstream Signed Identity Propagation
When downstream databases require Row-Level Security (RLS), Propylon Gateway injects cryptographically signed HTTP headers:
- `X-Propylon-User-Id`: The authenticated enterprise principal.
- `X-Propylon-Tenant-Id`: The corporate tenant / cost center.
- `X-Propylon-Signature`: HMAC-SHA256 signature calculated over timestamp and user ID, preventing header forgery.

---

## 6. Why Downstream Evaluation is an Anti-Pattern

| Evaluation at Downstream MCP Servers (Anti-Pattern) | Centralized Evaluation at Propylon Gateway (Best Practice) |
| :--- | :--- |
| **Inconsistent Security**: Each tool author implements custom JWT parsers and auth checks, introducing zero-day gaps. | **Uniform Zero Trust**: Single hardened Rust engine enforces policies across all 100+ tools identically. |
| **Credential Sprawl**: Every MCP server must be registered as an OAuth client with enterprise IdP credentials. | **Zero Sprawl**: Only Propylon connects to the IdP; downstreams require zero IdP configuration. |
| **Fragmented Audit Trail**: Logs are distributed across 20+ servers in diverse formats. | **Unified Cryptographic Audit**: Centralized SOC 2 Type II hash chain and real-time SIEM streaming. |
