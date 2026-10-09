---
name: mcp-enterprise-gap-auditor
description: Specialized enterprise gap audit skill targeting the 9 fatal enterprise flaws and complaints of first-generation MCP gateways.
---

# MCP Enterprise Gap Auditor Skill

This skill specifically targets, measures, and tracks remediation for the 9 fundamental enterprise complaints and failure modes identified in first-generation MCP proxies (e.g., `mcp-gateway`):

## The 9 Enterprise Complaints Targeted

1. **In-Memory State Lock-in & No Clustering**:
   - *Legacy Flaw*: Single-process memory (`Arc<Mutex<HashMap>>`), lost on pod restart, impossible to run across multi-pod Kubernetes clusters.
   - *Aegis Defense*: `DistributedState`, `DistributedCache`, `DistributedRateLimiter`, and `DistributedCircuitBreaker` abstractions with Redis Cluster backend (Phase 1).

2. **Coarse Role Bits / No Dynamic Payload ABAC (Issue #555)**:
   - *Legacy Flaw*: Any caller with access to a tool can execute destructive or unlimited actions without argument bounds checks.
   - *Aegis Defense*: Dynamic payload inspection (`eval_payload`), `PolicyContext`, and OPA/Cedar rule engine (Phase 2).

3. **Subprocess Silent Crashes & Cascading Failure (Issue #3034)**:
   - *Legacy Flaw*: Stdio backends crash silently; timeouts propagate without circuit breakers or health probes.
   - *Aegis Defense*: Distributed circuit breaker trips automatically; Kubernetes `/healthz` and `/readyz` probes.

4. **Zero Data Loss Prevention (PII / PCI / Secrets Exfiltration)**:
   - *Legacy Flaw*: Tool outputs containing credit card numbers, SSNs, or API keys are dumped directly into LLM context.
   - *Aegis Defense*: Bidirectional `DlpPipeline` (`sanitize_response` and `inspect_request_arguments`).

5. **Prompt Token Bloat & Meta-Tool 2-Hop Search Overhead**:
   - *Legacy Flaw*: Exposing 100+ tools blows prompt budget; 2-hop search adds 1 extra turn and 1.2%–16.1% token penalty.
   - *Aegis Defense*: Progressive Disclosure (L0 Purpose ~50 tok, L1 Signature, L2 Full Schema) without 2-hop latency penalty.

6. **Anti-Poisoning & System Prompt Injection (OWASP LLM01, LLM07)**:
   - *Legacy Flaw*: Malicious tool descriptions or prompt injection payloads can hijack the host agent.
   - *Aegis Defense*: `PoisonScanner` inspecting tool descriptions and skill markdown.

7. **No Tamper-Evident SIEM Audit Logs**:
   - *Legacy Flaw*: Raw stdout logging fails SOC 2 Type II and ISO 27001 non-repudiation requirements.
   - *Aegis Defense*: `AuditSink` with `payload_hash_sha256` and cryptographic hash chaining.

8. **No FinOps & Runaway Loop Cost Cutoffs**:
   - *Legacy Flaw*: Infinite agent loops can exhaust third-party API quotas and cloud budgets without caps.
   - *Aegis Defense*: `QuotaEngine` with tenant budget envelopes and hard execution freeze.

9. **Skill Dispersal & Static Prompt Stuffing**:
   - *Legacy Flaw*: Prompts pasted into configs without versioning or on-demand loading.
   - *Aegis Defense*: Centralized `SkillRegistry` with Tier 1 Metadata and Tier 2 on-demand bundle loading.

## Execution

Run the MCP enterprise gap evaluation:
```bash
bash .agents/skills/mcp-enterprise-gap-auditor/scripts/audit_mcp_gaps.sh
```

To run in strict production-readiness mode (will fail until all Phases 1-6 drivers are implemented):
```bash
STRICT_ENTERPRISE=1 bash .agents/skills/mcp-enterprise-gap-auditor/scripts/audit_mcp_gaps.sh
```
