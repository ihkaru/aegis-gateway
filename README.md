# Aegis Gateway (🛡️)

> **Enterprise Zero-Trust MCP & Skill Gateway with Distributed State, Granular ABAC, Real-Time DLP, and Tamper-Proof Audit.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust: 2024](https://img.shields.io/badge/Rust-2024%20Edition-blue.svg)](https://www.rust-lang.org)
[![Security: Zero Unsafe](https://img.shields.io/badge/Unsafe-Denied%20(%23![deny(unsafe_code)])-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Tests: 52 Passed](https://img.shields.io/badge/Tests-52%2F52%20Passed%20(100%25)-brightgreen.svg)](#verification)
[![Architecture: SOLID](https://img.shields.io/badge/Architecture-SOLID%20%26%20DI-green.svg)](#architecture)

---

## Why Aegis Gateway?

Conventional MCP proxies operate as fragile single-process daemons with in-memory state lock-in, coarse permissions, zero data-loss prevention, and zombie process leaks.

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
    
    Core --> Tools["Hermetic Backend Upstreams<br/>(env_clear, typed argv, no zombies)"]
    Core --> Skills["Centralized Skill Registry<br/>(Progressive L0/L1/L2 Disclosure)"]
```

---

## Choose Your Journey (DX-Centric Personas)

Select your developer profile below for tailored quickstarts and real-world configurations:

### ⚡ 1. The Dev Hobbyist (Zero-Config Quickstart < 60s)
*Goal: Connect to Claude Desktop or Cursor immediately with zero external databases.*

1. **Clone and Build**:
   ```bash
   git clone https://github.com/ihkaru/aegis-gateway.git
   cd aegis-gateway && cargo build --release
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
3. *Why it just works*: Default in-memory drivers start instantly. Stdio stdout is strictly framed for JSON-RPC 2.0 with zero log pollution (diagnostics routed to stderr).

---

### 🛠️ 2. The Fullstack Team Lead (Local Multiplexing)
*Goal: Unify multiple micro-servers (Postgres, Filesystem, Git) under one clean topology.*

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
cargo run -- --config aegis.yaml --stdio
```
*Guarantees*: Subprocesses run in hermetic process groups with `kill_on_drop(true)`—never leaving orphaned zombie processes on your machine.

---

### ☸️ 3. The DevOps & Cloud SRE (Kubernetes HA & Production)
*Goal: Multi-pod deployment behind AWS ALB / Ingress Controller with zero-downtime rolling updates.*

* **Streamable HTTP (RFC 2025-03-26)**: Single endpoint `POST /mcp` eliminates dual-endpoint SSE session stickiness failures on Kubernetes load balancers.
* **Kubernetes Health Probes**:
  * Liveness: `GET /healthz` -> `{"status":"healthy"}`
  * Readiness: `GET /readyz` -> Distributed state checks.
* **Graceful Drain Coordinator**:
  ```rust
  // Holds SIGTERM until active inflight tool requests finish gracefully
  gateway.drain_coordinator().wait_drain(Duration::from_secs(30)).await?;
  ```

---

### 🔒 4. The CISO & Security Red Team (Zero-Trust & Compliance)
*Goal: Enforce OWASP Top 10 for LLMs, PCI-DSS / HIPAA DLP, and SOC 2 Type II audit integrity.*

* **OWASP LLM08 Hermetic Subprocess Isolation**: Child processes launch with `cmd.env_clear()`, strictly preventing untrusted npm/pip packages from reading `AWS_SECRET_ACCESS_KEY` or DB secrets.
* **Dynamic Payload ABAC (Issue #555)**:
  ```rust
  // Blocks destructive SQL even if user has developer role
  opa.eval_payload("sql_execute", &json!({ "query": "DROP TABLE users;" })); // -> Deny
  ```
* **Inline PCI-DSS DLP**: Masks credit card PANs inline (`[REDACTED_CREDIT_CARD]`) before tool outputs reach LLM context windows.
* **Tamper-Evident SIEM Audit**: Consecutive SHA-256 hash chaining ensures cryptographic non-repudiation for SOC 2 Type II / ISO 27001.

---

### 📈 5. Enterprise Architect & FinOps (Runaway Loop Freezes)
*Goal: Prevent autonomous agent loops from draining corporate budgets.*

* **Multi-Tenant Budget Metering**: Partitioned tracking by `TenantId` and department.
* **Automated Hard Freeze**:
  ```rust
  // Once threshold is breached, subsequent invocations are rejected immediately
  quota.check_budget(&tenant).await?; // -> false (Frozen)
  ```
* **Circuit Breakers**: Tripped backends fast-fail in microseconds, preventing cascading pool exhaustion.

---

## Graduated 6-Tier E2E Testing Suite

Aegis Gateway enforces real-world reliability through a progressive 6-Tier E2E matrix:

```bash
cargo test --test e2e_tier1_hobbyist_quickstart      # Tier 1: Zero-Config Golden Path
cargo test --test e2e_tier2_clumsy_dev_negative      # Tier 2: Boundary Misuse & Broken JSON
cargo test --test e2e_tier3_team_multiplexing        # Tier 3: Subprocess Orchestration & Anti-Zombie
cargo test --test e2e_tier4_enterprise_ha_production # Tier 4: K8s Probes & Graceful Drain
cargo test --test e2e_tier5_ciso_adversarial_security# Tier 5: OWASP LLM, DLP, & SIEM Hash Chains
cargo test --test e2e_tier6_sre_chaos_resilience     # Tier 6: Fault Injection & FinOps Hard Freeze
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
| **E2E Suite**| Graduated Testing | 6-Tier Persona Validation Matrix | `Completed` |

---

## Verification & Auditing

Run the comprehensive enterprise governance check:
```bash
bash scripts/governance-check.sh
```

Run all 52 unit, integration, and E2E test cases:
```bash
cargo test
```

Check architectural invariants (SOLID & <= 350 lines per file):
```bash
find src tests scripts -type f -name "*.rs" -exec wc -l {} + | sort -n
```

---

## License

This project is licensed under the permissive **[MIT License](LICENSE)**. Free for commercial enterprise adoption without source-available or non-commercial restrictions.
