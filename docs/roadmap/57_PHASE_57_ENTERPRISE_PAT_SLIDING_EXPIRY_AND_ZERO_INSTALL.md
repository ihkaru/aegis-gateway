# Phase 57 Roadmap: Enterprise Fine-Grained Personal Access Tokens (PAT), Sliding Expiry, Least-Privilege Scoping, and Zero-Client-Install Architecture

## 1. Executive Summary & Problem Statement
In Phase 56, Propylon Gateway achieved enterprise-grade cryptographic JWTs, Refresh Token Rotation (RTR), and zero-knowledge credential isolation via the local stdio bridge (`propylon bridge`). While CISO-compliant, this required installing the `propylon` binary on the client workstation and handling daily/session token lifecycles.

**Phase 57** delivers the **Zero-Client-Install, Set-and-Forget Developer Experience (DevEx)**:
1. **0% Host Installation**: No local CLI binary, daemon, Python, or Node required on the developer's laptop, PC, or Termux.
2. **One-Time Copy-Paste**: User generates a token once from the web portal and pastes it into standard MCP client configs (`~/.claude.json`, `~/.cursor/mcp.json`).
3. **Long-Lived Lifespan (30–90 Days)**: Developer does not need to re-authenticate daily.
4. **CISO Gold-Standard Security**:
   - **RFC 7519 Cryptographic JWT with canonical `prp_pat_` prefix**.
   - **Sliding-Window Inactivity Timeout**: Inactive tokens expire automatically after 14 days of non-use.
   - **Least-Privilege Tool Scoping**: Tokens can be restricted to safe tools (`execute_code`, read queries) and forbid destructive actions (`destructive_drop_table`, finance mutations).
   - **Audit Trail & Last-Used Visibility**: SIEM tracking of last-used IP, timestamp, and tools invoked.
   - **Instant One-Click Kill-Switch**: Remote revocation via Web UI or `DELETE /api/v1/auth/pat/{id}`.

---

## 2. Implementation Sub-Phases

### Phase 57.1: Fine-Grained PAT Data Model & Cryptographic Token Generation
- File: `src/policy/pat_service.rs`
- Defines `PersonalAccessToken`, `CreatePatRequest`, `CreatePatResponse`, and `PatSummary`.
- Canonical token format: `prp_pat_<base64url_jwt>` signed with HMAC-SHA256, carrying `sub`, `jti`, `token_type: "pat"`, `token_name`, `scopes`, `exp`, `max_idle_secs`.

### Phase 57.2: Sliding-Window Inactivity Engine & Replay/Revocation Registry
- Registry tracks `last_used_at` per token ID.
- Inactivity validation: Rejects requests if `now - last_used_at > max_idle_secs` (e.g. 14 days dormant).
- Atomic in-memory state with fast thread-safe locks.

### Phase 57.3: REST Management APIs for PAT
- File: `src/control/pat_api.rs`
- `GET /api/v1/auth/pat`: Lists active and revoked tokens for the authenticated user.
- `POST /api/v1/auth/pat`: Generates a new scoped PAT with custom name, TTL (30, 60, 90 days), and tool scopes.
- `DELETE /api/v1/auth/pat/{id}`: Instantly revokes the PAT and notifies revocation blacklist.

### Phase 57.4: MCP Data Plane Scope Enforcement & Data-Plane Integration
- File: `src/transport/server.rs`
- Recognize `Authorization: Bearer prp_pat_...` in HTTP/SSE data plane.
- On `tools/call`, inspect requested tool against PAT's allowed scopes (`safe_dev`, `execute_code`, `*`). Reject destructive tools if not explicitly permitted.

### Phase 57.5: Embedded Web UI "Access Tokens" Panel
- File: `ui/src/components/AccessTokensModal.tsx` & `ui/src/App.tsx`
- 1-Click modal in the Web UI:
  - Form to enter Token Name, TTL (30 / 60 / 90 days), and Scopes.
  - Generates token and provides one-click copyable snippets for Claude Code (`claude mcp add`) and Cursor (`mcp.json`).
  - Table of active tokens with "Revoke" button and "Terakhir Digunakan" status.

### Phase 57.6: Graduated Integration Test Suite & Governance Verification
- File: `tests/phase57_enterprise_pat_and_zero_install_test.rs`
- Verifies:
  1. Cryptographic PAT creation, `prp_pat_` parsing, and validation.
  2. Sliding-window inactivity rejection after idle period.
  3. Tool scope enforcement (safe tools allowed, destructive tools blocked).
  4. Remote revocation kill-switch via REST API.
  5. Direct HTTP MCP call in Claude Code / Cursor format without local binary.
- Run `bash scripts/governance-check.sh` and deploy to production on ThinkStation P320.
