# MCP Gateway Operational Gap & Vulnerability Taxonomy

This reference guide categorizes common functional gaps, edge-case failures, and architectural complaints identified in first-generation MCP gateways (e.g., `docker/mcp-gateway`, `microsoft/mcp-gateway`) to guide automated remediation in Aegis Gateway.

---

## 1. Protocol Conformance & Ingress Transport (PROT)

| Code | Subcategory | Real-World Failure Mode | Enterprise Remediation Standard |
| :--- | :--- | :--- | :--- |
| `PROT-01` | **Framing & Taxonomy** | Custom JSON-RPC error codes break standard MCP clients (`-32700..-32603`). | Adhere strictly to MCP 2024-11-05 and JSON-RPC 2.0 specifications. |
| `PROT-02` | **Streamable HTTP** | Legacy proxies rely on split SSE endpoints (`GET /sse` + `POST /message`), failing behind Kubernetes L4/L7 ALBs. | RFC 2025-03-26 single-endpoint `POST /mcp` streaming. |
| `PROT-03` | **Header Negotiation** | Refusing or ignoring `mcp-protocol-version` request headers causes silent protocol mismatch (Issue #540). | Header inspection rejecting unserved revisions with HTTP 400. |
| `PROT-04` | **Clean Stdio Channel** | Subprocesses writing debug logs to `stdout` break JSON-RPC parsers in Claude Desktop and Cursor. | Hermetic Stdio transport isolating logging to `stderr`. |

---

## 2. Subprocess & Upstream Multiplexing (PROC)

| Code | Subcategory | Real-World Failure Mode | Enterprise Remediation Standard |
| :--- | :--- | :--- | :--- |
| `PROC-01` | **Zombie Subprocesses** | Node.js or Python child processes left running indefinitely after client disconnect (Issue #593). | Subprocess spawner with `kill_on_drop(true)` and PID tree tracking. |
| `PROC-02` | **Env Secret Leakage** | Child processes inherit parent environment variables (`AWS_SECRET_ACCESS_KEY`, `DATABASE_URL`) (OWASP LLM08). | `cmd.env_clear()` with explicit, whitelisted env injection. |
| `PROC-03` | **Package Cache Contention** | Concurrent `npx` or `uv` spawns corrupt shared global caches (Issue #622). | Per-backend isolated cache directory (`$TMPDIR/aegis-cache/{server}/`). |
| `PROC-04` | **Command Line Splitting** | Windows backslash paths torn apart by Unix POSIX shell argument parsers (Issue #523). | OS-aware splitting via `CommandLineToArgvW` on Windows vs POSIX on Unix. |

---

## 3. Zero-Trust Security & Dynamic ABAC (SEC)

| Code | Subcategory | Real-World Failure Mode | Enterprise Remediation Standard |
| :--- | :--- | :--- | :--- |
| `SEC-01` | **Payload Bound Bypass** | Caller with tool permission executes arbitrary SQL (`DROP TABLE`) or large wire transfers (Issue #555). | Dynamic argument evaluation (`eval_payload`) with OPA/Cedar constraint engines. |
| `SEC-02` | **Refusal Logging Discrepancy**| Refused tool calls logged as successful or lacking rejection cause links (Issue #591). | Explicit refusal audit events (`AuditAction::Blocked`) with rejection rule metadata. |
| `SEC-03` | **Tool Description Poisoning** | Malicious upstream servers inject system prompts via tool descriptions (OWASP LLM01). | Pre-registration `PoisonScanner` regex and semantic sanitization. |
| `SEC-04` | **Inline Data Exfiltration** | Sensitive PII/PCI/PHI returned in tool output is exposed directly to LLMs. | Bidirectional `DlpPipeline` with Luhn card verification and PHI tokenization. |

---

## 4. Operational High Availability & Resilience (OPS)

| Code | Subcategory | Real-World Failure Mode | Enterprise Remediation Standard |
| :--- | :--- | :--- | :--- |
| `OPS-01` | **In-Memory State Lock-in** | Gateway crashes lose sessions; unable to run multi-pod in Kubernetes. | Pluggable distributed state (`DistributedState`, Redis Cluster). |
| `OPS-02` | **Cascading Subprocess Hang** | Stdio backend freeze hangs entire client session indefinitely (Issue #3034). | Distributed Circuit Breakers and configurable invocation timeouts. |
| `OPS-03` | **Runaway Agent Billing** | Autonomous recursive loops exhaust LLM/Tool budgets uncontrollably. | `QuotaEngine` with tenant envelopes and automated hard freeze cutoff. |
| `OPS-04` | **Abrupt Termination Loss** | Rolling updates drop inflight requests without draining. | `DrainCoordinator` holding SIGTERM until active tasks complete gracefully. |

---

## 5. Tool Discovery, Token Optimization & Planning (DISC)

| Code | Subcategory | Real-World Failure Mode | Enterprise Remediation Standard |
| :--- | :--- | :--- | :--- |
| `DISC-01` | **Schema Token Bloat** | Registering 100+ tools consumes 30k+ prompt tokens per invocation. | Progressive Disclosure (L0 Purpose, L1 Signature, L2 Full Schema). |
| `DISC-02` | **Server Name Blindness** | User asking for "postgres tool" gets empty match if tool description omits server (Issue #2317). | Multi-attribute search indexing server names, descriptions, and tags. |
| `DISC-03` | **Autonomous Plan Cycles** | Multi-tool autonomous agents get trapped in circular DAG execution chains. | `ExecutionPlanner` DFS cycle detection and preflight ABAC validation. |
