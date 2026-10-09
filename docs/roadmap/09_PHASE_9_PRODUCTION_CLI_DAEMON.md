# Phase 9: Production CLI Daemon & Legacy Parity

> **Milestone Tag**: `v1.0.0-cli-daemon-and-parity`  
> **Status**: `Completed`  
> **Standards Reference**: POSIX Utility Conventions, 12-Factor App (Process & Port Binding), MCP Specification 2024-11-05 & 2025-11-25

---

## 1. Executive Summary & Objective

Phase 9 transforms Aegis Gateway from an embedded library into a **standalone, production-grade CLI daemon** and ensures **100% operational parity** with legacy `mcp-gateway` deployments.

Developers can run Aegis Gateway directly from their terminal, manage backends via dynamic CLI subcommands, deploy it behind Docker/Kubernetes, and connect AI desktop clients (Claude Desktop, Cursor) with zero manual friction.

---

## 2. CLI Architecture & Subcommand Matrix

The Aegis CLI (`aegis-gateway`) provides rich ergonomics powered by `clap`:

| Subcommand | Functionality | Primary Persona |
| :--- | :--- | :--- |
| `serve` | Start HTTP daemon (`0.0.0.0:39400`) or Stdio channel (`--stdio`) | SRE, Cloud Ops, Desktop Dev |
| `init` | Scaffold starter `aegis.yaml` with best practices | Solo Developer, Hobbyist |
| `validate` | Preflight parse & validate YAML/JSON topology file | CI/CD Engineer, GitOps |
| `add` | Dynamically add/update an MCP backend server | Developer, Automation Script |
| `remove` | Remove an MCP backend server from configuration | Developer, Automation Script |
| `list` | Inspect all configured backend servers (text or JSON) | Operator, Platform Lead |
| `doctor` | Preflight check host runtimes (`bun`/`node`, `python3`, `git`) | Setup Engineer, Onboarding |
| `audit` | Export SOC 2 Type II cryptographic hash-chain audit report | Compliance Officer, CISO |

---

## 3. Legacy Operational Parity & Bugfixes Solved

Phase 9 audits and closes critical operational failure modes identified from real-world usage of legacy MCP gateways:

### 3.1. Tool Search by Backend Name (Issue #2317)
* **Problem**: In legacy gateways, users or LLMs querying a backend name (e.g., `postgres`, `codesearch`) received zero results if the tool names (`query_sql`, `find_symbol`) and descriptions did not happen to repeat the server name.
* **Aegis Parity**: `ProgressiveProjector::filter_by_context` scores and ranks candidate tools using `tool.server` and `tool.tags`, ensuring tools served by that backend rank at the top of progressive discovery.

### 3.2. Dedicated Package Manager Cache Isolation (Issue #622)
* **Problem**: Concurrently spawning multiple `npx` or `uv` backend subprocesses causes lock contention on the host's global `~/.npm` or `~/.cache` directory, tearing the cache tree and failing subsequent spawns with `MODULE_NOT_FOUND`.
* **Aegis Parity**: `HermeticSubprocessBackend::build_command_for_backend` automatically generates dedicated, sanitized cache directories (`$TMPDIR/aegis-cache/npm/<backend>`, `UV_CACHE_DIR`, `PIP_CACHE_DIR`) while preserving operator-set overrides.

### 3.3. Cross-Platform Windows & Unix Command Splitting (Issue #523 / #527)
* **Problem**: Windows path separators (`C:\Windows\py.exe`) are corrupted by POSIX `shlex` as escape sequences (`C:Windowspy.exe`).
* **Aegis Parity**: Host-aware `command_split` module implements both Unix POSIX shell quote rules and Windows `CommandLineToArgvW` rules, preserving backslashes as path separators.

### 3.4. Streamable HTTP Protocol Version Header Validation (Issue #540)
* **Problem**: Clients sending unserved or bogus protocol versions (e.g. `1999-01-01`) would silently receive responses shaped by unsupported versions without notification.
* **Aegis Parity**: `LiveHttpServer` validates `mcp-protocol-version` request headers on `/mcp`. Supported versions (`2024-11-05`, `2025-11-25`, `2024-10-07`) and absent headers pass; invalid revisions receive standard HTTP 400 with JSON-RPC error code `-32022`.

---

## 4. Empirical Verification Evidence

1. **CLI Daemon Workflow**: `tests/phase9_cli_daemon_test.rs` (5 passing tests).
   - `test_phase9_init_and_validate_workflow`
   - `test_phase9_doctor_preflight`
   - `test_phase9_live_http_server_router_endpoints`
   - `test_phase9_daemon_supervisor_bootstrap`
   - `test_phase9_audit_soc2_report_export`
2. **Legacy Parity Verification**: `tests/phase9_legacy_parity_test.rs` (5 passing tests).
   - `test_issue_2317_search_tool_by_backend_name`
   - `test_issue_622_per_backend_package_manager_cache_isolation`
   - `test_issue_523_windows_and_unix_command_split`
   - `test_issue_540_mcp_protocol_version_header_validation`
   - `test_cli_add_remove_list_dynamic_topology_lifecycle`
