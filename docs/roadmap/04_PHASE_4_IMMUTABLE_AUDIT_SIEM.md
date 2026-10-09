# Phase 4: Tamper-Evident SIEM Audit & Compliance

> **Milestone Tag**: `v0.5.0-immutable-audit`  
> **Status**: `In Progress` (Structured JSON sink verified; Hash chaining & OTel streaming planned)

---

## 1. Objectives

Provide non-repudiation, immutable cryptographic audit trails, and unified enterprise observability for all agent interactions, policy decisions, and tool executions to satisfy SOC 2 Type II, ISO 27001, and FedRAMP compliance standards.

---

## 2. Architecture & Contracts

Defined in [`src/core/audit.rs`](../../src/core/audit.rs):

```rust
#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn emit(&self, event: &AuditEvent) -> AegisResult<()>;
}
```

### Audit Invariants

1. **Non-Repudiation**:
   - Every event records the authenticated caller ID, tenant ID, target MCP server, tool name, and UTC timestamp.
   - Request and response payloads are hashed using SHA-256 to prove authenticity without storing raw sensitive data.
2. **Cryptographic Log Chaining**:
   - Sequential audit events form a cryptographic hash chain:
     $$\text{Hash}_n = \text{SHA256}(\text{Hash}_{n-1} \,\|\, \text{Event}_n)$$
   - Prevents log deletion, truncation, or historical retroactive tampering.
3. **Zero-Leakage SIEM Streaming**:
   - When DLP redacts sensitive tokens, the audit log registers `AuditAction::DlpRedacted` with the category and hash, but NEVER transmits raw unmasked secrets to external SIEM collectors.

---

## 3. Milestones & Checklist

- [x] **4.1 Core Audit Contract & Action Models**: Implement `AuditSink`, `AuditEvent`, and comprehensive `AuditAction` variants (Empirically verified in `tests/enterprise_governance_test.rs`).
- [x] **4.2 Structured JSON Sink**: Provide buffered structured event emission with thread safety and zero-allocation metadata serialization.
- [ ] **4.3 Tamper-Evident Hash-Chain Sequencer**: Implement verifiable chained event recording with periodic signed checkpoints.
- [ ] **4.4 OpenTelemetry (OTel) Distributed Tracing**: Export W3C TraceContext headers (`traceparent`) linking LLM orchestration prompts to downstream backend tool spans via gRPC OTLP.
- [ ] **4.5 Native Enterprise SIEM Adapters**:
  - Splunk HEC (HTTP Event Collector) sink with batched delivery.
  - Datadog Logs API adapter with automatic service and env tagging.
  - AWS CloudWatch Logs / Amazon S3 Glacier archive sink.
- [ ] **4.6 SOC 2 Audit Report Scaffolder**: CLI command `aegis-audit export-compliance-pack` generating cryptographic proof and summary tables for external compliance auditors.
