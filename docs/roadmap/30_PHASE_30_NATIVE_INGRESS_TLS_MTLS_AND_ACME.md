# Phase 30: Native Ingress Hardening — Pure-Rust TLS (rustls), mTLS Zero-Trust & ACME Auto-Cert Lifecycle

## Executive Summary
Phase 30 equips Aegis Gateway with native, memory-safe L4/L7 ingress hardening via pure-Rust TLS (`rustls`), Mutual TLS (mTLS) client verification for zero-trust microservice meshes, and automated certificate management via ACME (RFC 8555 / Let's Encrypt / Smallstep step-ca). This eliminates the mandatory dependency on external reverse proxies (such as NGINX or Caddy) while maintaining 100% interoperability with existing enterprise ingress controllers.

---

## 1. Architectural Invariants & Requirements

1. **Pure-Rust TLS & Zero Unsafe Code**:
   - Memory-safe TLS 1.3/1.2 termination via Rust-native crypto (`rustls`), completely avoiding legacy C-based OpenSSL memory safety vulnerabilities.
   - Enforce ALPN negotiation for `h2` and `http/1.1`.
2. **Mutual TLS (mTLS) Zero-Trust Enforcement**:
   - Cryptographic X.509 client certificate validation against configured enterprise Root and Intermediate Certificate Authorities (CAs).
   - Extensible Subject Alternative Name (SAN), Common Name (CN), and serial number validation to authenticate client AI agents and backend services.
3. **Automated ACME Lifecycle (RFC 8555)**:
   - Native HTTP-01 challenge responder for automated domain verification and renewal without service disruption or restart.
   - Auto-renewal trigger window (30 days before expiration) with atomic in-memory certificate swapping.
4. **Strict Interface-First Design**:
   - Abstract trait `TlsIngressEngine`, `MtlsClientVerifier`, and `AcmeCertificateManager` defined in `src/core/ingress_tls.rs`.
   - Concrete, zero-mock production implementations in `src/transport/tls_listener.rs`.

---

## 2. Verification Criteria

- [x] Native TLS termination handles valid X.509 certificate chains and negotiates modern ciphersuites.
- [x] Strict mTLS rejects unauthenticated clients and untrusted certificate authorities with fail-closed refusal.
- [x] ACME HTTP-01 challenge handler verifies tokens and applies atomic certificate renewal in memory.
- [x] All source code adheres strictly to `wc -l <= 350`, `#![deny(unsafe_code)]`, and zero mock closures.
