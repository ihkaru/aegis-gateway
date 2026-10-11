# Phase 56: Enterprise Cryptographic JWT, Token Rolling, Zero-Knowledge Credential Isolation, and CISO Governance

## Executive Summary
This phase hardens Propylon Gateway's authentication architecture against rigorous enterprise and CISO (Chief Information Security Officer) compliance standards (NIST SP 800-63B, OWASP Top 10 for LLM Applications 2026, RFC 7519, RFC 6749, and RFC 9449). It eliminates pseudo-token strings, introduces cryptographically signed HMAC-SHA256 JWTs with standard claims, sliding window token rolling, instant session revocation kill-switches, POSIX 0600 file permission lockdown, and zero-knowledge credential isolation for AI coding agents (Claude Code, Antigravity IDE, Cursor).

---

## Architecture Principles & Threat Mitigation

### Threat 1: Credential Context Leakage to LLMs (OWASP LLM06 / LLM07)
* **Risk**: If AI coding agents (Claude Code / AGY CLI) have direct access to raw Bearer tokens in their configuration or system prompts, the LLM may accidentally echo the credential into conversation history, reasoning traces, or git commits.
* **Mitigation**: **Zero-Knowledge Stdio Bridge Pattern**. The agent connects exclusively via standard input/output (`command: "propylon", args: ["bridge"]`). The local bridge process securely reads the credential from a machine-local file and injects `Authorization: Bearer <jwt>` directly into outbound TLS packets. The LLM never touches, reads, or logs the secret token.

### Threat 2: Static Token Lifetime & Replay Attacks (NIST SP 800-63B)
* **Risk**: Long-lived static tokens cannot be expired safely without disrupting operational workflows.
* **Mitigation**: **Short-Lived JWTs + Automated Token Rolling (RTR)**.
  * Access tokens expire in 1-8 hours (configurable).
  * Refresh endpoint (`POST /api/v1/auth/refresh`) issues rotated tokens.
  * Local CLI bridge silently rolls the token when nearing expiration without interrupting developer workflows.

### Threat 3: Forgery & Tampering of Pseudo-Tokens
* **Risk**: Pseudo-strings (`propylon_jwt_<user>`) lack cryptographic signatures and could be forged if network boundary is breached.
* **Mitigation**: Standard RFC 7519 HMAC-SHA256 tokens (`header.payload.signature`) signed with server master key (`PROPYLON_JWT_SECRET` / Infisical).

### Threat 4: Local Credential File Snooping
* **Risk**: Unauthorized processes on multi-user systems reading `~/.config/propylon/token`.
* **Mitigation**: POSIX `0600` permission enforcement (`read/write by owner only`).

---

## Implementation Roadmap

### Phase 56.1: Cryptographic JWT & Token Revocation Engine
* File: `src/policy/jwt_service.rs`
* Generates signed JWTs with `sub`, `iss`, `aud`, `iat`, `exp`, `jti`, `roles`, `department`, `tenant`.
* Manages in-memory token revocation blacklist (`jti`).

### Phase 56.2: REST Endpoints for Refresh & Revocation
* Update: `src/control/auth_api.rs` and `src/control/saml_api.rs`
* `POST /api/v1/auth/refresh`: Sliding window token rotation.
* `POST /api/v1/auth/revoke`: Immediate kill-switch invalidating token `jti`.
* ACS endpoint emits cryptographically signed JWT instead of pseudo-string.

### Phase 56.3: CLI Bridge Auto-Rolling & Zero-Knowledge Isolation
* Update: `src/cli/bridge_cmd.rs`
* POSIX `0600` permissions on `~/.config/propylon/token`.
* Background auto-refresh when token is within expiration window.

### Phase 56.4: UI Dashboard Session & Token Expiration Widget
* Display remaining token TTL and refresh button in Web UI.

### Phase 56.5: Verification & Governance
* Comprehensive test suite in `tests/phase56_jwt_rolling_and_ciso_governance_test.rs`.
* Full governance check execution (`bash scripts/governance-check.sh`).
