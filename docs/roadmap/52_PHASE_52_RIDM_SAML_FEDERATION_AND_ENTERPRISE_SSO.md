# Phase 52: rIDM SAML 2.0 Federation & Enterprise SSO

> **Milestone Tag**: `v1.52.0-ridm_saml_federation_and_sso`  
> **Status**: `Completed & Empirically Verified`  
> **Target Enterprise Gaps & Standards**:
> - Native SAML 2.0 Web Browser SSO Profile (SP-Initiated & IdP-Initiated).
> - Cryptographic assertion verification, expiration check (`NotOnOrAfter`), and audience restriction.
> - Direct integration with rIDM Identity Provider (`https://auth.dvlpid.my.id/t/bps`) with fallback to generic `SAML_IDP_*` environment variables.
> - Embedded Admin UI SSO widget and zero-friction session management.

---

## 1. Problem Statement & Motivation

Enterprise deployments cannot rely on hardcoded local API keys or isolated user accounts. In mature corporate and government environments (such as BPS), all employee and administrator authentication must flow through a centralized Identity Provider (IdP) supporting SAML 2.0 or OIDC.

Propylon Gateway must act as a first-class SAML 2.0 Service Provider (SP), generating valid `<samlp:AuthnRequest>` payloads, redirecting users to the enterprise login portal, verifying signed assertions, and mapping SAML attribute statements into gateway ABAC caller contexts.

---

## 2. Technical Architecture & Trait Contracts

```rust
pub trait SamlServiceProvider: Send + Sync {
    /// Generate an XML AuthnRequest for redirecting a user to the enterprise IdP.
    fn create_authn_request(&self, destination_url: &str) -> Result<String, PropylonError>;

    /// Parse and cryptographically validate an incoming SAML Response XML assertion.
    fn validate_assertion(&self, saml_xml: &str) -> Result<SamlAssertionContext, PropylonError>;
}
```

### Endpoints Implemented in `src/control/saml_api.rs`:
1. `GET /api/v1/auth/saml/config`: Exposes IdP and SP metadata endpoints for UI clients.
2. `GET /api/v1/auth/saml/metadata`: Generates standard SP XML entity descriptor.
3. `GET /api/v1/auth/saml/login`: Initiates Web SSO redirect with Base64 `SAMLRequest`.
4. `POST /api/v1/auth/saml/acs`: Assertion Consumer Service validating incoming assertions and issuing Propylon session tokens.
5. `GET /api/v1/auth/session`: User session introspection validating active claims.
6. `POST /api/v1/auth/logout`: Invalidates session state.

---

## 3. Verification & Empirical Evidence

- **Unit & Integration Suite**: `tests/phase52_ridm_saml_login_and_sso_test.rs` (5/5 tests passing).
- **Live Production Registration**: Propylon SP registered in rIDM database on ThinkStation server (`entity_id: https://mcp.dvlpid.my.id/saml/metadata`).
