# Aegis Gateway (🛡️)

> **Enterprise Zero-Trust MCP & Skill Control Plane with Distributed State, Granular ABAC, Real-Time DLP, and Tamper-Proof Audit.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust: 2024](https://img.shields.io/badge/Rust-2024%20Edition-blue.svg)](https://www.rust-lang.org)
[![Security: Zero Unsafe](https://img.shields.io/badge/Unsafe-Denied%20(%23![deny(unsafe_code)])-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Tests: 62 Passed](https://img.shields.io/badge/Tests-62%2F62%20Passed%20(100%25)-brightgreen.svg)](#verification)
[![Architecture: SOLID](https://img.shields.io/badge/Architecture-SOLID%20%26%20DI-green.svg)](#architecture)

---

## Why Aegis Gateway?

Conventional MCP proxies operate as fragile single-process scripts with in-memory state lock-in, coarse permissions, zero data-loss prevention, and zombie process leaks.

**Aegis Gateway** is engineered from day one as a **Cloud-Native, Enterprise-by-Design Control Plane** mediating between AI agents (Claude Desktop, Cursor, Goose, custom LLM agents) and tools.

```mermaid
flowchart TD
    Client["AI Agent / LLM Client<br/>(Claude, Cursor, Autonomous Agent)"] --> GW["Aegis Gateway (Zero-Trust Control Plane)"]
    
    subgraph Core["Aegis Enterprise Pipeline (SOLID / Dependency Inversion)"]
        GW --> P1["1. Distributed HA & Breakers<br/>(Streamable HTTP, Redis, K8s Probes)"]
        P1 --> P2["2. Dynamic ABAC Policy<br/>(Role + Department + Payload-Bounds)"]
        P2 --> P3["3. Real-Time DLP & Anti-Poison<br/>(Mask PCI/SSN, Block OWASP LLM01)"]
        P3 --> P4["4. Tamper-Evident SIEM Audit<br/>(SHA-256 Hash Chaining, SOC 2 Type II)"]
        P4 --> P5["5. Multi-Tenant FinOps<br/>(Real-time Metering, Hard Budget Freeze)"]
    end
    
    Core --> Tools["Hermetic Backend Upstreams<br/>(env_clear, per-backend cache, zero zombies)"]
    Core --> Skills["Centralized Skill Registry<br/>(Progressive L0/L1/L2 Disclosure)"]
```

---

## ⚡ Production CLI Toolset (Developer Experience)

Aegis Gateway comes with a dedicated, ergonomic CLI daemon (`aegis-gateway`) for developer workstations and cloud deployments:

```bash
# 1. Quickstart preflight diagnostics
aegis-gateway doctor

# 2. Scaffold starter topology configuration
aegis-gateway init --output aegis.yaml

# 3. Dynamically manage MCP backend servers
aegis-gateway add postgres --command npx --args -y @modelcontextprotocol/server-postgres
aegis-gateway add github-remote --url https://mcp.internal.acme.com/mcp
aegis-gateway list
aegis-gateway remove postgres

# 4. Validate configuration syntax & upstream availability
aegis-gateway validate --config aegis.yaml

# 5. Start live daemon:
# Run on Streamable HTTP for network clients and Kubernetes (port 39400)
aegis-gateway serve --host 0.0.0.0 --port 39400

# Or run directly on Stdio for Claude Desktop and Cursor
aegis-gateway --stdio
```

---

## Choose Your Journey (DX-Centric Personas)

### ⚡ 1. The Dev Hobbyist (Zero-Config Quickstart < 60s)
*Goal: Connect to Claude Desktop or Cursor immediately with zero external databases.*

1. **Build the CLI**:
   ```bash
   cargo build --release
   ```
2. **Add to Claude Desktop** (`claude_desktop_config.json`):
   ```json
   {
     "mcpServers": {
       "aegis": {
         "command": "/path/to/aegis-gateway/target/release/aegis-gateway",
         "args": ["--stdio"]
       }
     }
   }
   ```
3. *Why it just works*: Default in-memory drivers start instantly. Stdio `stdout` is strictly framed for JSON-RPC 2.0 with zero log pollution (diagnostic logs route exclusively to `stderr`).

---

### 🛠️ 2. The Fullstack Team Lead (Multiplexing & Parity)
*Goal: Multiplex micro-servers under declarative topology with package cache isolation.*

Create `aegis.yaml`:
```yaml
mcpServers:
  filesystem:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"]
    timeout_secs: 30
  database:
    command: "python3"
    args: ["-m", "mcp_postgres"]
    env:
      PG_HOST: "127.0.0.1"
```
Run with topology:
```bash
aegis-gateway serve --config aegis.yaml --stdio
```
* **Per-Backend Cache Isolation (Issue #622)**: Concurrent `npx` or `uv` spawns receive isolated cache subtrees under `$TMPDIR/aegis-cache/`, preventing torn package trees and `MODULE_NOT_FOUND` timeouts.
* **Server Name Search Discovery (Issue #2317)**: Prompts querying server names (e.g. `postgres`, `filesystem`) instantly surface all member tools even if descriptions omit the server name.
* **Zero Zombie Guarantee**: Subprocesses launch with `kill_on_drop(true)`—never leaving orphaned background processes.

---

### ☸️ 3. The DevOps & Cloud SRE (Kubernetes HA & Production)
*Goal: Multi-pod deployment behind AWS ALB / Ingress with zero-downtime rolling updates.*

* **Streamable HTTP (RFC 2025-03-26)**: Single endpoint `POST /mcp` eliminates dual-endpoint SSE session stickiness failures on Kubernetes load balancers.
* **Protocol Version Header Validation (Issue #540)**: Validates `mcp-protocol-version` requests, refusing unserved revisions with standard HTTP 400 Bad Request.
* **Kubernetes Health Probes**:
  * Liveness: `GET /healthz` -> `{"status":"healthy"}`
  * Readiness: `GET /readyz` -> Distributed state checks.
* **Graceful Drain Coordinator**:
  Holds SIGTERM until active inflight tool requests finish gracefully.

---

### 🔒 4. The CISO & Security Red Team (Zero-Trust & Compliance)
*Goal: Enforce OWASP Top 10 for LLMs, PCI-DSS / HIPAA DLP, and SOC 2 Type II audit integrity.*

* **OWASP LLM08 Hermetic Subprocess Isolation**: Child processes launch with `cmd.env_clear()`, strictly preventing untrusted packages from reading `AWS_SECRET_ACCESS_KEY` or DB secrets.
* **Dynamic Payload ABAC (Issue #555)**:
  Blocks destructive SQL or unauthorized fund transfers even if the caller holds a developer role.
* **Inline PCI-DSS DLP**: Masks credit card PANs inline (`[REDACTED_CREDIT_CARD]`) before tool outputs reach LLM context windows.
* **Tamper-Evident SIEM Audit**: Consecutive SHA-256 hash chaining ensures cryptographic non-repudiation:
  ```bash
  aegis-gateway audit --output soc2-audit-report.json
  ```

---

### 📈 5. Enterprise Architect & FinOps (Runaway Loop Freezes)
*Goal: Prevent autonomous agent loops from draining corporate budgets.*

* **Multi-Tenant Budget Metering**: Partitioned tracking by `TenantId` and department.
* **Automated Hard Freeze**: Once threshold is breached, subsequent invocations are rejected immediately.
* **Distributed Circuit Breakers**: Tripped backends fast-fail in microseconds, preventing cascading pool exhaustion.

---

### 🧠 6. Autonomous Agent Engineers (Search, Planning & Zero-Mock Execution)
*Goal: Token-efficient tool discovery, multi-step DAG planning with cycle prevention, and genuine backend execution.*

* **Progressive Tool Discovery (`gateway_search_tools`)**: Eliminates LLM context bloat through multi-tier progressive disclosure:
  * **L0**: Capability summary (~50 tokens per tool).
  * **L1**: Signature and parameter keys.
  * **L2**: Full JSON Schema only on demand.
* **DAG Execution Planning (`gateway_plan_tasks`)**:
  * **Cycle Detection**: DFS recursion check flags circular dependencies prior to execution.
  * **Preflight ABAC**: Authorizes every step against tenant policies before any tool fires.
  * **Risk Tier Scoring**: Assigns Low/Medium/High/Critical risk tiers; mandates explicit human approval for destructive actions.
* **100% Genuine Wire Routing**: Dispatches tool calls asynchronously to registered child processes and HTTP servers with zero mock stubs.

---

### 🛡️ 7. Enterprise Data Governance & In-Situ Analytics (Zero-Egress Clean Room)
*Goal: Enforce GDPR / UU PDP compliance, prevent raw data exfiltration, and run high-speed in-situ analytics.*

* **Configurable Policy Tiers (`AEGIS_POLICY_TIER`)**:
  * `dev`: Permissive egress for friction-free developer experimentation.
  * `hybrid`: Small files (<= 10MB) & derived artifacts pass; raw datasets (> 10MB) mandate in-situ analytics or HITL approval.
  * `strict`: Zero-egress clean room; raw datasets are strictly confined to server memory.
* **Smart Taxonomy & In-Situ Analytics (`execute_in_situ_query`)**: DuckDB / Polars / Python engine runs queries inside server RAM, returning aggregated summary tables (< 1KB) rather than 100MB+ raw files (>99.9% egress reduction).
* **Preflight Egress Evaluation (`evaluate_data_egress`)**: Agents evaluate download rules and size thresholds before initiating transfers.

---

### 🤝 8. Human-in-the-Loop (HITL) Multi-Channel Approval & Zero-Lockin Resumption
*Goal: Provide durable task suspension and interactive managerial approval across Slack, Microsoft Teams, and Webhooks without vendor lock-in.*

* **Pluggable Dispatchers**: Interactive Slack Block Kit (Approve/Deny buttons), Microsoft Teams Adaptive Cards, Generic Webhook, and In-Band MCP prompts.
* **Durable Cryptographic Resume (`resolve_approval`)**: Validates HMAC-SHA256 signature tokens to resume suspended workflows with non-repudiation audit trails.
* **Zero-Knowledge Credential Proxy & Managed OAuth**: Infisical Universal Auth dynamic pulling, loopback transport proxy, and RFC 9728 PKCE authorization code lifecycle.

---

## 🧪 Graduated Testing Framework & Parity Suite

Aegis Gateway enforces real-world reliability through a progressive 6-Tier E2E matrix and legacy parity verification:

```bash
cargo test --test e2e_tier1_hobbyist_quickstart      # Tier 1: Zero-Config Golden Path
cargo test --test e2e_tier2_clumsy_dev_negative      # Tier 2: Boundary Misuse & Broken JSON
cargo test --test e2e_tier3_team_multiplexing        # Tier 3: Subprocess Orchestration & Anti-Zombie
cargo test --test e2e_tier4_enterprise_ha_production # Tier 4: K8s Probes & Graceful Drain
cargo test --test e2e_tier5_ciso_adversarial_security# Tier 5: OWASP LLM, DLP, & SIEM Hash Chains
cargo test --test e2e_tier6_sre_chaos_resilience     # Tier 6: Fault Injection & FinOps Hard Freeze
cargo test --test phase9_cli_daemon_test             # Tier 7: Production CLI Daemon & Server
cargo test --test phase9_legacy_parity_test          # Tier 8: Legacy Gaps (Search, Cache, Split, Headers)
cargo test --test phase11_search_planning_and_real_routing_test # Tier 9: Search, DAG Planning & Real Routing
cargo test --test phase12_egress_guardrails_refusal_integrity_and_outcome_attestation_test # Tier 10: Egress, Refusal & Attestation
cargo test --test phase13_stream_buffer_reuse_oauth_resource_metadata_and_process_reaper_test # Tier 11: Stream Reuse, OAuth & Reaping
cargo test --test phase14_session_teardown_fence_configurable_oauth_callback_host_typed_ssrf_refusal_and_large_catalog_deep_indexing_test # Tier 12: Session Fence, OAuth Host & Deep Indexing
cargo test --test phase15_runtime_dag_expression_interpolation_pipeline_and_hybrid_semantic_rrf_tool_retrieval_engine_test # Tier 13: DAG Interpolation & Hybrid RRF Search
cargo test --test phase16_universal_code_sandbox_credential_broker_and_egress_firewall_test # Tier 14: Hermetic Code Sandbox, Credential Broker & Egress Firewall
cargo test --test phase22_data_egress_and_in_situ_test # Tier 15: Policy Tiers & In-Situ Analytics
cargo test --test phase23_multi_channel_approval_test  # Tier 16: Multi-Channel HITL & Durable Resume
```

---

## Enterprise Roadmap

Full specs available in [`docs/roadmap/`](docs/roadmap/):

| Phase | Milestone | Standard / Focus | Status |
| :--- | :--- | :--- | :--- |
| **Phase 1** | Distributed Foundation | Redis Cluster, Rate Limiter, Circuit Breaker | `Completed` |
| **Phase 2** | Zero-Trust IAM & ABAC | OIDC / JWT, OPA Dynamic Payload Constraints | `Completed` |
| **Phase 3** | Real-Time DLP | Sub-ms Regex + Presidio PHI/PII Masking | `Completed` |
| **Phase 4** | Tamper-Evident SIEM | SHA-256 Hash Chained Audit, OpenTelemetry | `Completed` |
| **Phase 5** | FinOps & Multi-Tenancy | Tenant Quotas, Soft Alerts, Hard Freeze | `Completed` |
| **Phase 6** | Centralized Skill OS | GitOps Hot-Reload, Semantic Vector RAG | `Completed` |
| **Phase 7** | MCP Wire Transports | JSON-RPC 2.0, Clean Stdio, Streamable HTTP | `Completed` |
| **Phase 8** | Backend Multiplexing | Hermetic Sandboxing, Config Loader | `Completed` |
| **Phase 9** | Production CLI Daemon | CLI Subcommands (`serve`, `add`, `list`), Legacy Parity | `Completed` |
| **Phase 10**| Graduated E2E Testing | 6-Tier Persona Validation Matrix | `Completed` |
| **Phase 11**| Search, Planning & Zero-Mock | Progressive Discovery, Task Planning & Real Wire Routing | `Completed` |
| **Phase 12**| Egress, Refusal & Attestation | SSRF Guardrails, Refusal Audit Integrity & Outcome Attestation | `Completed` |
| **Phase 13**| Stream, OAuth & Reaper | Reusable Stream Buffer, OAuth Resource Metadata & Process Reaping | `Completed` |
| **Phase 14**| Session Fence, OAuth Host & Deep Indexing | In-Flight Session Teardown Guard, Configurable OAuth Host, Typed SSRF & Large Catalog Deep Indexing | `Completed` |
| **Phase 15**| DAG Interpolation & Hybrid RRF | Runtime DAG Expression Interpolation Pipeline & Hybrid Semantic RRF Tool Retrieval Engine | `Completed` |
| **Phase 16**| Universal Code Sandbox & Credential Broker | Hermetic Context-Agnostic Execution Sandbox, Zero-Knowledge Vault Credential Broker & Default-Deny Egress Firewall | `Completed` |
| **Phase 17**| Dynamic Vault & Hot Rotation | Infisical Universal Auth Secret Store, Zero-Plaintext on Disk & Webhook Rotation | `Completed` |
| **Phase 18**| Zero-Knowledge Egress Credential Proxy | Loopback Proxy Sidecar, Zero-Secret Process Space & Anti-Obfuscation Injection | `Completed` |
| **Phase 19**| HITL Approval Gate & Risk Interceptor | Declarative Action Risk Classification, Suspended Task State & Interactive Approval | `Completed` |
| **Phase 20**| User Identity Delegation & Blast Scoping | 3-Legged OAuth OBO Impersonation, Non-Repudiation Audit & Virtual Resource Isolation | `Completed` |
| **Phase 21**| Agnostic Managed OAuth Connect Engine | RFC 9728 Discovery, Vendor-Agnostic PKCE Router & Interactive Auth Lifecycle | `Completed` |
| **Phase 22**| Policy Tiers & In-Situ Analytics | Configurable Policy Profiles (Dev/Hybrid/Strict), Granular Data Egress & In-Situ Analytics Enclave | `Completed` |
| **Phase 23**| Multi-Channel Approval & Durable Resume | Pluggable Multi-Channel Approval Dispatcher (Slack/Teams/Webhook/MCP) & Durable Resume Router | `Completed` |

---

## Verification & Auditing

Run the comprehensive enterprise governance check and zero-mock audit:
```bash
bash scripts/audit_mock_detection.sh
bash scripts/governance-check.sh
```

Run all unit, integration, and E2E test suites:
```bash
cargo test
```

Check architectural invariants (SOLID & <= 350 lines per file):
```bash
find src tests -type f -name "*.rs" -exec wc -l {} + | sort -n
```

---

## License

This project is licensed under the permissive **[MIT License](LICENSE)**. Free for commercial enterprise adoption without source-available or non-commercial restrictions.
