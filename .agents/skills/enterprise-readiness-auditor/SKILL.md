---
name: enterprise-readiness-auditor
description: Automated verification and compliance auditor for Enterprise AI/MCP Gateway standards (Distributed State/HA, Zero-Trust IAM & ABAC, DLP/PII Masking, Tamper-Proof Audit, and MIT License).
---

# Enterprise Readiness Auditor Skill

This skill governs and verifies that any implementation of `aegis-gateway` strictly satisfies the 6 core pillars required by Enterprise CISOs, Cloud Architects, and Compliance Officers.

## The 6 Enterprise Pillars (Mandatory Gate)

1. **Pillar 1: Distributed State & Clustering (HA)**
   - No single-process bottlenecks or purely in-memory states in production paths.
   - Cache, Rate-Limiter, and Circuit-Breaker MUST implement the distributed trait contracts (`DistributedState`, `DistributedCache`, `DistributedRateLimiter`).
   - Pods in Kubernetes must be stateless and horizontally scalable behind Load Balancers.

2. **Pillar 2: Zero-Trust IAM & Granular ABAC**
   - No static shared secrets or unvetted bearer keys.
   - Granular Attribute-Based Access Control (ABAC) evaluating: caller subject, tenant ID, tool name, argument payload bounds, and time-of-day.
   - Pluggable validation for Enterprise IdP (OIDC/SAML claims).

3. **Pillar 3: Data Loss Prevention (DLP) & PII Masking**
   - Tool response payloads MUST pass through real-time DLP inspection before returning to LLM context.
   - Automated detection and redaction for Credit Cards (PCI-DSS), National IDs / SSN (GDPR/PII), and Medical Info (HIPAA).

4. **Pillar 4: Tamper-Evident SIEM Audit Streaming**
   - Every discovery, policy check, invocation, and error must emit structured audit events.
   - Pluggable streaming sinks (OpenTelemetry, Splunk, Elastic, Datadog) with SHA-256 integrity signatures.

5. **Pillar 5: Multi-Tenant FinOps & Hard Quotas**
   - Tenant-level and department-level cost accounting and token tracking.
   - Hard budget freezes when monthly quotas are exhausted.

6. **Pillar 6: Permissive License & IP Safety (MIT)**
   - Pure MIT license without non-commercial restrictions (PolyForm traps).
   - Zero GPL/AGPL copyleft contamination in cargo dependencies.

## Audit Workflow for Coding Agents

Whenever a coding agent modifies or adds features to `aegis-gateway`, the agent MUST:
1. Run the enterprise audit suite:
   ```bash
   bash scripts/governance-check.sh
   # or run the dedicated auditor binary:
   cargo run --bin aegis-audit
   ```
2. Verify that all 6 pillars pass with a score of 100%.
3. Any regression or shortcut (such as adding hardcoded in-memory state without trait abstraction) MUST fail the build.
