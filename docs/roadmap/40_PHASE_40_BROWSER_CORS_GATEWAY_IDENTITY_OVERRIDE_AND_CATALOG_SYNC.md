# Phase 40: Browser CORS, Gateway Identity Override & Catalog Sync Sentinel

> **Milestone Tag**: `v1.40.0-browser_cors_identity_override_and_catalog_sync`  
> **Status**: `In Planning`  
> **Target Gaps & Standards**:
> - Resolves `docker/mcp-gateway#544` (Opt-in browser CORS for Streamable HTTP MCP clients)
> - Resolves `docker/mcp-gateway#583` (Configurable `serverInfo.name` identity override via CLI / Config)
> - Resolves `docker/mcp-gateway#561` (Catalog synchronization zero-tool drop refusal)
> - Standard Compliance: W3C CORS Specification, MCP Protocol 2024-11-05, OWASP API Security Top 10

---

## 1. Problem Statement & Motivation

First-generation MCP gateways exhibited three prominent production limitations when integrated into modern browser and enterprise environments:

1. **Browser CORS Failure on Streamable HTTP (`docker/mcp-gateway#544`)**:
   - Modern browser-based web applications (e.g., chat interfaces, web IDEs, internal agent portales) connecting directly to Streamable HTTP endpoints (`POST /mcp`) are blocked by standard browser security models because first-generation gateways omit `Access-Control-Allow-Origin`, `Access-Control-Allow-Methods`, and preflight `OPTIONS` routing.
2. **Hardcoded Gateway Identity (`docker/mcp-gateway#583`)**:
   - Upstream reference implementations hardcoded `serverInfo.name` (e.g. `"docker-mcp-gateway"`), preventing enterprises from white-labeling, segmenting gateway tiers (`"corp-staging-gateway"` vs `"corp-prod-gateway"`), or complying with internal infrastructure naming conventions.
3. **Catastrophic Silent Zero-Tool Drop (`docker/mcp-gateway#561`)**:
   - During upstream catalog polling or re-syncing (`catalog pull`), temporary upstream network blips or empty responses caused the gateway to wipe out its internal tool registry, abruptly reporting 0 available tools to connected agents without warning.

---

## 2. Technical Architecture & Trait Contracts

Aegis Gateway introduces three dedicated core abstractions in `src/transport/` and `src/discovery/`:

### A. Dynamic Browser CORS Enforcer (`src/transport/cors.rs`)
```rust
pub trait CorsPolicyEnforcer: Send + Sync {
    fn evaluate_preflight(&self, origin: Option<&str>, requested_headers: Option<&str>) -> CorsPreflightOutcome;
    fn apply_cors_headers(&self, headers: &mut axum::http::HeaderMap, origin: Option<&str>);
}
```
- Configurable origin whitelist supporting explicit domains, wildcard dev modes, and regex subdomains.
- Preflight `OPTIONS` fast-path returning `204 No Content` with cached `Access-Control-Max-Age: 86400`.
- Automatic pass-through of `mcp-protocol-version`, `content-type`, and `authorization` headers.

### B. Gateway Server Identity Override (`src/core/identity.rs`)
```rust
pub struct GatewayServerIdentity {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
}
```
- Configurable via `aegis.yaml` (`server_info.name`), CLI (`--server-name <NAME>`), or environment variable (`AEGIS_SERVER_NAME`).
- Propagated deterministically into JSON-RPC `initialize` responses:
  ```json
  {
    "serverInfo": {
      "name": "enterprise-corp-gateway",
      "version": "1.40.0"
    }
  }
  ```

### C. Catalog Drift & Zero-Tool Fallback Sentinel (`src/discovery/catalog_sentinel.rs`)
```rust
pub trait CatalogSyncSentinel: Send + Sync {
    fn evaluate_catalog_sync(&self, server: &str, previous_tools: &[ToolDefinition], incoming_tools: &[ToolDefinition]) -> CatalogSyncOutcome;
}
```
- Anti-zero-tool drop rule: If an active server returns 0 tools without an explicit administrative deletion event, the sync is refused with `CatalogSyncOutcome::RefusedEmptySync`, preserving the existing healthy snapshot and alerting telemetry.

---

## 3. Empirical Verification Plan

1. **Unit & Integration Tests (`tests/phase40_browser_cors_gateway_identity_override_and_catalog_sync_test.rs`)**:
   - `test_phase40_browser_cors_preflight_and_origin_whitelist`: Verify `OPTIONS` preflight, allowed headers, and rejected disallowed origins.
   - `test_phase40_server_info_identity_override`: Verify custom server name reflection in initialize handshake.
   - `test_phase40_catalog_zero_tool_drop_protection`: Verify preservation of cached catalog when incoming sync returns 0 tools.
2. **Governance Verification**:
   - Enforce hard constraint `wc -l <= 350` across all new files.
   - Zero mock closures in `src/`.
   - Verified via `bash scripts/governance-check.sh`.
