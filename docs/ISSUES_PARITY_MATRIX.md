# Aegis Gateway: Enterprise Issue Parity & Community Resolution Ledger

> **Standard Compliance**: ISO 27001, SOC 2 Type II Non-Repudiation, NIST SP 800-218 (SSDF), OWASP Agentic AI Top 10  
> **Target Benchmarks**: `MikkoParkkola/mcp-gateway` (canonical Rust proxy), `docker/mcp-gateway`, `microsoft/mcp-gateway`  
> **Policy**: Zero-Mock, Interface-First, SOLID Architecture, Hard Constraint `wc -l <= 350`.

---

## 1. Governance & Issue Tracking Standard

In high-assurance enterprise systems, issue resolution cannot remain ephemeral in commit messages. Aegis Gateway enforces a three-tier tracking structure:
1. **Master Issue Parity Matrix (`docs/ISSUES_PARITY_MATRIX.md`)**: The central permanent index linking every upstream bug, community complaint, and CVE to concrete code and empirical test traces.
2. **Phase Specifications (`docs/roadmap/NN_PHASE_*.md`)**: Dedicated architectural milestone contracts containing problem statements, threat models, and interface definitions.
3. **Automated Audit Verifiers (`scripts/audit_gap_coverage.py` & `scripts/governance-check.sh`)**: Static and dynamic verification validating that no regression occurs.

---

## 2. Comprehensive Issue Resolution Ledger (Verified in Live Code)

| Repository | Issue | Category | Upstream Vulnerability / Deficit | Aegis Gateway Resolution | Source File | Empirical Test Trace |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `MikkoParkkola/mcp-gateway` | [#3265](https://github.com/MikkoParkkola/mcp-gateway/issues/3265) | `SEC` | In-memory state lock-in; pod crashes lose session states | Pluggable `DistributedState` with Redis Cluster Lua-scripts & in-memory dev backend | `src/core/state.rs` | `tests/phase1_distributed_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#555](https://github.com/MikkoParkkola/mcp-gateway/issues/555) | `SEC` | Coarse role bits; no dynamic payload argument bounds | ABAC engine with JSON AST evaluation, OPA/Rego constraints & DDL blocking | `src/policy/mod.rs` | `tests/phase2_zero_trust_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#3034](https://github.com/MikkoParkkola/mcp-gateway/issues/3034) | `OPS` | Subprocess silent crash cascades timeout across gateway | Distributed Circuit Breakers (`Closed/HalfOpen/Open`) & K8s probes | `src/state/mod.rs` | `tests/phase8_backend_multiplexing_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#2578](https://github.com/MikkoParkkola/mcp-gateway/issues/2578) | `SEC` | Zero Data Loss Prevention (DLP); PII/PCI leaked to LLMs | Bidirectional `DlpPipeline` with sub-ms SIMD Luhn mask & HIPAA PHI encrypt | `src/dlp/mod.rs` | `tests/phase3_dlp_guardrails_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#2568](https://github.com/MikkoParkkola/mcp-gateway/issues/2568) | `OPS` | Meta-tool 2-hop search token bloat (16% token waste) | Progressive Disclosure: L0 Purpose (~50 tokens), L1 Signature, L2 Schema | `src/discovery/mod.rs` | `tests/phase6_skill_os_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#2546](https://github.com/MikkoParkkola/mcp-gateway/issues/2546) | `SEC` | Zero audit non-repudiation; plain stdout logs altered | Tamper-evident SHA-256 Hash Chaining with SOC 2 Type II report generator | `src/audit/mod.rs` | `tests/phase4_siem_audit_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#2543](https://github.com/MikkoParkkola/mcp-gateway/issues/2543) | `OPS` | Runaway autonomous agent loops drain token budgets | FinOps `QuotaEngine` with automatic hard freeze cutoff & chargeback | `src/state/quota.rs` | `tests/phase5_finops_quota_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#2532](https://github.com/MikkoParkkola/mcp-gateway/issues/2532) | `OPS` | Runtime reconfiguration requires full process restarts | Zero-Downtime Dynamic Control Plane via atomic in-memory swapping | `src/control/mod.rs` | `tests/phase27_dynamic_control_plane_and_hot_reload_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#2531](https://github.com/MikkoParkkola/mcp-gateway/issues/2531) | `PROT`| Stdio line framing corruption on multi-line JSON | Hermetic Stdio Transport with byte-delimited framing buffer | `src/transport/stdio.rs` | `tests/phase7_mcp_wire_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#2526](https://github.com/MikkoParkkola/mcp-gateway/issues/2526) | `OPS` | Single-pod deployment unable to scale horizontally | Distributed Cluster Sync Engine via Redis pub/sub state bus | `src/cluster/mod.rs` | `tests/phase28_cluster_sync_engine_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#2530](https://github.com/MikkoParkkola/mcp-gateway/issues/2530) | `OPS` | Stdio EOF teardown awaits have no time limit (hang) | 2-Stage Graceful Teardown: SIGTERM with 1500ms timeout & buffer drain | `src/backend/subprocess.rs` | `tests/phase36_resilient_subprocess_teardown_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#2573](https://github.com/MikkoParkkola/mcp-gateway/issues/2573) | `OPS` | Killed child processes corrupt profiler and logs | Synchronous standard output drain before child process termination | `src/backend/subprocess.rs` | `tests/phase36_resilient_subprocess_teardown_test.rs` |
| `MikkoParkkola/mcp-gateway` | [#2567](https://github.com/MikkoParkkola/mcp-gateway/issues/2567) | `OPS` | Busy session reaped prematurely at static age limit | Active task reference counter leasing; locks session during streaming | `src/state/session_fence.rs` | `tests/phase36_resilient_subprocess_teardown_test.rs` |
| `docker/mcp-gateway` | [#597](https://github.com/docker/mcp-gateway/issues/597) | `OPS` | Chaos resilience failure under sudden network drops | Automated Half-Open health recovery probes & exponential backoff | `src/state/mod.rs` | `tests/e2e_tier6_sre_chaos_resilience.rs` |
| `docker/mcp-gateway` | [#585](https://github.com/docker/mcp-gateway/issues/585) | `SEC` | Unverified AI agent identity triggers arbitrary tools | Ed25519 AgentCert Identity & Trust Verification Gate | `src/trust/mod.rs` | `tests/phase26_agent_cert_and_threat_evidence_test.rs` |
| `docker/mcp-gateway` | [#569](https://github.com/docker/mcp-gateway/issues/569) | `SEC` | Tool output cannot be verified as authentic | Tool Outcome Attestation (TOA) cryptographic HMAC proof | `src/audit/attestation.rs` | `tests/phase12_egress_guardrails_refusal_integrity_and_outcome_attestation_test.rs` |
| `microsoft/mcp-gateway` | [#109](https://github.com/microsoft/mcp-gateway/issues/109) | `SEC` | Untrusted external URLs executed without preflight | Autonomous Threat Evidence enrichment (VirusTotal/DNSBL preflight) | `src/trust/mod.rs` | `tests/phase26_agent_cert_and_threat_evidence_test.rs` |
| `microsoft/mcp-gateway` | [#98](https://github.com/microsoft/mcp-gateway/issues/98) | `PROT`| Legacy protocol incompatibility with MCP 2026-07-28 | Dual-stack Stateless MCP 2026-07-28 with stateless header injection | `src/transport/mod.rs` | `tests/phase29_modern_stateless_mcp_test.rs` |

---

## 3. Scheduled & Roadmapped Issues (Upcoming Milestones)

| Repository | Issue | Category | Upstream Vulnerability / Deficit | Scheduled Aegis Resolution | Target Milestone | Roadmap Specification |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `MikkoParkkola/mcp-gateway` | [#2555](https://github.com/MikkoParkkola/mcp-gateway/issues/2555) | `SEC` | Replayed chain-refused call loses original caller context | Immutable cryptographic correlation header retained across retries | Phase 37 | `docs/roadmap/37_PHASE_37_CRYPTOGRAPHIC_NONCE_REPLAY_AND_TLS_PINNING.md` |
| `MikkoParkkola/mcp-gateway` | [#2554](https://github.com/MikkoParkkola/mcp-gateway/issues/2554) | `SEC` | Re-paired or restarted HTTP backends lose TLS pinning | Persistent SPKI SHA-256 Subject Public Key Info certificate pinning | Phase 37 | `docs/roadmap/37_PHASE_37_CRYPTOGRAPHIC_NONCE_REPLAY_AND_TLS_PINNING.md` |
| `MikkoParkkola/mcp-gateway` | [#2547](https://github.com/MikkoParkkola/mcp-gateway/issues/2547) | `SEC` | Approval task gate evaluated without nonce check | Monotonic Nonce Admission Gate verifying single-use authorization | Phase 37 | `docs/roadmap/37_PHASE_37_CRYPTOGRAPHIC_NONCE_REPLAY_AND_TLS_PINNING.md` |
| `docker/mcp-gateway` | [#558](https://github.com/docker/mcp-gateway/issues/558) | `OPS` | Slow scraping/batch tools starve sub-ms tools (HOL) | Head-of-Line (HOL) Guard with Weighted Fair Queuing (WFQ) priority lanes | Phase 38 | `docs/roadmap/38_PHASE_38_HOL_GUARD_AND_ADAPTIVE_ROUTING.md` |
| `microsoft/mcp-gateway` | [#105](https://github.com/microsoft/mcp-gateway/issues/105) | `OPS` | Silent schema changes and latency degradation drift | Real-time Adaptive Schema & Performance Drift Sentinel | Phase 38 | `docs/roadmap/38_PHASE_38_HOL_GUARD_AND_ADAPTIVE_ROUTING.md` |

---

## 4. Continuous Verification & Compliance Guarantee

Every issue listed in this matrix is accompanied by:
1. **Zero Mock Invariant**: Verified without dummy stubs or fake closures in `src/`.
2. **Automated Static Audit**: Monitored by `.agents/skills/mcp-gap-resolver/scripts/audit_gap_coverage.py`.
3. **Continuous Regression Testing**: Evaluated on every git push via `cargo-nextest run`.
