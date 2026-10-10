# Phase 31: Enterprise Identity Federation — SAML 2.0 Web SSO & SCIM 2.0 Inbound Receiver

## Executive Summary
Phase 31 bridges Aegis Gateway with enterprise identity providers (IdPs)—such as Microsoft Entra ID (Azure AD), Okta, Ping Identity, and Active Directory Federation Services (ADFS). By supporting legacy SAML 2.0 XML assertion verification alongside automated SCIM 2.0 (RFC 7644) inbound lifecycle provisioning and deprovisioning, enterprises can govern AI agent access and developer tool sessions using their existing corporate directory without vendor lock-in.

---

## 1. Architectural Invariants & Requirements

1. **SAML 2.0 Service Provider (SP)**:
   - Standard AuthnRequest generation and base64 XML assertion payload validation.
   - Verification of Issuer, Audience Restriction (`EntityID`), valid lifetime (`NotBefore` / `NotOnOrAfter`), and cryptographic signatures.
   - Role and department extraction from SAML AttributeStatements.
2. **SCIM 2.0 Inbound Receiver (RFC 7644)**:
   - Handle `/v2/Users` and `/v2/Groups` lifecycle webhooks from corporate directories.
   - Immediate session revocation and token tombstoning upon `active: false` (deprovisioning) events.
   - Group-to-Policy tier mapping (e.g. `Security-Auditors` -> `Strict`, `Engineering` -> `Dev`).
3. **Interface-First & Zero Mocks**:
   - Contracts `SamlServiceProvider` and `ScimInboundReceiver` defined in `src/core/federation.rs`.
   - Production drivers implemented in `src/policy/saml.rs` and `src/policy/scim.rs`.
   - Strict `wc -l <= 350`, zero unsafe code, and MIT compliance.

---

## 2. Verification Criteria

- [x] SAML AuthnRequest generation outputs RFC-compliant parameters.
- [x] SAML response parser correctly verifies valid assertions and rejects expired/tampered XML tokens.
- [x] SCIM 2.0 user deprovisioning immediately triggers token tombstoning and session teardown.
- [x] Group provisioning maps corporate directory memberships to appropriate Aegis PolicyTiers.
