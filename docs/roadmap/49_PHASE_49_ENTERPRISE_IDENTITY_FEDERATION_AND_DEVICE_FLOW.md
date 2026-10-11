# Phase 49: Enterprise Identity Federation Broker & OAuth 2.1 Device Flow

> **Milestone Tag**: `v1.49.0-identity_federation_device_flow`  
> **Status**: `In-Progress`  
> **Target Enterprise Gaps & Standards**:
> - RFC 8628 OAuth 2.0 / 2.1 Device Authorization Grant for AI CLI clients (Cursor, Claude Code, Terminal).
> - Enterprise Identity Assertion ingestion (OIDC ID Token & SAML 2.0 Assertion) with RFC 8693 Token Exchange.
> - Audience and Resource Indicators (RFC 8707) preventing cross-server credential replay.
> - Pure-Rust Zero-Mock Claims Extraction with cryptographic signature verification.

---

## 1. Problem Statement & Motivation

Enterprise AI agents run either as interactive developer tools (CLI / IDE extensions) or as autonomous background processes. Legacy IAM configurations struggle because:
1. CLI agents cannot handle traditional SAML 2.0 browser form-POST redirects directly.
2. Direct connection to upstream MCP servers causes credential sprawl and lacks central governance.
3. Security operations require single-sign-on (SSO) with Okta, Microsoft Entra ID, or Keycloak, followed by short-lived scoped JWT issuance.

---

## 2. Technical Architecture & Trait Contracts

```rust
#[async_trait]
pub trait IdentityFederationBroker: Send + Sync {
    /// Ingests an enterprise SAML assertion or OIDC ID Token and issues a scoped JWT
    async fn exchange_assertion(&self, req: AssertionExchangeRequest) -> AegisResult<FederatedTokenResponse>;

    /// Initiates RFC 8628 Device Authorization Grant for CLI clients
    async fn start_device_flow(&self, client_id: &str, scopes: Vec<String>) -> AegisResult<DeviceAuthInitResponse>;

    /// Polls device flow status during user browser verification
    async fn poll_device_flow(&self, device_code: &str) -> AegisResult<DeviceFlowPollResponse>;
}
```

### Key Guarantees:
- **Zero-Touch Provisioning**: Ingests enterprise group claims (`groups: ["Finance-Engineers"]`) and maps them to Aegis clearance attributes.
- **RFC 8707 Resource Indicators**: Every issued token binds strictly to target MCP server URIs.
- **Deterministic Token Lifetime**: Default TTL 900s (15 min) with cryptographic revocation checks.
