# Phase 55: Enterprise GitOps Immutability & Modular Headless Lean Profile (Opt-Out UI & Mutation REST APIs)

## 1. Executive Summary & Enterprise Background
Enterprise environments (financial institutions, government agencies, and critical infrastructure) enforce strict Zero-Trust and GitOps compliance standards (SOC 2 Type II, CIS Benchmarks, ISO 27001). Under these security regimes:
1. **Immutable Infrastructure & GitOps Only**: Runtime mutation of gateway backends, routes, and security tiers via REST APIs or MCP methods is strictly forbidden. Any configuration change must follow declarative version-controlled GitOps workflows (e.g. Kubernetes ConfigMaps mounted as `propylon.yaml`).
2. **Headless Modular Profile (UI Opt-Out)**: Microservices and data plane gateways in Kubernetes or edge environments should not serve static web UI assets, reducing both attack surface and memory footprint.
3. **Ultra-Low Memory Footprint (Lean Profile)**: Eliminating UI ring buffers, unneeded in-memory telemetry queues, and idle listeners reduces VmRSS memory consumption from ~20MB down to <10MB.

---

## 2. Architecture & Design Principles

### 2.1 Profile Configuration Schema
Configurable via environment variables, CLI flags, or `propylon.yaml`:
- `PROPYLON_UI_ENABLED=false` or `--no-ui`
- `PROPYLON_IMMUTABLE_CONFIG=true` or `--immutable-config`
- `PROPYLON_LEAN_PROFILE=true` or `--lean`

```yaml
profile:
  ui_enabled: false        # Headless mode: UI disabled
  immutable_config: true   # GitOps only: block REST API mutations
  lean_memory: true        # Zero-overhead buffers: skip UI history
```

### 2.2 Rejection Contract for Immutable Mode
When `immutable_config` is enabled, all dynamic mutation endpoints return HTTP 403:
- `POST /api/v1/backends` -> HTTP 403 Forbidden
- `DELETE /api/v1/backends/*` -> HTTP 403 Forbidden
- `POST /api/v1/config/apply` -> HTTP 403 Forbidden
- `POST /api/v1/policy/tier` -> HTTP 403 Forbidden
- MCP `gateway_register_tools` -> JSON-RPC Error -32003

Read-only observability endpoints (`GET /api/v1/overview`, `GET /api/v1/backends`, `GET /api/v1/config/raw`, `/health`, `/stats`) remain accessible.

---

## 3. Implementation Plan
- **55.1 Core Profile**: Implement `EnterpriseRuntimeProfile` in `src/core/profile.rs`.
- **55.2 Immutability Guardrail**: Enforce immutable check in `src/control/web_api.rs` and `src/control/config_api.rs`.
- **55.3 Headless Fallback**: Implement UI opt-out check in `src/transport/fallback.rs` and `src/control/web_ui.rs`.
- **55.4 CLI & Arg Binding**: Add `--no-ui`, `--immutable-config`, and `--lean` to `src/cli/args.rs`.
- **55.5 Test Suite & Governance**: `tests/phase55_gitops_immutability_and_lean_profile_test.rs` proving immutability, UI opt-out, and memory efficiency.
