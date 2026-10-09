# Phase 12: Egress Guardrails, Refusal Audit Integrity & Outcome Attestation

> **Milestone Tag**: `v1.12.0-egress-guardrails-attestation`  
> **Status**: `Completed`  
> **Standards Reference**: OWASP Top 10 for LLMs (LLM08, LLM10), Model Context Protocol (MCP 2024-11-05), SOC 2 Type II Non-Repudiation, RFC 1918 Private Addressing

---

## 1. Problem Statement & Motivation
Comparative audit against live GitHub issues of legacy MCP gateways (`docker/mcp-gateway`, `microsoft/mcp-gateway`) revealed four critical operational gaps:
1. **Refusal Audit Integrity (docker/mcp-gateway#591)**: First-generation proxies logged refused tool calls as normal invocations or emitted no audit links, violating SOC 2 non-repudiation.
2. **SSRF & Source-Rights Egress Guardrails (docker/mcp-gateway#594 & microsoft/mcp-gateway#109)**: Unrestricted URL parameters enabled SSRF attacks against cloud metadata (`169.254.169.254`) and internal VPC subnets.
3. **Tool Outcome Attestation (microsoft/mcp-gateway#102 & docker/mcp-gateway#557)**: Lack of tamper-evident execution provenance for downstream CI/CD validation gates.
4. **Namespaced Prompt & Tool Collision (docker/mcp-gateway#581)**: Upstream servers with identical prompt/tool names (`code_review`, `query`) overwritten or broken on load.

---

## 2. Technical Architecture & Trait Contracts

### 2.1 Egress Policy Guardrail (`src/policy/egress.rs`)
- Trait `EgressPolicyGuard` inspecting URLs in requests.
- `DefaultEgressGuard` blocking:
  - AWS & Google Cloud metadata endpoints (`169.254.169.254`, `metadata.google.internal`).
  - RFC-1918 private IPv4 ranges (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `127.0.0.1`, `localhost`).
  - Dangerous protocols (`file://`, `gopher://`).
  - Wildcard domain allowlists (`*.corp.internal`, `api.github.com`).
  - Deep recursive argument inspection for nested URL attributes.

### 2.2 Refusal Audit Integrity (`src/lib.rs`)
- Guaranteed emission of explicit `AuditAction::PolicyEvaluated { allowed: false, reason }` events on:
  - Budget/quota exhaustion (`QuotaExceeded`).
  - Rate limit trips (`RateLimitExceeded`).
  - Circuit breaker trips (`CircuitOpen`).
  - ABAC / Egress denials (`PolicyDenied`).
- Cryptographically chained into SIEM audit sequence with zero fake "tool invoked" lines.

### 2.3 Tool Outcome Attestation Engine (`src/audit/attestation.rs`)
- Cryptographic provenance `ToolOutcomeAttestation` sealing:
  - `(tool, arguments_hash_sha256, result_hash_sha256, timestamp, duration_ms, gateway_identity)`.
  - Signature verification (`verify(&secret_key)`) detecting outcome/argument tampering.
  - Optional `attestation` embedding in `ToolCallResponse`.

### 2.4 Namespaced Collision Catalog (`src/discovery/namespace.rs`)
- `NamespacedCatalog` resolving tools and prompts via `{server}__{name}`.
- Unqualified aliases retained while unambiguous; automatically stripped upon multi-server collisions to prevent accidental misrouting.

---

## 3. Empirical Verification Plan
- Integration test suite: `tests/phase12_egress_guardrails_refusal_integrity_and_outcome_attestation_test.rs` (4/4 tests passing).
- Zero-mock compliance: `bash scripts/audit_mock_detection.sh` (100% PASS).
- Hard line constraint: `wc -l <= 350` across all files.
