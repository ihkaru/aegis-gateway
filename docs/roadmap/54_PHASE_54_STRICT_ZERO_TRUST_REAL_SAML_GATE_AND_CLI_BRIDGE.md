# Phase 54: Strict Zero-Trust Enforcement, Real SAML 2.0 Gate & CLI Stdio Bridge

**Document Version:** 1.0.0  
**Status:** Approved for Implementation  
**Standard:** Enterprise Zero-Trust Architecture & NIST SP 800-207 (October 2026)  
**Strict File Constraint:** `wc -l <= 350` across all files.

---

## 1. Executive Summary & Problem Statement

### 1.1 The Problems
1. **Unprotected Dashboard Exposure**: In previous builds, unauthenticated browser visitors could see the entire admin dashboard because `/api/v1/auth/session` had a development mock fallback, and the Svelte UI lacked a strict authentication gate.
2. **Missing HTTP 302 Redirection**: `/api/v1/auth/saml/login` returned raw JSON `{ "redirect_url": "..." }` instead of an immediate browser-level `HTTP 302 Found` redirection to the real rIDM SAML IdP (`https://auth.dvlpid.my.id/t/bps/saml/sso`).
3. **Data Plane Enforcement Gap**: Under `strict` policy tier, unauthenticated MCP requests (`/mcp`, `/sse`) were not actively rejected with HTTP 401 if missing an enterprise Bearer token.
4. **Headless Client Friction ("Cara B")**: Developers using CLI tools like Claude Code and Antigravity IDE require a local stdio-to-remote-SSE bridge (`propylon bridge`) and CLI login helper (`propylon login`) to authenticate without manual token copy-pasting.

### 1.2 Proposed Architecture
- **Pure Rust Embedded Single-Binary**: Keep Svelte embedded via `rust-embed` in Axum. SvelteKit with Node.js SSR is rejected because it introduces a ~200MB Node.js runtime container dependency, breaking the single-port 30MB rootless container invariant.
- **Client-Side Auth Guard in Svelte**: A strict `LoginGate` component completely blocks dashboard rendering until `/api/v1/auth/session` returns a verified identity.
- **Real HTTP 302 Redirections**: `/saml/login` redirects directly to rIDM IdP; ACS (`/api/v1/auth/saml/acs`) validates assertions and redirects back to `/?token=...`.
- **Strict Zero-Trust MCP Gate**: Rejects unauthenticated `/mcp` and `/sse` requests with HTTP 401 when `strict` policy tier is active.
- **CLI Stdio Bridge**: `propylon bridge --remote <URL>` proxies stdin/stdout JSON-RPC to remote SSE with auto-injected Bearer tokens.

---

## 2. Implementation Phases & Milestones

### Phase 54.1: Real HTTP 302 SAML Redirection & Zero-Mock Session
- Update `SamlApiDispatcher`:
  - `GET /saml/login` & `GET /api/v1/auth/saml/login`: Generate AuthnRequest and return `(302, Location: https://auth.dvlpid.my.id/t/bps/saml/sso?SAMLRequest=...)`.
  - `POST /api/v1/auth/saml/acs`: Validate assertion; for browser form POSTs, return `(302, Location: /?token=propylon_jwt_...)`.
  - `GET /api/v1/auth/session`: Return `401 Unauthorized` (`authenticated: false`) if no valid token provided. Remove all mock fallbacks.

### Phase 54.2: Svelte UI Zero-Trust Authentication Barrier
- Remove static mock login button from `AuthModal.svelte`.
- Implement full-screen `LoginGate.svelte` with enterprise branding and single "Sign in with BPS rIDM SSO" button.
- In `App.svelte`: If `!authUser`, render only `LoginGate`. Once authenticated, render full dashboard.

### Phase 54.3: Strict Zero-Trust Data Plane Enforcement
- In `LiveHttpServer` (`handle_post_mcp` & `handle_sse`):
  - Check `Authorization: Bearer <token>` header when `policy_tier == "strict"`.
  - Return `HTTP 401 Unauthorized` with JSON-RPC error if unauthenticated.
  - Populate `CallerContext` with verified SAML subject for ABAC & audit.

### Phase 54.4: CLI Login & Stdio Bridge Subcommands ("Cara B")
- Implement `propylon login` (browser-assisted login).
- Implement `propylon bridge --remote <URL>` for Claude Code & Antigravity IDE stdio transport.

### Phase 54.5: Gauntlet & Production Verification
- Create `tests/phase54_strict_zero_trust_and_cli_bridge_test.rs`.
- Verify 100% test passing, `wc -l <= 350`, zero mocks. Deploy to ThinkStation Coolify.
